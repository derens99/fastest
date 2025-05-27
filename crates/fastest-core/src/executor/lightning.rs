use super::TestResult;
use crate::discovery::TestItem;
use crate::error::{Error, Result};
use crate::utils::PYTHON_CMD;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Global compiled Python code cache
static PYTHON_CODE_CACHE: Lazy<Arc<Mutex<HashMap<String, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Lightning-fast executor using aggressive optimizations
pub struct LightningExecutor {
    verbose: bool,
}

impl LightningExecutor {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        if tests.is_empty() {
            return Ok(vec![]);
        }

        let start = Instant::now();

        // Group tests by module and sort for cache efficiency
        let mut module_tests: HashMap<String, Vec<TestItem>> = HashMap::new();
        for test in tests {
            let module = test.id.split("::").next().unwrap_or("test").to_string();
            module_tests.entry(module).or_default().push(test);
        }

        // Sort tests within modules for better cache locality
        for tests in module_tests.values_mut() {
            tests.sort_by(|a, b| a.function_name.cmp(&b.function_name));
        }

        // Create single mega-batch for small test counts
        let total_tests: usize = module_tests.values().map(|v| v.len()).sum();

        let results = if total_tests <= 200 {
            // For small test counts, run everything in a single process
            if self.verbose {
                eprintln!(
                    "⚡ Executing {} tests in single lightning-fast process",
                    total_tests
                );
            }

            let all_tests: Vec<TestItem> = module_tests.into_values().flatten().collect();
            self.execute_single_batch(all_tests)?
        } else {
            // For larger test counts, use parallel execution
            if self.verbose {
                eprintln!(
                    "⚡ Executing {} tests in parallel lightning batches",
                    total_tests
                );
            }

            let batches: Vec<Vec<TestItem>> = module_tests.into_values().collect();

            batches
                .into_par_iter()
                .flat_map(|batch| {
                    self.execute_single_batch(batch).unwrap_or_else(|e| {
                        if self.verbose {
                            eprintln!("Batch failed: {}", e);
                        }
                        vec![]
                    })
                })
                .collect()
        };

        if self.verbose {
            let duration = start.elapsed();
            eprintln!(
                "✅ All tests completed in {:.1}ms ({:.2}ms per test)",
                duration.as_secs_f64() * 1000.0,
                (duration.as_secs_f64() * 1000.0) / results.len() as f64
            );
        }

        Ok(results)
    }

    fn execute_single_batch(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        let code = self.build_lightning_code(&tests);

        let mut cmd = Command::new(&*PYTHON_CMD);
        cmd.arg("-u")
            .arg("-O") // Optimize
            .arg("-c")
            .arg(&code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("PYTHONUNBUFFERED", "1");

        // Preserve virtual environment
        if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
            cmd.env("VIRTUAL_ENV", venv);
        }
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }

        let output = cmd
            .output()
            .map_err(|e| Error::Execution(format!("Failed to execute: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if self.verbose && !stderr.is_empty() {
            eprintln!("Python stderr: {}", stderr);
        }

        // Parse results
        if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(results_array) = json_data["results"].as_array() {
                return Ok(results_array
                    .iter()
                    .filter_map(|r| self.parse_result(r))
                    .collect());
            }
        }

        Err(Error::Execution(format!(
            "Failed to parse results. Stderr: {}",
            stderr
        )))
    }

    fn build_lightning_code(&self, tests: &[TestItem]) -> String {
        // Check cache first
        let cache_key = format!("{:?}", tests.iter().map(|t| &t.id).collect::<Vec<_>>());

        {
            let cache = PYTHON_CODE_CACHE.lock();
            if let Some(code) = cache.get(&cache_key) {
                return code.clone();
            }
        }

        // Build ultra-optimized Python code
        let mut code = String::with_capacity(512 + tests.len() * 128);

        // Minimal imports with aliases
        code.push_str(
            r#"import sys,json,time,os
sys.path.insert(0,os.getcwd())
"#,
        );

        // Add unique test directories to sys.path
        let mut test_dirs = std::collections::HashSet::new();
        for test in tests {
            if let Some(parent) = test.path.parent() {
                test_dirs.insert(parent.to_string_lossy().to_string());
            }
        }
        for dir in test_dirs {
            code.push_str(&format!("sys.path.insert(0,'{}')\n", dir));
        }

        code.push_str("p=time.perf_counter\nr=[]\n");

        // Group by module for efficient imports
        let mut modules = std::collections::HashSet::new();
        let mut test_map = Vec::new();

        for test in tests {
            let full_module = test.id.split("::").next().unwrap_or("test");
            let func = test.id.split("::").nth(1).unwrap_or(&test.function_name);

            // Extract just the module name without directory
            let module = if let Some(pos) = full_module.rfind('/') {
                &full_module[pos + 1..]
            } else if let Some(pos) = full_module.rfind('\\') {
                &full_module[pos + 1..]
            } else {
                full_module
            };

            modules.insert(module.to_string());
            test_map.push((module.to_string(), func.to_string(), test.id.clone()));
        }

        // Import modules
        for module in &modules {
            code.push_str(&format!("import {}\n", module));
        }

        // Pre-fetch all functions for speed
        code.push_str("f={}\n");
        for (module, func, _) in &test_map {
            code.push_str(&format!("f['{}::{}']={}.{}\n", module, func, module, func));
        }

        // Execute tests with minimal overhead
        code.push_str("for k,v in f.items():\n s=p()\n try:\n  v()\n  r.append({'id':k,'p':1,'d':p()-s})\n except Exception as e:\n  r.append({'id':k,'p':0,'d':p()-s,'e':str(e)})\n");

        // Map results back to original IDs
        code.push_str("m={");
        for (module, func, id) in &test_map {
            code.push_str(&format!("'{}::{}':'{}',", module, func, id));
        }
        code.push_str("}\n");

        // Output with ID mapping
        code.push_str("print(json.dumps({'results':[{'id':m[x['id']],'passed':x['p'],'duration':x['d'],'error':x.get('e')} for x in r]}))\n");

        // Cache the code
        {
            let mut cache = PYTHON_CODE_CACHE.lock();
            cache.insert(cache_key, code.clone());
        }

        code
    }

    fn parse_result(&self, json: &serde_json::Value) -> Option<TestResult> {
        let test_id = json["id"].as_str()?.to_string();
        let passed = json["passed"]
            .as_u64()
            .map(|v| v == 1)
            .or_else(|| json["passed"].as_bool())
            .unwrap_or(false);
        let duration = Duration::from_secs_f64(json["duration"].as_f64().unwrap_or(0.0));
        let error = json["error"].as_str().map(String::from);

        Some(TestResult {
            test_id,
            passed,
            duration,
            output: if passed { "PASSED" } else { "FAILED" }.to_string(),
            error,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}
