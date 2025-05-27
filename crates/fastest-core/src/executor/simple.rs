use super::TestResult;
use crate::discovery::TestItem;
use crate::error::{Error, Result};
use crate::utils::PYTHON_CMD;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Ultra-simple executor optimized for speed over features
pub struct SimpleExecutor {
    verbose: bool,
}

impl SimpleExecutor {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        if tests.is_empty() {
            return Ok(vec![]);
        }

        let start = Instant::now();

        // For simple tests, run everything in a single subprocess
        let code = self.build_simple_runner(&tests);
        
        if self.verbose {
            eprintln!("⚡ Executing {} tests in single process", tests.len());
        }

        let results = self.execute_subprocess(&code)?;

        if self.verbose {
            let duration = start.elapsed();
            eprintln!("✅ All tests completed in {:.2}s", duration.as_secs_f64());
        }

        Ok(results)
    }

    fn build_simple_runner(&self, tests: &[TestItem]) -> String {
        // Group tests by module for efficient imports
        let mut module_tests: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
        
        for test in tests {
            let module = test.id.split("::").next().unwrap_or("test");
            let func_name = test.id.split("::").nth(1).unwrap_or(&test.function_name);
            module_tests.entry(module).or_default().push((func_name, &test.id));
        }

        // Build ultra-minimal Python code
        let mut code = String::with_capacity(512 + tests.len() * 128);
        
        // Minimal imports
        code.push_str("import sys,json,time\n");
        
        // Import all test modules
        for module in module_tests.keys() {
            code.push_str(&format!("import {}\n", module));
        }
        
        // Pre-allocate results
        code.push_str(&format!("r=[]\n"));
        code.push_str("p=time.perf_counter\n");
        
        // Execute all tests
        for (module, module_tests) in module_tests {
            for (func_name, test_id) in module_tests {
                code.push_str(&format!(
                    "s=p()\ntry:\n    {}.{}()\n    r.append({{'id':'{}','passed':True,'duration':p()-s,'stdout':'','stderr':''}})\nexcept Exception as e:\n    r.append({{'id':'{}','passed':False,'duration':p()-s,'stdout':'','stderr':'','error':str(e)}})\n",
                    module, func_name, test_id, test_id
                ));
            }
        }
        
        code.push_str("print(json.dumps({'results':r}))\n");
        code
    }

    fn execute_subprocess(&self, code: &str) -> Result<Vec<TestResult>> {
        let mut cmd = Command::new(&*PYTHON_CMD);
        cmd.arg("-c")
            .arg(code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Preserve environment
        if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
            cmd.env("VIRTUAL_ENV", venv);
        }
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }

        let output = cmd
            .output()
            .map_err(|e| Error::Execution(format!("Failed to execute tests: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if self.verbose && !stderr.is_empty() {
            eprintln!("Python stderr: {}", stderr);
        }

        // Parse results
        if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(results_array) = json_data["results"].as_array() {
                let results = results_array
                    .iter()
                    .filter_map(|r| self.parse_test_result(r))
                    .collect();
                return Ok(results);
            }
        }

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

        let output = if passed {
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