//! PyO3 in-process test executor.
//!
//! Runs Python tests directly inside the current process using the embedded
//! Python interpreter via PyO3. This is fast for small test suites (<=20 tests)
//! because it avoids the overhead of spawning subprocesses.

use std::ffi::CString;
use std::time::Instant;

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
        let timer = TestTimer::start(self.timeout_config.clone());
        let start = Instant::now();

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
                    (TestOutcome::Failed, captured, Some(error_msg))
                }
            }
        });

        let duration = start.elapsed();

        TestResult {
            test_id: test.id.clone(),
            outcome,
            duration,
            output: String::new(),
            error,
            stdout: captured.stdout,
            stderr: captured.stderr,
        }
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

/// Build the Python code string needed to execute a test item.
///
/// The generated code:
/// 1. Adds the test file's parent directory to sys.path
/// 2. Imports the test module
/// 3. For class-based tests: instantiates the class, calls the method
/// 4. For function tests: calls the function directly
pub fn build_test_code(test: &TestItem) -> String {
    let path_str = test.path.to_string_lossy().replace('\\', "/");
    let parent_dir = test
        .path
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();

    let module_name = test
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    let call_code = if let Some(ref class_name) = test.class_name {
        format!(
            "__fastest_cls = getattr(__fastest_mod, '{class_name}')\n\
             __fastest_instance = __fastest_cls()\n\
             getattr(__fastest_instance, '{func_name}')()",
            class_name = class_name,
            func_name = test.function_name,
        )
    } else {
        format!(
            "getattr(__fastest_mod, '{func_name}')()",
            func_name = test.function_name,
        )
    };

    format!(
        r#"import sys, importlib, os
__fastest_test_dir = os.path.dirname(os.path.abspath(r'{path}'))
if __fastest_test_dir not in sys.path:
    sys.path.insert(0, __fastest_test_dir)
if r'{parent}' and r'{parent}' not in sys.path:
    sys.path.insert(0, r'{parent}')
__fastest_mod = importlib.import_module('{module}')
importlib.reload(__fastest_mod)
{call}"#,
        path = path_str,
        parent = parent_dir,
        module = module_name,
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
        assert!(code.contains("importlib.reload"));
        assert!(code.contains("getattr(__fastest_mod, 'test_add')()"));
        // Should NOT contain class instantiation
        assert!(!code.contains("__fastest_cls"));
        assert!(!code.contains("__fastest_instance"));
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
