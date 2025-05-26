use super::TestResult;
use crate::discovery::TestItem;
use crate::error::{Error, Result};
use crate::markers::{extract_markers, BuiltinMarker};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

// Global Python process pool for reuse
static PYTHON_POOL: Lazy<Arc<RwLock<Vec<PythonWorker>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

struct PythonWorker {
    process: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::process::ChildStdout,
}

/// Ultra-optimized test executor using persistent Python processes
pub struct OptimizedExecutor {
    num_workers: usize,
    use_persistent_workers: bool,
    batch_size: usize,
    verbose: bool,
}

impl OptimizedExecutor {
    pub fn new(num_workers: Option<usize>, verbose: bool) -> Self {
        let num_workers = num_workers.unwrap_or_else(|| {
            // Use 2x CPU cores for I/O bound tests
            num_cpus::get().saturating_mul(2)
        });

        Self {
            num_workers,
            use_persistent_workers: true,
            batch_size: 50, // Optimal batch size based on benchmarks
            verbose,
        }
    }

    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        if tests.is_empty() {
            return Ok(vec![]);
        }

        let start = Instant::now();
        
        // Step 1: Pre-filter skip tests (no subprocess needed)
        let (skip_results, tests_to_run) = self.preprocess_tests(tests);
        
        // Step 2: Group tests optimally
        let test_batches = self.create_optimal_batches(tests_to_run);
        
        if self.verbose {
            eprintln!("⚡ Executing {} tests in {} batches", 
                     test_batches.iter().map(|b| b.len()).sum::<usize>(),
                     test_batches.len());
        }

        // Step 3: Execute in parallel with work stealing
        let results = Arc::new(DashMap::new());
        let results_clone = results.clone();
        
        test_batches.into_par_iter().for_each(|batch| {
            if let Ok(batch_results) = self.execute_batch_optimized(batch) {
                for result in batch_results {
                    results_clone.insert(result.test_id.clone(), result);
                }
            }
        });

        // Collect results - clone the Arc and then iterate
        let results_map = Arc::try_unwrap(results).unwrap_or_else(|arc| (*arc).clone());
        let mut all_results: Vec<TestResult> = results_map
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        
        // Add skip results
        all_results.extend(skip_results);
        
        if self.verbose {
            let duration = start.elapsed();
            eprintln!("✅ All tests completed in {:.2}s", duration.as_secs_f64());
        }

        Ok(all_results)
    }

    fn preprocess_tests(&self, tests: Vec<TestItem>) -> (Vec<TestResult>, Vec<TestItem>) {
        let mut skip_results = Vec::new();
        let mut tests_to_run = Vec::new();

        for test in tests {
            let markers = extract_markers(&test.decorators);
            if let Some(skip_reason) = BuiltinMarker::should_skip(&markers) {
                skip_results.push(TestResult {
                    test_id: test.id.clone(),
                    passed: true,
                    duration: Duration::from_secs(0),
                    output: "SKIPPED".to_string(),
                    error: Some(skip_reason.clone()),
                    stdout: String::new(),
                    stderr: format!("SKIPPED: {}", skip_reason),
                });
            } else {
                tests_to_run.push(test);
            }
        }

        (skip_results, tests_to_run)
    }

    fn create_optimal_batches(&self, tests: Vec<TestItem>) -> Vec<Vec<TestItem>> {
        // Group by module first
        let mut module_groups: HashMap<String, Vec<TestItem>> = HashMap::new();
        for test in tests {
            let module = test.path.to_string_lossy().to_string();
            module_groups.entry(module).or_default().push(test);
        }

        // Create batches that balance locality with parallelism
        let mut batches = Vec::new();
        for (_, module_tests) in module_groups {
            // Split large modules into smaller batches
            for chunk in module_tests.chunks(self.batch_size) {
                batches.push(chunk.to_vec());
            }
        }

        batches
    }

    fn execute_batch_optimized(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        // Build ultra-optimized Python code
        let runner_code = self.build_ultra_fast_runner(&tests);
        
        // Use persistent worker if available
        if self.use_persistent_workers {
            if let Some(result) = self.try_persistent_worker(&runner_code) {
                return result;
            }
        }

        // Fallback to subprocess
        self.execute_subprocess(&runner_code)
    }

    fn build_ultra_fast_runner(&self, tests: &[TestItem]) -> String {
        // Group tests by module for single import
        let mut module_tests: HashMap<String, Vec<&TestItem>> = HashMap::new();
        for test in tests {
            // Extract module name from test ID, handling leading dots
            let test_id = test.id.trim_start_matches('.');
            let module = test_id.split("::").next().unwrap_or("test");
            module_tests.entry(module.to_string()).or_default().push(test);
        }

        let mut imports = String::new();
        let mut test_map = String::new();
        
        for (module, tests) in module_tests {
            // Convert module path to Python import format
            let import_module = module.replace('/', ".").replace('\\', ".");
            imports.push_str(&format!("import {}\n", import_module));
            
            for test in tests {
                let is_xfail = BuiltinMarker::is_xfail(&extract_markers(&test.decorators));
                
                // Extract the function path after the module name
                let test_id = test.id.trim_start_matches('.');
                let function_path = if let Some(pos) = test_id.find("::") {
                    &test_id[pos+2..]
                } else {
                    &test.function_name
                };
                
                // Check if this is a parametrized test
                let params_decorator = test.decorators.iter()
                    .find(|d| d.starts_with("__params__="))
                    .map(|d| d.trim_start_matches("__params__="));
                
                // Extract base function name (without parameter suffix)
                let base_function_name = if let Some(bracket_pos) = function_path.find('[') {
                    &function_path[..bracket_pos]
                } else {
                    function_path
                };
                
                // For class methods, extract just the method name without class prefix
                let method_name = if test.class_name.is_some() {
                    // If function_path contains "::", extract only the method name
                    if let Some(pos) = base_function_name.rfind("::") {
                        &base_function_name[pos+2..]
                    } else {
                        base_function_name
                    }
                } else {
                    base_function_name
                };
                
                if let Some(params_json) = params_decorator {
                    // This is a parametrized test
                    test_map.push_str(&format!(
                        "    '{}': {{'func': {}.{}, 'async': {}, 'xfail': {}, 'params': json.loads('{}')}},\n",
                        test.id,
                        import_module,
                        if let Some(class) = &test.class_name {
                            format!("{}().{}", class, method_name)
                        } else {
                            base_function_name.to_string()
                        },
                        if test.is_async { "True" } else { "False" },
                        if is_xfail { "True" } else { "False" },
                        params_json.replace('\'', "\\'")
                    ));
                } else {
                    // Regular test
                    test_map.push_str(&format!(
                        "    '{}': {{'func': {}.{}, 'async': {}, 'xfail': {}, 'params': None}},\n",
                        test.id,
                        import_module,
                        if let Some(class) = &test.class_name {
                            format!("{}().{}", class, test.function_name)
                        } else {
                            function_path.to_string()
                        },
                        if test.is_async { "True" } else { "False" },
                        if is_xfail { "True" } else { "False" }
                    ));
                }
            }
        }

        format!(
            r#"
import sys
import os
import json
import time
import asyncio
import traceback
from io import StringIO
from contextlib import redirect_stdout, redirect_stderr

# Add current directory to Python path
sys.path.insert(0, os.getcwd())

# Pre-import all modules
{}

# Pre-compiled test map
test_map = {{
{}
}}

# Ultra-fast test executor
async def run_all_tests():
    results = []
    
    for test_id, test_info in test_map.items():
        stdout_buf = StringIO()
        stderr_buf = StringIO()
        
        start = time.perf_counter()
        try:
            with redirect_stdout(stdout_buf), redirect_stderr(stderr_buf):
                # Call test with parameters if provided
                if test_info.get('params'):
                    # Convert params dict to function arguments
                    if test_info['async']:
                        await test_info['func'](**test_info['params'])
                    else:
                        test_info['func'](**test_info['params'])
                else:
                    # Regular test without parameters
                    if test_info['async']:
                        await test_info['func']()
                    else:
                        test_info['func']()
            
            duration = time.perf_counter() - start
            
            if test_info['xfail']:
                # Unexpected pass
                results.append({{
                    'id': test_id,
                    'passed': False,
                    'duration': duration,
                    'stdout': stdout_buf.getvalue(),
                    'stderr': stderr_buf.getvalue(),
                    'error': 'XPASS: Test marked as xfail but passed'
                }})
            else:
                results.append({{
                    'id': test_id,
                    'passed': True,
                    'duration': duration,
                    'stdout': stdout_buf.getvalue(),
                    'stderr': stderr_buf.getvalue()
                }})
                
        except Exception as e:
            duration = time.perf_counter() - start
            
            if test_info['xfail']:
                # Expected failure
                results.append({{
                    'id': test_id,
                    'passed': True,
                    'duration': duration,
                    'stdout': stdout_buf.getvalue(),
                    'stderr': stderr_buf.getvalue(),
                    'xfail': True
                }})
            else:
                results.append({{
                    'id': test_id,
                    'passed': False,
                    'duration': duration,
                    'stdout': stdout_buf.getvalue(),
                    'stderr': stderr_buf.getvalue(),
                    'error': str(e),
                    'traceback': traceback.format_exc()
                }})
    
    return results

# Run with minimal overhead
results = asyncio.run(run_all_tests())
print(json.dumps({{'results': results}}))
"#,
            imports, test_map
        )
    }

    fn try_persistent_worker(&self, _code: &str) -> Option<Result<Vec<TestResult>>> {
        // TODO: Implement persistent worker pool
        None
    }

    fn execute_subprocess(&self, code: &str) -> Result<Vec<TestResult>> {
        // Debug: write code to file
        if self.verbose {
            if let Ok(mut file) = std::fs::File::create("/tmp/fastest_debug.py") {
                let _ = write!(file, "{}", code);
                eprintln!("Debug: Python code written to /tmp/fastest_debug.py");
            }
        }
        
        let output = Command::new("python")
            .arg("-c")
            .arg(code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| Error::Execution(format!("Failed to execute tests: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if self.verbose && !stderr.is_empty() {
            eprintln!("Python stderr: {}", stderr);
        }

        if self.verbose {
            eprintln!("Python stdout length: {}", stdout.len());
            if stdout.len() < 1000 {
                eprintln!("Python stdout: {}", stdout);
            }
        }

        if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(results_array) = json_data["results"].as_array() {
                if self.verbose {
                    eprintln!("Found {} results in JSON", results_array.len());
                }
                let results = results_array
                    .iter()
                    .filter_map(|r| self.parse_test_result(r))
                    .collect();
                return Ok(results);
            } else {
                if self.verbose {
                    eprintln!("No results array in JSON");
                }
            }
        } else {
            if self.verbose {
                eprintln!("Failed to parse JSON from stdout");
            }
        }

        // Fallback error
        Err(Error::Execution(format!(
            "Failed to parse results. Stderr: {}",
            stderr
        )))
    }

    fn parse_test_result(&self, json: &serde_json::Value) -> Option<TestResult> {
        let test_id = json["id"].as_str()?.to_string();
        let passed = json["passed"].as_bool().unwrap_or(false);
        let duration = Duration::from_secs_f64(json["duration"].as_f64().unwrap_or(0.0));
        let stdout = json["stdout"].as_str().unwrap_or("").to_string();
        let stderr = json["stderr"].as_str().unwrap_or("").to_string();
        let error = json["error"].as_str().map(String::from);
        
        let is_xfail = json.get("xfail").and_then(|v| v.as_bool()).unwrap_or(false);
        let output = if is_xfail {
            "XFAIL".to_string()
        } else if passed {
            "PASSED".to_string()
        } else {
            "FAILED".to_string()
        };

        Some(TestResult {
            test_id,
            passed,
            duration,
            output,
            error,
            stdout,
            stderr,
        })
    }
}

/// Additional optimizations for specific scenarios
impl OptimizedExecutor {
    /// Execute tests with fixture caching
    pub fn execute_with_fixtures(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        // TODO: Implement fixture caching across test runs
        self.execute(tests)
    }
    
    /// Execute with test result caching for re-runs
    pub fn execute_with_cache(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        // TODO: Cache test results based on file content hash
        self.execute(tests)
    }
} 