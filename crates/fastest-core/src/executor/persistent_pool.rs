use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct WorkerCommand {
    id: usize,
    code: String,
}

#[derive(Debug, Deserialize)]
struct WorkerResponse {
    id: usize,
    results: Vec<serde_json::Value>,
    error: Option<String>,
}

/// A persistent Python worker that stays alive between test runs
pub struct PersistentWorker {
    process: Child,
    stdin: Mutex<std::process::ChildStdin>,
    response_rx: Receiver<WorkerResponse>,
    _reader_thread: thread::JoinHandle<()>,
}

impl PersistentWorker {
    pub fn spawn() -> Result<Self> {
        let mut process = Command::new("python")
            .arg("-u") // Unbuffered output
            .arg("-c")
            .arg(WORKER_CODE)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;

        let (tx, rx) = channel();

        // Spawn reader thread
        let reader_thread = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Ok(response) = serde_json::from_str::<WorkerResponse>(&line) {
                        let _ = tx.send(response);
                    }
                }
            }
        });

        Ok(Self {
            process,
            stdin: Mutex::new(stdin),
            response_rx: rx,
            _reader_thread: reader_thread,
        })
    }

    pub fn execute(&self, id: usize, code: &str) -> Result<Vec<serde_json::Value>> {
        // Send command
        let command = WorkerCommand {
            id,
            code: code.to_string(),
        };

        let mut stdin = self.stdin.lock().unwrap();
        writeln!(stdin, "{}", serde_json::to_string(&command)?)?;
        stdin.flush()?;
        drop(stdin);

        // Wait for response with timeout
        match self.response_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(response) if response.id == id => {
                if let Some(error) = response.error {
                    Err(anyhow!("Worker error: {}", error))
                } else {
                    Ok(response.results)
                }
            }
            Ok(_) => Err(anyhow!("Received response for wrong command ID")),
            Err(_) => Err(anyhow!("Worker timeout")),
        }
    }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }
}

impl Drop for PersistentWorker {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

/// Pool of persistent Python workers
pub struct PersistentWorkerPool {
    workers: Arc<Mutex<Vec<PersistentWorker>>>,
    _size: usize,
    next_worker: Arc<Mutex<usize>>,
}

impl PersistentWorkerPool {
    pub fn new(size: usize) -> Result<Self> {
        let mut workers = Vec::with_capacity(size);

        // Pre-spawn workers
        for _ in 0..size {
            workers.push(PersistentWorker::spawn()?);
        }

        Ok(Self {
            workers: Arc::new(Mutex::new(workers)),
            _size: size,
            next_worker: Arc::new(Mutex::new(0)),
        })
    }

    pub fn execute(&self, code: &str) -> Result<Vec<serde_json::Value>> {
        let mut workers = self.workers.lock().unwrap();

        // Find available worker or spawn new one
        let worker_count = workers.len();
        for i in 0..worker_count {
            if workers[i].is_alive() {
                return workers[i].execute(i, code);
            }
        }

        // All workers dead, spawn new one
        let new_worker = PersistentWorker::spawn()?;
        let result = new_worker.execute(worker_count, code);
        workers.push(new_worker);

        result
    }

    pub fn shutdown(&self) {
        let mut workers = self.workers.lock().unwrap();
        workers.clear(); // Drop will kill processes
    }
}

// Python code for persistent worker
const WORKER_CODE: &str = r#"
import sys
import json
import traceback
import importlib
import asyncio
from io import StringIO
from contextlib import redirect_stdout, redirect_stderr

# Pre-load common test modules
import unittest
import pytest
import time

# Cache for imported modules
module_cache = {}

def execute_test_code(code):
    """Execute test code and return results."""
    # Create a new namespace for execution
    namespace = {
        '__name__': '__main__',
        'sys': sys,
        'json': json,
        'time': time,
        'asyncio': asyncio,
        'StringIO': StringIO,
        'redirect_stdout': redirect_stdout,
        'redirect_stderr': redirect_stderr,
        'traceback': traceback
    }
    
    # Execute the code
    exec(code, namespace)
    
    # Extract results
    return namespace.get('results', [])

# Main worker loop
while True:
    try:
        line = sys.stdin.readline()
        if not line:
            break
            
        command = json.loads(line.strip())
        command_id = command['id']
        code = command['code']
        
        try:
            results = execute_test_code(code)
            response = {
                'id': command_id,
                'results': results,
                'error': None
            }
        except Exception as e:
            response = {
                'id': command_id,
                'results': [],
                'error': f"{type(e).__name__}: {str(e)}\n{traceback.format_exc()}"
            }
        
        print(json.dumps(response))
        sys.stdout.flush()
        
    except Exception as e:
        # Fatal error, exit
        print(json.dumps({
            'id': -1,
            'results': [],
            'error': f"Worker fatal error: {str(e)}"
        }))
        sys.stdout.flush()
        break
"#;
