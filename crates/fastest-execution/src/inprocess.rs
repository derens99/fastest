//! PyO3 in-process test executor.
//!
//! Runs Python tests directly inside the current process using the embedded
//! Python interpreter via PyO3. This is fast for small test suites (<=20 tests)
//! because it avoids the overhead of spawning subprocesses.

use std::ffi::CString;

use fastest_core::fixtures::builtin::generate_builtin_code;
use fastest_core::markers::{classify_marker, BuiltinMarker};
use fastest_core::model::{TestItem, TestOutcome, TestResult};
use pyo3::ffi::c_str;
use pyo3::prelude::*;

use crate::capture::CapturedOutput;
use crate::timeout::{TestTimer, TimeoutConfig};

/// Executes Python tests in-process using PyO3.
pub struct InProcessExecutor {
    timeout_config: TimeoutConfig,
}

impl InProcessExecutor {
    /// Create a new in-process executor with default timeout.
    pub fn new() -> Self {
        Self {
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Create a new in-process executor with a custom timeout.
    pub fn with_timeout(timeout_config: TimeoutConfig) -> Self {
        Self { timeout_config }
    }

    /// Execute a batch of tests in-process and return results.
    pub fn execute(&self, tests: &[TestItem]) -> Vec<TestResult> {
        tests.iter().map(|test| self.run_single(test)).collect()
    }

    /// Run a single test item using the embedded Python interpreter.
    fn run_single(&self, test: &TestItem) -> TestResult {
        // Check for skip/skipif markers BEFORE execution
        for marker in &test.markers {
            match classify_marker(marker) {
                BuiltinMarker::Skip { reason } => {
                    return TestResult {
                        test_id: test.id.clone(),
                        outcome: TestOutcome::Skipped { reason },
                        duration: std::time::Duration::ZERO,
                        output: String::new(),
                        error: None,
                        stdout: String::new(),
                        stderr: String::new(),
                    };
                }
                BuiltinMarker::Skipif { condition, reason } => {
                    let should_skip = Python::with_gil(|py| {
                        let _ = py.run(c_str!("import sys, os, platform"), None, None);
                        match CString::new(condition.as_str()) {
                            Ok(cond_cstr) => match py.eval(&cond_cstr, None, None) {
                                Ok(result) => result.is_truthy().unwrap_or(false),
                                Err(_) => false,
                            },
                            Err(_) => false,
                        }
                    });
                    if should_skip {
                        return TestResult {
                            test_id: test.id.clone(),
                            outcome: TestOutcome::Skipped { reason },
                            duration: std::time::Duration::ZERO,
                            output: String::new(),
                            error: None,
                            stdout: String::new(),
                            stderr: String::new(),
                        };
                    }
                }
                _ => {}
            }
        }

        let timer = TestTimer::start(self.timeout_config.clone());

        let code = build_test_code(test);

        let (outcome, captured, error) = Python::with_gil(|py| {
            // Set up stdout/stderr capture in Python
            let setup_capture = c_str!(
                "import sys, io\n\
                 __fastest_stdout = io.StringIO()\n\
                 __fastest_stderr = io.StringIO()\n\
                 __fastest_old_stdout = sys.stdout\n\
                 __fastest_old_stderr = sys.stderr\n\
                 sys.stdout = __fastest_stdout\n\
                 sys.stderr = __fastest_stderr"
            );
            let restore_capture = c_str!(
                "sys.stdout = __fastest_old_stdout\n\
                 sys.stderr = __fastest_old_stderr\n\
                 __fastest_captured_stdout = __fastest_stdout.getvalue()\n\
                 __fastest_captured_stderr = __fastest_stderr.getvalue()"
            );

            // Run setup
            if let Err(e) = py.run(setup_capture, None, None) {
                return (
                    TestOutcome::Error {
                        message: format!("Failed to set up capture: {e}"),
                    },
                    CapturedOutput::new(),
                    Some(format!("{e}")),
                );
            }

            // Build CString for the dynamic test code
            let code_cstring = match CString::new(code.as_bytes()) {
                Ok(cs) => cs,
                Err(e) => {
                    return (
                        TestOutcome::Error {
                            message: format!("Test code contains null byte: {e}"),
                        },
                        CapturedOutput::new(),
                        Some(format!("{e}")),
                    );
                }
            };

            // Run the test
            let test_result = py.run(&code_cstring, None, None);

            // Restore stdout/stderr and collect captured output
            let _ = py.run(restore_capture, None, None);

            let captured = extract_captured_output(py);

            match test_result {
                Ok(()) => {
                    if timer.is_expired() {
                        (
                            TestOutcome::Error {
                                message: format!(
                                    "Test exceeded timeout of {:?}",
                                    timer.timeout_duration()
                                ),
                            },
                            captured,
                            None,
                        )
                    } else {
                        (TestOutcome::Passed, captured, None)
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    // Distinguish import/collection errors from test assertion failures
                    let is_collection_error = error_msg.contains("ModuleNotFoundError")
                        || error_msg.contains("ImportError")
                        || error_msg.contains("SyntaxError")
                        || error_msg.contains("AttributeError: module");
                    if error_msg.contains("SKIPPED:") {
                        let reason = error_msg
                            .split("SKIPPED:")
                            .nth(1)
                            .map(|s| s.trim().to_string());
                        (TestOutcome::Skipped { reason }, captured, None)
                    } else if is_collection_error {
                        (
                            TestOutcome::Error {
                                message: error_msg.clone(),
                            },
                            captured,
                            Some(error_msg),
                        )
                    } else {
                        (TestOutcome::Failed, captured, Some(error_msg))
                    }
                }
            }
        });

        let duration = timer.elapsed();

        let mut result = TestResult {
            test_id: test.id.clone(),
            outcome,
            duration,
            output: String::new(),
            error,
            stdout: captured.stdout,
            stderr: captured.stderr,
        };

        // After execution, check for xfail markers and transform outcome
        let xfail_info = test.markers.iter().find_map(|m| {
            if let BuiltinMarker::Xfail { reason, strict } = classify_marker(m) {
                Some((reason, strict))
            } else {
                None
            }
        });

        if let Some((reason, strict)) = xfail_info {
            result.outcome = match result.outcome {
                TestOutcome::Failed => TestOutcome::XFailed { reason },
                TestOutcome::Passed => {
                    if strict {
                        // strict xfail: passing is a failure
                        result.error = Some("test unexpectedly passed (strict xfail)".into());
                        TestOutcome::Failed
                    } else {
                        TestOutcome::XPassed
                    }
                }
                other => other,
            };
            // XFailed should not have error field (it's expected)
            if matches!(result.outcome, TestOutcome::XFailed { .. }) {
                result.error = None;
            }
        }

        result
    }
}

impl Default for InProcessExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract captured stdout/stderr from the Python interpreter's globals.
fn extract_captured_output(py: Python<'_>) -> CapturedOutput {
    let stdout = py
        .eval(c_str!("__fastest_captured_stdout"), None, None)
        .and_then(|v| v.extract::<String>())
        .unwrap_or_default();
    let stderr = py
        .eval(c_str!("__fastest_captured_stderr"), None, None)
        .and_then(|v| v.extract::<String>())
        .unwrap_or_default();
    CapturedOutput::from_strings(stdout, stderr)
}

/// Convert a JSON value to a Python literal expression.
fn json_value_to_python(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            format!("'{}'", escape_for_python_string(s))
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(json_value_to_python).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    format!(
                        "'{}': {}",
                        escape_for_python_string(k),
                        json_value_to_python(v)
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

/// Escape a string for safe embedding in a Python single-quoted string literal.
fn escape_for_python_string(s: &str) -> String {
    s.replace('\\', "\\\\") // must be first to avoid double-escaping
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Check that a string is a valid Python identifier (safe for code interpolation).
fn is_valid_python_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Build the Python code string needed to execute a test item.
///
/// The generated code:
/// 1. Adds the test file's parent directory to sys.path
/// 2. Imports the test module
/// 3. For class-based tests: instantiates the class, calls the method
/// 4. For function tests: calls the function directly
pub(crate) fn build_test_code(test: &TestItem) -> String {
    // Escape characters that could break out of Python string literals
    let path_str = escape_for_python_string(&test.path.to_string_lossy().replace('\\', "/"));
    let parent_dir = test
        .path
        .parent()
        .map(|p| escape_for_python_string(&p.to_string_lossy().replace('\\', "/")))
        .unwrap_or_default();

    let module_name = test
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    // Validate module_name is a valid Python identifier to prevent code injection
    let module_name = if module_name.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !module_name.is_empty()
        && !module_name.starts_with(|c: char| c.is_ascii_digit())
    {
        module_name
    } else {
        // Sanitize: keep only valid identifier characters
        let sanitized: String = module_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if sanitized.is_empty() {
            "unknown".to_string()
        } else {
            sanitized
        }
    };

    // Validate function name is a safe Python identifier
    let func_name = if is_valid_python_identifier(&test.function_name) {
        test.function_name.clone()
    } else {
        // Sanitize: keep only valid identifier characters
        test.function_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };

    // Determine which fixture_deps are actual fixtures (not parametrize params)
    let param_names: std::collections::HashSet<&str> = test
        .parameters
        .as_ref()
        .map(|p| p.names.iter().map(|n| n.as_str()).collect())
        .unwrap_or_default();

    // Build fixture setup code and kwargs
    let mut fixture_setup_parts: Vec<String> = Vec::new();
    let mut fixture_cleanup_parts: Vec<String> = Vec::new();
    let mut fixture_kwarg_names: Vec<String> = Vec::new();
    let mut conftest_fixture_names: Vec<String> = Vec::new();

    for dep in &test.fixture_deps {
        if dep == "self" || param_names.contains(dep.as_str()) {
            continue;
        }
        if let Some(code) = generate_builtin_code(dep) {
            fixture_setup_parts.push(code);
            fixture_kwarg_names.push(dep.clone());
            if dep == "monkeypatch" {
                fixture_cleanup_parts.push("monkeypatch.undo()".to_string());
            } else if dep == "capsys" {
                fixture_cleanup_parts.push("capsys._restore()".to_string());
            } else if dep == "capfd" {
                fixture_cleanup_parts.push("capfd._restore()".to_string());
            } else if dep == "caplog" {
                fixture_cleanup_parts.push("caplog._restore()".to_string());
            } else if dep == "request" {
                fixture_cleanup_parts.push("request._run_finalizers()".to_string());
            }
        } else {
            conftest_fixture_names.push(dep.clone());
            // Don't add to fixture_kwarg_names — conftest fixtures use dynamic kwargs
        }
    }

    // Generate conftest loading code for non-builtin fixtures
    // Uses a dynamic dict (__fastest_fix_kwargs) so unresolved deps
    // don't cause NameError — the function can use its default values.
    let has_conftest_fixtures = !conftest_fixture_names.is_empty();
    if has_conftest_fixtures {
        fixture_setup_parts.push(
            "__fastest_fix_kwargs = {}\n\
             import importlib.util as __fastest_ilu\n\
             import inspect as __fastest_insp\n\
             __fastest_conftest_paths = []\n\
             __fastest_cd = __fastest_test_dir\n\
             while True:\n\
             \x20   __fastest_cp = os.path.join(__fastest_cd, 'conftest.py')\n\
             \x20   if os.path.exists(__fastest_cp):\n\
             \x20       __fastest_conftest_paths.append(__fastest_cp)\n\
             \x20   __fastest_pd = os.path.dirname(__fastest_cd)\n\
             \x20   if __fastest_pd == __fastest_cd:\n\
             \x20       break\n\
             \x20   __fastest_cd = __fastest_pd\n\
             __fastest_conftest_paths.reverse()"
                .to_string(),
        );
        for name in &conftest_fixture_names {
            // Validate fixture name is a safe Python identifier before interpolation
            if !is_valid_python_identifier(name) {
                continue;
            }
            fixture_setup_parts.push(format!(
                "for __fastest_cp in __fastest_conftest_paths:\n\
                 \x20   __fastest_ck = 'conftest_' + __fastest_cp.replace(os.sep, '_').replace('.', '_')\n\
                 \x20   if __fastest_ck in sys.modules:\n\
                 \x20       del sys.modules[__fastest_ck]\n\
                 \x20   import pytest as __fastest_pt\n\
                 \x20   __fastest_orig_fix = __fastest_pt.fixture\n\
                 \x20   __fastest_pt.fixture = lambda f=None, **kw: f if f is not None else (lambda fn: fn)\n\
                 \x20   __fastest_cspec = __fastest_ilu.spec_from_file_location(__fastest_ck, __fastest_cp)\n\
                 \x20   __fastest_cmod = __fastest_ilu.module_from_spec(__fastest_cspec)\n\
                 \x20   __fastest_cspec.loader.exec_module(__fastest_cmod)\n\
                 \x20   __fastest_pt.fixture = __fastest_orig_fix\n\
                 \x20   if hasattr(__fastest_cmod, '{name}'):\n\
                 \x20       __fastest_fix_sig = __fastest_insp.signature(__fastest_cmod.{name})\n\
                 \x20       __fastest_fix_args = {{}}\n\
                 \x20       for __fastest_pn in __fastest_fix_sig.parameters:\n\
                 \x20           if __fastest_pn in __fastest_fix_kwargs:\n\
                 \x20               __fastest_fix_args[__fastest_pn] = __fastest_fix_kwargs[__fastest_pn]\n\
                 \x20           elif __fastest_pn in locals():\n\
                 \x20               __fastest_fix_args[__fastest_pn] = locals()[__fastest_pn]\n\
                 \x20       __fastest_fix_result = __fastest_cmod.{name}(**__fastest_fix_args)\n\
                 \x20       if __fastest_insp.isgenerator(__fastest_fix_result):\n\
                 \x20           try:\n\
                 \x20               __fastest_fix_kwargs['{name}'] = next(__fastest_fix_result)\n\
                 \x20           except StopIteration:\n\
                 \x20               __fastest_fix_kwargs['{name}'] = None\n\
                 \x20       else:\n\
                 \x20           __fastest_fix_kwargs['{name}'] = __fastest_fix_result\n\
                 \x20       break"
            ));
        }
    }

    let fixture_setup = if fixture_setup_parts.is_empty() {
        String::new()
    } else {
        fixture_setup_parts.join("\n") + "\n"
    };

    // Merge parametrize kwargs with fixture kwargs
    let mut all_kwargs_parts: Vec<String> = Vec::new();
    if let Some(ref params) = test.parameters {
        for name in &params.names {
            if let Some(val) = params.values.get(name) {
                all_kwargs_parts.push(format!("{}={}", name, json_value_to_python(val)));
            }
        }
    }
    for name in &fixture_kwarg_names {
        all_kwargs_parts.push(format!("{name}={name}"));
    }
    if has_conftest_fixtures {
        all_kwargs_parts.push("**__fastest_fix_kwargs".to_string());
    }
    let kwargs = all_kwargs_parts.join(", ");

    // Build cleanup code string
    let cleanup_code = fixture_cleanup_parts.join("\n\x20   ");

    // Build the complete call expression with setup/teardown and fixture cleanup
    let call_code = if let Some(ref class_name) = test.class_name {
        let class_name = if is_valid_python_identifier(class_name) {
            class_name.clone()
        } else {
            class_name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect()
        };
        let async_part = if test.is_async {
            "\n\
             \x20   if asyncio.iscoroutine(__fastest_result):\n\
             \x20       asyncio.run(__fastest_result)"
        } else {
            ""
        };
        let cleanup_part = if !fixture_cleanup_parts.is_empty() {
            format!("\n\x20   {cleanup_code}")
        } else {
            String::new()
        };
        let async_import = if test.is_async {
            "import asyncio\n"
        } else {
            ""
        };
        format!(
            "{async_import}\
             __fastest_cls = getattr(__fastest_mod, '{class_name}')\n\
             if not getattr(__fastest_cls, '_fastest_setup_class_done', False):\n\
             \x20   if hasattr(__fastest_cls, 'setup_class'):\n\
             \x20       __fastest_cls.setup_class()\n\
             \x20   __fastest_cls._fastest_setup_class_done = True\n\
             __fastest_instance = __fastest_cls()\n\
             try:\n\
             \x20   if hasattr(__fastest_instance, 'setup_method'):\n\
             \x20       __fastest_instance.setup_method()\n\
             \x20   __fastest_result = getattr(__fastest_instance, '{func_name}')({kwargs}){async_part}\n\
             finally:\n\
             \x20   if hasattr(__fastest_instance, 'teardown_method'):\n\
             \x20       __fastest_instance.teardown_method()\n\
             \x20   if hasattr(__fastest_cls, 'teardown_class') and not getattr(__fastest_cls, '_fastest_teardown_class_done', False):\n\
             \x20       __fastest_cls.teardown_class()\n\
             \x20       __fastest_cls._fastest_teardown_class_done = True{cleanup_part}"
        )
    } else if !fixture_cleanup_parts.is_empty() && test.is_async {
        format!(
            "import asyncio\n\
             if hasattr(__fastest_mod, 'setup_function'):\n\
             \x20   __fastest_mod.setup_function(getattr(__fastest_mod, '{func_name}'))\n\
             try:\n\
             \x20   __fastest_result = getattr(__fastest_mod, '{func_name}')({kwargs})\n\
             \x20   if asyncio.iscoroutine(__fastest_result):\n\
             \x20       asyncio.run(__fastest_result)\n\
             finally:\n\
             \x20   if hasattr(__fastest_mod, 'teardown_function'):\n\
             \x20       __fastest_mod.teardown_function(getattr(__fastest_mod, '{func_name}'))\n\
             \x20   {cleanup_code}"
        )
    } else if !fixture_cleanup_parts.is_empty() {
        format!(
            "if hasattr(__fastest_mod, 'setup_function'):\n\
             \x20   __fastest_mod.setup_function(getattr(__fastest_mod, '{func_name}'))\n\
             try:\n\
             \x20   __fastest_result = getattr(__fastest_mod, '{func_name}')({kwargs})\n\
             finally:\n\
             \x20   if hasattr(__fastest_mod, 'teardown_function'):\n\
             \x20       __fastest_mod.teardown_function(getattr(__fastest_mod, '{func_name}'))\n\
             \x20   {cleanup_code}"
        )
    } else if test.is_async {
        format!(
            "import asyncio\n\
             if hasattr(__fastest_mod, 'setup_function'):\n\
             \x20   __fastest_mod.setup_function(getattr(__fastest_mod, '{func_name}'))\n\
             try:\n\
             \x20   __fastest_result = getattr(__fastest_mod, '{func_name}')({kwargs})\n\
             \x20   if asyncio.iscoroutine(__fastest_result):\n\
             \x20       asyncio.run(__fastest_result)\n\
             finally:\n\
             \x20   if hasattr(__fastest_mod, 'teardown_function'):\n\
             \x20       __fastest_mod.teardown_function(getattr(__fastest_mod, '{func_name}'))"
        )
    } else {
        format!(
            "if hasattr(__fastest_mod, 'setup_function'):\n\
             \x20   __fastest_mod.setup_function(getattr(__fastest_mod, '{func_name}'))\n\
             try:\n\
             \x20   __fastest_result = getattr(__fastest_mod, '{func_name}')({kwargs})\n\
             finally:\n\
             \x20   if hasattr(__fastest_mod, 'teardown_function'):\n\
             \x20       __fastest_mod.teardown_function(getattr(__fastest_mod, '{func_name}'))"
        )
    };

    let parent_path_setup = if parent_dir.is_empty() {
        String::new()
    } else {
        format!(
            "if '{parent}' not in sys.path:\n\
             \x20   sys.path.insert(0, '{parent}')\n",
            parent = parent_dir,
        )
    };

    // Install pytest.raises/warns/approx shim if real pytest isn't available
    let pytest_shim = "\
if not hasattr(sys.modules.get('pytest', None), 'raises'):\n\
\x20   class _PytestRaisesCtx:\n\
\x20       def __init__(s, exc, match=None): s.expected_exception = exc; s.match = match; s.value = None\n\
\x20       def __enter__(s): return s\n\
\x20       def __exit__(s, et, ev, tb):\n\
\x20           if et is None: raise AssertionError(f'DID NOT RAISE {s.expected_exception}')\n\
\x20           if not issubclass(et, s.expected_exception): return False\n\
\x20           s.value = ev\n\
\x20           if s.match:\n\
\x20               import re\n\
\x20               if not re.search(s.match, str(ev)): raise AssertionError(f'{ev!r} does not match {s.match!r}')\n\
\x20           return True\n\
\x20   class _PytestShim:\n\
\x20       class _M:\n\
\x20           def __getattr__(s, n):\n\
\x20               def d(*a, **k): return a[0] if a and callable(a[0]) else (lambda f: f)\n\
\x20               return d\n\
\x20       mark = _M()\n\
\x20       def raises(s, exc, *a, match=None, **kw): return _PytestRaisesCtx(exc, match=match)\n\
\x20       def approx(s, exp, rel=None, abs=None): return exp\n\
\x20       def fixture(s, f=None, **kw): return f if f else (lambda fn: fn)\n\
\x20       def param(s, *v, id=None, marks=()): return v if len(v)!=1 else v[0]\n\
\x20       def skip(s, reason=''): raise Exception(f'SKIPPED: {reason}')\n\
\x20       def fail(s, reason=''): raise AssertionError(reason)\n\
\x20       def importorskip(s, modname, minversion=None, reason=None):\n\
\x20           try:\n\
\x20               mod = __import__(modname)\n\
\x20               if minversion:\n\
\x20                   ver = getattr(mod, '__version__', '')\n\
\x20                   if ver < minversion: raise Exception(f'SKIPPED: {reason or modname+\">=\"+minversion+\" required\"}')\n\
\x20               return mod\n\
\x20           except ImportError: raise Exception(f'SKIPPED: {reason or \"could not import \"+repr(modname)}')\n\
\x20   try:\n\
\x20       import pytest\n\
\x20       if not hasattr(pytest, 'raises'): pytest.raises = _PytestShim().raises\n\
\x20   except ImportError:\n\
\x20       sys.modules['pytest'] = _PytestShim()\n";

    format!(
        "import sys, importlib, os\n\
         __fastest_test_dir = os.path.dirname(os.path.abspath('{path}'))\n\
         if __fastest_test_dir not in sys.path:\n\
         \x20   sys.path.insert(0, __fastest_test_dir)\n\
         {parent_setup}\
         {pytest_shim}\
         import importlib.util as __fastest_ilu2\n\
         __fastest_spec2 = __fastest_ilu2.spec_from_file_location('{module}', '{path}')\n\
         __fastest_mod = __fastest_ilu2.module_from_spec(__fastest_spec2)\n\
         __fastest_spec2.loader.exec_module(__fastest_mod)\n\
         {fixture_setup}\
         {call}",
        path = path_str,
        parent_setup = parent_path_setup,
        pytest_shim = pytest_shim,
        module = module_name,
        fixture_setup = fixture_setup,
        call = call_code,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_item(function_name: &str, class_name: Option<&str>, path: &str) -> TestItem {
        TestItem {
            id: format!("{path}::{function_name}"),
            path: PathBuf::from(path),
            function_name: function_name.into(),
            line_number: Some(1),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: class_name.map(|s| s.into()),
            markers: vec![],
            parameters: None,
            name: function_name.into(),
        }
    }

    #[test]
    fn test_build_test_code_function() {
        let item = make_test_item("test_add", None, "tests/test_math.py");
        let code = build_test_code(&item);

        assert!(code.contains("import sys, importlib, os"));
        assert!(code.contains("test_math"));
        assert!(code.contains("spec_from_file_location"));
        assert!(code.contains("getattr(__fastest_mod, 'test_add')()"));
        // Should NOT contain class instantiation or async handling
        assert!(!code.contains("__fastest_cls"));
        assert!(!code.contains("__fastest_instance"));
        assert!(!code.contains("asyncio"));
    }

    #[test]
    fn test_build_test_code_class_method() {
        let item = make_test_item("test_add", Some("TestCalc"), "tests/test_math.py");
        let code = build_test_code(&item);

        assert!(code.contains("import sys, importlib, os"));
        assert!(code.contains("test_math"));
        assert!(code.contains("getattr(__fastest_mod, 'TestCalc')"));
        assert!(code.contains("__fastest_instance = __fastest_cls()"));
        assert!(code.contains("getattr(__fastest_instance, 'test_add')()"));
        assert!(!code.contains("asyncio"));
    }

    #[test]
    fn test_build_test_code_async_function() {
        let mut item = make_test_item("test_async_add", None, "tests/test_async.py");
        item.is_async = true;
        let code = build_test_code(&item);

        assert!(code.contains("import asyncio"));
        assert!(code.contains("asyncio.iscoroutine(__fastest_result)"));
        assert!(code.contains("asyncio.run(__fastest_result)"));
    }

    #[test]
    fn test_build_test_code_backslash_normalization() {
        let item = make_test_item("test_x", None, r"tests\subdir\test_foo.py");
        let code = build_test_code(&item);

        // Backslashes should be normalized to forward slashes in the path
        assert!(code.contains("tests/subdir/test_foo.py"));
        assert!(code.contains("test_foo"));
    }
}
