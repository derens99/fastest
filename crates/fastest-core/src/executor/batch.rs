use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use crate::error::{Error, Result};
use crate::discovery::TestItem;
use super::TestResult;

/// Fast batch executor that runs multiple tests in a single Python process
pub struct BatchExecutor {
    python_path: String,
}

impl BatchExecutor {
    pub fn new() -> Self {
        Self {
            python_path: "python".to_string(),
        }
    }
    
    /// Execute tests grouped by module for maximum efficiency
    pub fn execute_tests(&self, tests: Vec<TestItem>) -> Vec<TestResult> {
        // Group tests by module
        let mut module_groups: HashMap<String, Vec<TestItem>> = HashMap::new();
        for test in tests {
            let module_path = test.path.to_string_lossy().to_string();
            module_groups.entry(module_path).or_insert_with(Vec::new).push(test);
        }
        
        let mut all_results = Vec::new();
        
        // Execute each module's tests in a single subprocess
        for (module_path, module_tests) in module_groups {
            let module_tests_clone = module_tests.clone();
            match self.execute_module_tests(&module_path, module_tests) {
                Ok(results) => all_results.extend(results),
                Err(e) => {
                    // If module execution fails, mark all tests as failed
                    for test in module_tests_clone {
                        all_results.push(TestResult {
                            test_id: test.id,
                            passed: false,
                            duration: Duration::from_secs(0),
                            output: "FAILED".to_string(),
                            error: Some(format!("Module execution failed: {}", e)),
                            stdout: String::new(),
                            stderr: String::new(),
                        });
                    }
                }
            }
        }
        
        all_results
    }
    
    fn execute_module_tests(&self, module_path: &str, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        let start = Instant::now();
        
        // Build optimized runner code
        let runner_code = self.build_optimized_runner(&module_path, &tests);
        
        // Execute all tests in one process
        let output = Command::new(&self.python_path)
            .arg("-c")
            .arg(&runner_code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| Error::Execution(format!("Failed to execute module tests: {}", e)))?;
        
        let total_duration = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        // Parse JSON results
        self.parse_results(&stdout, &stderr, tests, total_duration)
    }
    
    fn build_optimized_runner(&self, module_path: &str, tests: &[TestItem]) -> String {
        let test_dir = std::path::Path::new(module_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        
        // Get the full module name by converting path to module notation
        let module_name = if let Some(test_item) = tests.first() {
            // Extract module path from test ID (e.g., "test_project.tests.test_math::test_add" -> "test_project.tests.test_math")
            test_item.id.split("::").next().unwrap_or("test").to_string()
        } else {
            std::path::Path::new(module_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "test".to_string())
        };
        
        let mut test_specs = String::new();
        for test in tests {
            test_specs.push_str(&format!(
                "    {{'id': '{}', 'name': '{}', 'is_async': {}, 'class_name': {}}},\n",
                test.id,
                test.function_name,
                if test.is_async { "True" } else { "False" },
                test.class_name.as_ref().map_or("None".to_string(), |c| format!("'{}'", c))
            ));
        }
        
        format!(
            r#"
import sys
import json
import time
import traceback
import asyncio
import io
import importlib
from contextlib import redirect_stdout, redirect_stderr

# Add parent directories to sys.path to support package imports
import os
current_dir = r'{}'
while current_dir and current_dir != '/':
    if current_dir not in sys.path:
        sys.path.insert(0, current_dir)
    current_dir = os.path.dirname(current_dir)

# Import module using importlib for better handling of nested modules
try:
    test_module = importlib.import_module('{}')
except Exception as e:
    print(json.dumps({{'error': f'Failed to import module "{}": {{str(e)}}', 'results': []}}))
    sys.exit(1)

# Pre-compile test list
tests = [
{}]

# Pre-fetch test functions for speed
test_funcs = {{}}
for test_spec in tests:
    try:
        if test_spec['class_name']:
            cls = getattr(test_module, test_spec['class_name'])
            instance = cls()
            test_funcs[test_spec['id']] = getattr(instance, test_spec['name'])
        else:
            test_funcs[test_spec['id']] = getattr(test_module, test_spec['name'])
    except AttributeError:
        pass

results = []

# Run tests with minimal overhead
for test_spec in tests:
    test_id = test_spec['id']
    stdout_capture = io.StringIO()
    stderr_capture = io.StringIO()
    
    start = time.perf_counter()
    try:
        test_func = test_funcs.get(test_id)
        if not test_func:
            raise AttributeError(f"Test function not found: {{test_spec['name']}}")
        
        with redirect_stdout(stdout_capture), redirect_stderr(stderr_capture):
            if test_spec['is_async']:
                asyncio.run(test_func())
            else:
                test_func()
        
        duration = time.perf_counter() - start
        results.append({{
            'id': test_id,
            'passed': True,
            'duration': duration,
            'stdout': stdout_capture.getvalue(),
            'stderr': stderr_capture.getvalue(),
            'error': None
        }})
    except Exception as e:
        duration = time.perf_counter() - start
        results.append({{
            'id': test_id,
            'passed': False,
            'duration': duration,
            'stdout': stdout_capture.getvalue(),
            'stderr': stderr_capture.getvalue(),
            'error': str(e),
            'traceback': traceback.format_exc()
        }})

print(json.dumps({{'results': results}}))
"#,
            test_dir,
            module_name,
            module_name,
            test_specs
        )
    }
    
    fn parse_results(&self, stdout: &str, stderr: &str, tests: Vec<TestItem>, total_duration: Duration) -> Result<Vec<TestResult>> {
        let test_count = tests.len();
        if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(stdout) {
            if let Some(results_array) = json_data["results"].as_array() {
                let mut results = Vec::new();
                
                for result_json in results_array {
                    let test_id = result_json["id"].as_str().unwrap_or("");
                    let passed = result_json["passed"].as_bool().unwrap_or(false);
                    let duration_secs = result_json["duration"].as_f64().unwrap_or(0.0);
                    let test_stdout = result_json["stdout"].as_str().unwrap_or("").to_string();
                    let test_stderr = result_json["stderr"].as_str().unwrap_or("").to_string();
                    let error = result_json["error"].as_str().map(String::from);
                    
                    results.push(TestResult {
                        test_id: test_id.to_string(),
                        passed,
                        duration: Duration::from_secs_f64(duration_secs),
                        output: if passed { "PASSED".to_string() } else { "FAILED".to_string() },
                        error,
                        stdout: test_stdout,
                        stderr: test_stderr,
                    });
                }
                
                return Ok(results);
            }
        }
        
        // Fallback for parse errors
        Ok(tests.into_iter().map(|test| TestResult {
            test_id: test.id,
            passed: false,
            duration: total_duration / test_count as u32,
            output: "FAILED".to_string(),
            error: Some(format!("Failed to parse results. Stderr: {}", stderr)),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        }).collect())
    }
} 