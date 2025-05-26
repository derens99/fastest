use super::TestResult;
use crate::discovery::TestItem;
use crate::error::{Error, Result};
use crate::fixtures::{
    generate_builtin_fixture_code, generate_test_code_with_fixtures, is_builtin_fixture,
    FixtureExecutor,
};
use crate::markers::{extract_markers, BuiltinMarker};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::Instant;

/// Run a single test in its own subprocess
pub fn run_test(test: &TestItem) -> Result<TestResult> {
    // If test has fixtures, use fixture-aware execution
    if !test.fixture_deps.is_empty() {
        run_test_with_fixtures(test)
    } else {
        run_test_simple(test)
    }
}

/// Run test with fixture support
fn run_test_with_fixtures(test: &TestItem) -> Result<TestResult> {
    let start = Instant::now();

    // Check for skip markers
    let markers = extract_markers(&test.decorators);
    if let Some(skip_reason) = BuiltinMarker::should_skip(&markers) {
        return Ok(TestResult {
            test_id: test.id.clone(),
            passed: true,
            duration: start.elapsed(),
            output: "SKIPPED".to_string(),
            error: Some(skip_reason.clone()),
            stdout: String::new(),
            stderr: format!("SKIPPED: {}", skip_reason),
        });
    }

    let is_xfail = BuiltinMarker::is_xfail(&markers);

    // Execute fixtures
    let mut fixture_executor = FixtureExecutor::new();
    let mut fixture_values = HashMap::new();

    // Resolve fixtures in dependency order
    for fixture_name in &test.fixture_deps {
        if is_builtin_fixture(fixture_name) {
            // Handle built-in fixtures by injecting their Python code
            if let Some(fixture_code) = generate_builtin_fixture_code(fixture_name) {
                fixture_executor.register_fixture_code(fixture_name.clone(), fixture_code);
            }
        }
    }

    // Execute fixtures
    match fixture_executor.execute_fixtures(&test.fixture_deps, &test.path, &fixture_values) {
        Ok(values) => fixture_values = values,
        Err(e) => {
            return Ok(TestResult {
                test_id: test.id.clone(),
                passed: false,
                duration: start.elapsed(),
                output: "FAILED".to_string(),
                error: Some(format!("Fixture setup failed: {}", e)),
                stdout: String::new(),
                stderr: e.to_string(),
            });
        }
    }

    // Generate test code with fixtures
    let python_code = generate_test_code_with_fixtures(test, &fixture_values);

    // Execute the test
    let output = Command::new("python")
        .arg("-c")
        .arg(&python_code)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::Execution(format!("Failed to execute test: {}", e)))?;

    let duration = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut passed = output.status.success();

    // Handle xfail
    if is_xfail && !passed {
        passed = true;
    } else if is_xfail && passed {
        passed = false;
    }

    let (output_str, error) = if is_xfail && passed {
        ("XPASS (expected failure but passed)".to_string(), None)
    } else if is_xfail && !output.status.success() {
        ("XFAIL (expected failure)".to_string(), None)
    } else if passed {
        ("PASSED".to_string(), None)
    } else {
        (
            "FAILED".to_string(),
            Some(format!(
                "Test failed with exit code: {}\nStderr: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )),
        )
    };

    Ok(TestResult {
        test_id: test.id.clone(),
        passed,
        duration,
        output: output_str,
        error,
        stdout,
        stderr,
    })
}

/// Run test without fixtures (original implementation)
fn run_test_simple(test: &TestItem) -> Result<TestResult> {
    let start = Instant::now();

    // Check for skip markers
    let markers = extract_markers(&test.decorators);
    if let Some(skip_reason) = BuiltinMarker::should_skip(&markers) {
        return Ok(TestResult {
            test_id: test.id.clone(),
            passed: true, // Skipped tests are considered "passed"
            duration: start.elapsed(),
            output: "SKIPPED".to_string(),
            error: Some(skip_reason.clone()),
            stdout: String::new(),
            stderr: format!("SKIPPED: {}", skip_reason),
        });
    }

    // Check if test is expected to fail
    let is_xfail = BuiltinMarker::is_xfail(&markers);

    // Build the Python command to run the test
    let python_code = build_test_runner_code(test);

    // Execute the test in a subprocess
    let output = Command::new("python")
        .arg("-c")
        .arg(&python_code)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| Error::Execution(format!("Failed to execute test: {}", e)))?;

    let duration = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut passed = output.status.success();

    // Handle xfail - if test is marked xfail and it fails, that's a pass
    if is_xfail && !passed {
        passed = true;
    } else if is_xfail && passed {
        // If xfail test passes, that's actually a failure (unexpected pass)
        passed = false;
    }

    let (output_str, error) = if is_xfail && passed {
        ("XPASS (expected failure but passed)".to_string(), None)
    } else if is_xfail && !output.status.success() {
        ("XFAIL (expected failure)".to_string(), None)
    } else if passed {
        ("PASSED".to_string(), None)
    } else {
        (
            "FAILED".to_string(),
            Some(format!(
                "Test failed with exit code: {}\nStderr: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )),
        )
    };

    Ok(TestResult {
        test_id: test.id.clone(),
        passed,
        duration,
        output: output_str,
        error,
        stdout,
        stderr,
    })
}

fn build_test_runner_code(test: &TestItem) -> String {
    // Get the test file's directory for adding to sys.path
    let test_dir = test
        .path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    // Get the module name (just the file without extension)
    let module_name = test
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "test".to_string());

    // Handle both regular and async tests
    let runner_code = if test.is_async {
        format!(
            r#"
import sys
import os
import asyncio
import traceback

# Add the test directory to Python path
sys.path.insert(0, r'{}')

try:
    # Import the test module
    import {} as test_module
    {}
    
    async def run_test():
        try:
            result = await {}
            print("Test passed")
        except Exception as e:
            print(f"Test failed: {{e}}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            sys.exit(1)
    
    asyncio.run(run_test())
except Exception as e:
    print(f"Failed to import or run test: {{e}}", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)
"#,
            test_dir,
            module_name,
            if let Some(class_name) = &test.class_name {
                format!("\n    # Get test class and create instance\n    test_class = getattr(test_module, '{}')\n    test_instance = test_class()", class_name)
            } else {
                format!(
                    "\n    # Get test function\n    test_func = getattr(test_module, '{}')",
                    test.function_name
                )
            },
            if test.class_name.is_some() {
                format!("test_instance.{}()", test.function_name)
            } else {
                "test_func()".to_string()
            }
        )
    } else {
        format!(
            r#"
import sys
import os
import traceback

# Add the test directory to Python path
sys.path.insert(0, r'{}')

try:
    # Import the test module
    import {} as test_module
    {}
    
    # Run the test
    {}
    print("Test passed")
except Exception as e:
    print(f"Test failed: {{e}}", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)
"#,
            test_dir,
            module_name,
            if let Some(class_name) = &test.class_name {
                format!("\n    # Get test class and create instance\n    test_class = getattr(test_module, '{}')\n    test_instance = test_class()", class_name)
            } else {
                format!(
                    "\n    # Get test function\n    test_func = getattr(test_module, '{}')",
                    test.function_name
                )
            },
            if test.class_name.is_some() {
                format!("test_instance.{}()", test.function_name)
            } else {
                "test_func()".to_string()
            }
        )
    };

    runner_code
}
