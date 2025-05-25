use std::process::{Command, Stdio, Child};
use std::time::Instant;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::Sender;
use std::io::{Write, BufRead, BufReader};
use crate::error::{Error, Result};
use crate::discovery::TestItem;
use super::TestResult;

/// A pool of Python worker processes for test execution
pub struct ProcessPool {
    workers: Vec<Worker>,
    job_queue: Arc<Mutex<Vec<TestJob>>>,
    result_tx: Sender<TestResult>,
}

struct Worker {
    process: Child,
    stdin: std::process::ChildStdin,
    stdout: std::process::ChildStdout,
}

struct TestJob {
    test: TestItem,
    job_id: usize,
}

impl ProcessPool {
    /// Create a new process pool with the specified number of workers
    pub fn new(num_workers: usize, result_tx: Sender<TestResult>) -> Result<Self> {
        let mut workers = Vec::new();
        let job_queue = Arc::new(Mutex::new(Vec::new()));
        
        // Start worker processes
        for _ in 0..num_workers {
            let worker_code = r#"
import sys
import json
import time
import traceback
import asyncio
import io
from contextlib import redirect_stdout, redirect_stderr

# Worker process that runs tests sent via stdin
while True:
    line = sys.stdin.readline()
    if not line:
        break
    
    try:
        test_data = json.loads(line)
        
        # Extract test info
        test_id = test_data['id']
        module_path = test_data['path']
        module_name = test_data['module_name']
        function_name = test_data['function_name']
        is_async = test_data['is_async']
        class_name = test_data.get('class_name')
        
        # Add test directory to path
        import os
        test_dir = os.path.dirname(module_path)
        if test_dir not in sys.path:
            sys.path.insert(0, test_dir)
        
        # Import module
        module = __import__(module_name)
        
        # Get test function
        if class_name:
            test_class = getattr(module, class_name)
            test_instance = test_class()
            test_func = getattr(test_instance, function_name)
        else:
            test_func = getattr(module, function_name)
        
        # Capture output
        stdout_capture = io.StringIO()
        stderr_capture = io.StringIO()
        
        # Run test
        start = time.perf_counter()
        try:
            with redirect_stdout(stdout_capture), redirect_stderr(stderr_capture):
                if is_async:
                    asyncio.run(test_func())
                else:
                    test_func()
            
            result = {
                'passed': True,
                'error': None,
                'stdout': stdout_capture.getvalue(),
                'stderr': stderr_capture.getvalue()
            }
        except Exception as e:
            result = {
                'passed': False,
                'error': str(e),
                'stdout': stdout_capture.getvalue(),
                'stderr': stderr_capture.getvalue() + '\n' + traceback.format_exc()
            }
        
        # Send result
        print(json.dumps(result))
        sys.stdout.flush()
        
    except Exception as e:
        result = {
            'passed': False,
            'error': f'Worker error: {str(e)}',
            'stdout': '',
            'stderr': traceback.format_exc()
        }
        print(json.dumps(result))
        sys.stdout.flush()
"#;
            
            let mut child = Command::new("python")
                .arg("-c")
                .arg(worker_code)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| Error::Execution(format!("Failed to spawn worker: {}", e)))?;
            
            let stdin = child.stdin.take().unwrap();
            let stdout = child.stdout.take().unwrap();
            
            workers.push(Worker {
                process: child,
                stdin,
                stdout,
            });
        }
        
        Ok(ProcessPool {
            workers,
            job_queue,
            result_tx,
        })
    }
    
    /// Add tests to the execution queue
    pub fn queue_tests(&self, tests: Vec<TestItem>) {
        let mut queue = self.job_queue.lock().unwrap();
        for (idx, test) in tests.into_iter().enumerate() {
            queue.push(TestJob {
                test,
                job_id: idx,
            });
        }
    }
    
    /// Start processing the queue
    pub fn start(&mut self) {
        let workers = std::mem::take(&mut self.workers);
        let job_queue = Arc::clone(&self.job_queue);
        let result_tx = self.result_tx.clone();
        
        for worker in workers {
            let queue = Arc::clone(&job_queue);
            let tx = result_tx.clone();
            
            thread::spawn(move || {
                process_worker(worker, queue, tx);
            });
        }
    }
}

fn process_worker(mut worker: Worker, job_queue: Arc<Mutex<Vec<TestJob>>>, result_tx: Sender<TestResult>) {
    let mut reader = BufReader::new(worker.stdout);
    
    loop {
        // Get next job
        let job = {
            let mut queue = job_queue.lock().unwrap();
            queue.pop()
        };
        
        let Some(job) = job else {
            // No more jobs
            break;
        };
        
        // Send test to worker
        let test_json = serde_json::json!({
            "id": job.test.id,
            "path": job.test.path.to_string_lossy(),
            "module_name": job.test.path.file_stem().unwrap().to_string_lossy(),
            "function_name": job.test.function_name,
            "is_async": job.test.is_async,
            "class_name": job.test.class_name,
        });
        
        let start = Instant::now();
        
        // Write test to worker stdin
        if let Err(e) = writeln!(worker.stdin, "{}", test_json.to_string()) {
            let _ = result_tx.send(TestResult {
                test_id: job.test.id,
                passed: false,
                duration: start.elapsed(),
                output: format!("Failed to send test to worker: {}", e),
                error: Some(e.to_string()),
                stdout: String::new(),
                stderr: String::new(),
            });
            continue;
        }
        
        // Read result from worker
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // Worker died
                break;
            }
            Ok(_) => {
                // Parse result
                if let Ok(result_json) = serde_json::from_str::<serde_json::Value>(&line) {
                    let passed = result_json["passed"].as_bool().unwrap_or(false);
                    let error = result_json["error"].as_str().map(String::from);
                    let stdout = result_json["stdout"].as_str().unwrap_or("").to_string();
                    let stderr = result_json["stderr"].as_str().unwrap_or("").to_string();
                    
                    let _ = result_tx.send(TestResult {
                        test_id: job.test.id,
                        passed,
                        duration: start.elapsed(),
                        output: if passed { "PASSED".to_string() } else { "FAILED".to_string() },
                        error,
                        stdout,
                        stderr,
                    });
                }
            }
            Err(e) => {
                let _ = result_tx.send(TestResult {
                    test_id: job.test.id,
                    passed: false,
                    duration: start.elapsed(),
                    output: format!("Failed to read from worker: {}", e),
                    error: Some(e.to_string()),
                    stdout: String::new(),
                    stderr: String::new(),
                });
            }
        }
    }
} 