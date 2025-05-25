use std::process::{Command, Stdio};
use std::time::Instant;
use crate::error::{Error, Result};
use crate::discovery::TestItem;
use super::TestResult;

/// Run a single test in its own subprocess
pub fn run_test(test: &TestItem) -> Result<TestResult> {
    let start = Instant::now();
    
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
    
    // Check if test passed
    let passed = output.status.success();
    let error = if !passed {
        Some(format!("Test failed with exit code: {}\nStderr: {}", 
            output.status.code().unwrap_or(-1), stderr))
    } else {
        None
    };
    
    Ok(TestResult {
        test_id: test.id.clone(),
        passed,
        duration,
        output: if passed { "PASSED".to_string() } else { "FAILED".to_string() },
        error,
        stdout,
        stderr,
    })
}

fn build_test_runner_code(test: &TestItem) -> String {
    // Get the test file's directory for adding to sys.path
    let test_dir = test.path.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    
    // Get the module name (just the file without extension)
    let module_name = test.path.file_stem()
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
                format!("\n    # Get test function\n    test_func = getattr(test_module, '{}')", test.function_name)
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
                format!("\n    # Get test function\n    test_func = getattr(test_module, '{}')", test.function_name)
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