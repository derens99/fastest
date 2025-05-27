use super::TestResult;
use crate::discovery::TestItem;
use crate::error::{Error, Result};
use crate::utils::PYTHON_CMD;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Number of persistent Python interpreters
const INTERPRETER_POOL_SIZE: usize = 8;

/// Global interpreter pool
static INTERPRETER_POOL: Lazy<Arc<InterpreterPool>> = Lazy::new(|| {
    Arc::new(InterpreterPool::new(INTERPRETER_POOL_SIZE).expect("Failed to create interpreter pool"))
});

/// Message to Python worker
#[derive(Debug, serde::Serialize)]
struct WorkerCommand {
    id: usize,
    tests: Vec<TestData>,
}

#[derive(Debug, serde::Serialize)]
struct TestData {
    id: String,
    module: String,
    func: String,
}

/// Response from Python worker
#[derive(Debug, serde::Deserialize)]
struct WorkerResponse {
    id: usize,
    results: Vec<TestResultData>,
}

#[derive(Debug, serde::Deserialize)]
struct TestResultData {
    id: String,
    passed: bool,
    duration: f64,
    error: Option<String>,
}

/// Ultra-fast Python interpreter wrapper
struct FastInterpreter {
    process: Child,
    stdin: Mutex<std::process::ChildStdin>,
    stdout: Mutex<BufReader<std::process::ChildStdout>>,
    busy: AtomicBool,
    id: usize,
}

impl FastInterpreter {
    fn spawn(id: usize) -> Result<Self> {
        let mut cmd = Command::new(&*PYTHON_CMD);
        cmd.arg("-u") // Unbuffered
            .arg("-O") // Optimize
            .arg("-c")
            .arg(&Self::get_worker_code())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("PYTHONUNBUFFERED", "1")
            .env("PYTHONHASHSEED", "0"); // Deterministic

        let mut process = cmd.spawn()
            .map_err(|e| Error::Execution(format!("Failed to spawn interpreter: {}", e)))?;

        let stdin = process.stdin.take()
            .ok_or_else(|| Error::Execution("Failed to get stdin".to_string()))?;
        
        let stdout = process.stdout.take()
            .ok_or_else(|| Error::Execution("Failed to get stdout".to_string()))?;
        
        let stdout = BufReader::new(stdout);

        Ok(Self {
            process,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(stdout),
            busy: AtomicBool::new(false),
            id,
        })
    }

    fn get_worker_code() -> String {
        r#"
import sys, json, time, importlib, gc, os

# Disable GC during test execution for speed
gc.disable()

# Pre-compile optimizations
import builtins
perf_counter = time.perf_counter
loads = json.loads
dumps = json.dumps

# Module and bytecode cache
module_cache = {}
func_cache = {}

def load_module_fast(module_name):
    if module_name in module_cache:
        return module_cache[module_name]
    
    mod = __import__(module_name)
    module_cache[module_name] = mod
    
    # Cache all test functions
    for name in dir(mod):
        if name.startswith('test_'):
            func = getattr(mod, name)
            if callable(func):
                func_cache[f"{module_name}.{name}"] = func
    
    return mod

def execute_test_ultra_fast(module_name, func_name, test_id):
    """Execute test with absolute minimum overhead"""
    # Try cache first
    cache_key = f"{module_name}.{func_name}"
    if cache_key in func_cache:
        func = func_cache[cache_key]
    else:
        # Load module if needed
        mod = load_module_fast(module_name)
        func = getattr(mod, func_name)
        func_cache[cache_key] = func
    
    # Execute with timing
    start = perf_counter()
    try:
        func()
        return {
            'id': test_id,
            'passed': True,
            'duration': perf_counter() - start,
            'error': None
        }
    except Exception as e:
        return {
            'id': test_id,
            'passed': False,
            'duration': perf_counter() - start,
            'error': str(e)
        }

# Pre-warm Python interpreter
import unittest
import pytest

# Main loop
sys.stdout.write("READY\n")
sys.stdout.flush()

while True:
    try:
        line = sys.stdin.readline()
        if not line:
            break
        
        command = loads(line.strip())
        command_id = command['id']
        tests = command['tests']
        
        # Pre-load all modules for this batch
        modules_to_load = set(test['module'] for test in tests)
        for module in modules_to_load:
            if module not in module_cache:
                load_module_fast(module)
        
        # Execute tests
        results = []
        for test in tests:
            result = execute_test_ultra_fast(test['module'], test['func'], test['id'])
            results.append(result)
        
        # Send results
        response = {'id': command_id, 'results': results}
        sys.stdout.write(dumps(response) + '\n')
        sys.stdout.flush()
        
    except KeyboardInterrupt:
        break
    except Exception as e:
        sys.stderr.write(f"Worker error: {e}\n")
        sys.stderr.flush()
"#.to_string()
    }

    fn execute_batch(&self, command_id: usize, tests: &[TestItem]) -> Result<Vec<TestResult>> {
        // Mark as busy
        self.busy.store(true, Ordering::SeqCst);
        
        let result = self.execute_batch_internal(command_id, tests);
        
        // Mark as available
        self.busy.store(false, Ordering::SeqCst);
        
        result
    }

    fn execute_batch_internal(&self, command_id: usize, tests: &[TestItem]) -> Result<Vec<TestResult>> {
        // Prepare test data
        let test_data: Vec<TestData> = tests.iter().map(|t| {
            let module = t.id.split("::").next().unwrap_or("test").to_string();
            let func = t.id.split("::").nth(1).unwrap_or(&t.function_name).to_string();
            TestData {
                id: t.id.clone(),
                module,
                func,
            }
        }).collect();

        let command = WorkerCommand {
            id: command_id,
            tests: test_data,
        };
        
        // Send command
        {
            let mut stdin = self.stdin.lock();
            writeln!(&mut stdin, "{}", serde_json::to_string(&command)?)?;
            stdin.flush()?;
        }
        
        // Read response
        let response_line = {
            let mut stdout = self.stdout.lock();
            let mut line = String::new();
            stdout.read_line(&mut line)?;
            line
        };
        
        // Parse response
        let response: WorkerResponse = serde_json::from_str(&response_line)
            .map_err(|e| Error::Execution(format!("Failed to parse response: {}", e)))?;
        
        if response.id != command_id {
            return Err(Error::Execution("Response ID mismatch".to_string()));
        }
        
        // Convert to TestResult
        Ok(response.results.into_iter().map(|r| TestResult {
            test_id: r.id,
            passed: r.passed,
            duration: Duration::from_secs_f64(r.duration),
            output: if r.passed { "PASSED" } else { "FAILED" }.to_string(),
            error: r.error,
            stdout: String::new(),
            stderr: String::new(),
        }).collect())
    }

    fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }

    fn is_alive(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }
}

impl Drop for FastInterpreter {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

/// Pool of persistent interpreters
struct InterpreterPool {
    interpreters: Vec<Arc<Mutex<FastInterpreter>>>,
    next_interpreter: AtomicUsize,
    command_counter: AtomicUsize,
}

impl InterpreterPool {
    fn new(size: usize) -> Result<Self> {
        let mut interpreters = Vec::with_capacity(size);
        
        // Spawn interpreters
        for i in 0..size {
            let interpreter = FastInterpreter::spawn(i)?;
            
            // Wait for ready signal
            {
                let mut stdout = interpreter.stdout.lock();
                let mut ready = String::new();
                stdout.read_line(&mut ready)?;
                if !ready.trim().eq("READY") {
                    return Err(Error::Execution("Interpreter failed to start".to_string()));
                }
            }
            
            interpreters.push(Arc::new(Mutex::new(interpreter)));
        }
        
        Ok(Self {
            interpreters,
            next_interpreter: AtomicUsize::new(0),
            command_counter: AtomicUsize::new(0),
        })
    }

    fn execute_batch(&self, tests: &[TestItem]) -> Result<Vec<TestResult>> {
        let command_id = self.command_counter.fetch_add(1, Ordering::Relaxed);
        
        // Find available interpreter
        let start_idx = self.next_interpreter.fetch_add(1, Ordering::Relaxed) % self.interpreters.len();
        
        for i in 0..self.interpreters.len() {
            let idx = (start_idx + i) % self.interpreters.len();
            let interpreter = &self.interpreters[idx];
            
            // Try to lock and check if busy
            if let Some(mut interp) = interpreter.try_lock() {
                if !interp.is_busy() && interp.is_alive() {
                    return interp.execute_batch(command_id, tests);
                }
            }
        }
        
        // All interpreters busy, wait for first one
        let interpreter = &self.interpreters[start_idx];
        let mut interp = interpreter.lock();
        interp.execute_batch(command_id, tests)
    }
}

/// Ultra-fast executor using persistent interpreter pool
pub struct UltraFastExecutor {
    verbose: bool,
}

impl UltraFastExecutor {
    pub fn new(verbose: bool) -> Self {
        // Force initialization of interpreter pool
        let _ = &*INTERPRETER_POOL;
        Self { verbose }
    }

    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        if tests.is_empty() {
            return Ok(vec![]);
        }

        let start = Instant::now();

        // Group tests by module for optimal performance
        let mut module_tests: HashMap<String, Vec<TestItem>> = HashMap::new();
        for test in tests {
            let module = test.id.split("::").next().unwrap_or("test").to_string();
            module_tests.entry(module).or_default().push(test);
        }

        // Create optimal batches
        let mut batches = Vec::new();
        let batch_size = 50; // Optimal size for interpreter reuse
        
        for (_, mut tests) in module_tests {
            // Sort by function name for cache locality
            tests.sort_by(|a, b| a.function_name.cmp(&b.function_name));
            
            if tests.len() <= batch_size {
                batches.push(tests);
            } else {
                for chunk in tests.chunks(batch_size) {
                    batches.push(chunk.to_vec());
                }
            }
        }

        if self.verbose {
            eprintln!("⚡ Executing {} tests in {} batches using ultra-fast executor", 
                batches.iter().map(|b| b.len()).sum::<usize>(), 
                batches.len()
            );
        }

        // Execute batches in parallel
        let results: Vec<TestResult> = batches
            .into_par_iter()
            .flat_map(|batch| {
                INTERPRETER_POOL.execute_batch(&batch).unwrap_or_else(|e| {
                    if self.verbose {
                        eprintln!("Batch execution failed: {}", e);
                    }
                    // Return failed results
                    batch.into_iter().map(|test| TestResult {
                        test_id: test.id,
                        passed: false,
                        duration: Duration::from_secs(0),
                        output: "FAILED".to_string(),
                        error: Some(format!("Execution failed: {}", e)),
                        stdout: String::new(),
                        stderr: String::new(),
                    }).collect()
                })
            })
            .collect();

        if self.verbose {
            let duration = start.elapsed();
            eprintln!("✅ All tests completed in {:.1}ms ({:.1}ms per test)", 
                duration.as_secs_f64() * 1000.0,
                (duration.as_secs_f64() * 1000.0) / results.len() as f64
            );
        }

        Ok(results)
    }
}