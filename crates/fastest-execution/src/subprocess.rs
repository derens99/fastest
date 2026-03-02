//! Subprocess pool executor with work-stealing.
//!
//! Spawns N persistent Python worker processes that communicate via
//! JSON over stdin/stdout. Uses `crossbeam_deque::Injector` for
//! fair work distribution across workers.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crossbeam_deque::{Injector, Steal};
use fastest_core::model::{TestItem, TestOutcome, TestResult};
use serde::{Deserialize, Serialize};

use crate::capture::CapturedOutput;
use crate::timeout::TimeoutConfig;

/// The embedded Python worker harness script.
const WORKER_HARNESS: &str = include_str!("worker_harness.py");

/// JSON payload sent to a worker process.
#[derive(Debug, Serialize)]
struct WorkerInput {
    id: String,
    path: String,
    function_name: String,
    class_name: Option<String>,
}

impl WorkerInput {
    fn from_test_item(item: &TestItem) -> Self {
        Self {
            id: item.id.clone(),
            path: item.path.to_string_lossy().to_string(),
            function_name: item.function_name.clone(),
            class_name: item.class_name.clone(),
        }
    }
}

/// JSON result received from a worker process.
#[derive(Debug, Deserialize)]
pub struct WorkerResult {
    pub test_id: Option<String>,
    pub outcome: Option<String>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

impl WorkerResult {
    /// Convert the worker result into a TestResult.
    pub fn into_test_result(self, fallback_id: &str) -> TestResult {
        let test_id = self.test_id.unwrap_or_else(|| fallback_id.to_string());
        let duration = Duration::from_millis(self.duration_ms.unwrap_or(0));
        let captured = CapturedOutput::from_strings(
            self.stdout.unwrap_or_default(),
            self.stderr.unwrap_or_default(),
        );

        let outcome = match self.outcome.as_deref() {
            Some("Passed") => TestOutcome::Passed,
            Some("Failed") => TestOutcome::Failed,
            _ => TestOutcome::Error {
                message: self
                    .error
                    .clone()
                    .unwrap_or_else(|| "Unknown worker error".into()),
            },
        };

        TestResult {
            test_id,
            outcome,
            duration,
            output: String::new(),
            error: self.error,
            stdout: captured.stdout,
            stderr: captured.stderr,
        }
    }
}

/// Parse a JSON line from a worker process into a WorkerResult.
pub fn parse_worker_result(json_line: &str) -> Result<WorkerResult, serde_json::Error> {
    serde_json::from_str(json_line)
}

/// Locate a Python interpreter on the system.
pub fn find_python() -> Option<String> {
    // Check PYO3_PYTHON env var first
    if let Ok(python) = std::env::var("PYO3_PYTHON") {
        return Some(python);
    }

    // Try common Python executable names
    let candidates = ["python3", "python", "python3.12", "python3.11"];
    for candidate in &candidates {
        if which::which(candidate).is_ok() {
            return Some(candidate.to_string());
        }
    }

    None
}

/// A pool of persistent Python subprocess workers.
pub struct SubprocessPool {
    num_workers: usize,
    python_path: String,
    timeout_config: TimeoutConfig,
}

impl SubprocessPool {
    /// Create a new subprocess pool.
    ///
    /// `num_workers` defaults to the number of CPUs if `None`.
    pub fn new(num_workers: Option<usize>) -> Self {
        let workers = num_workers.unwrap_or_else(num_cpus::get);
        let python = find_python().unwrap_or_else(|| "python3".into());
        Self {
            num_workers: workers.max(1),
            python_path: python,
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Create a subprocess pool with a specific Python path.
    pub fn with_python(num_workers: Option<usize>, python_path: String) -> Self {
        let workers = num_workers.unwrap_or_else(num_cpus::get);
        Self {
            num_workers: workers.max(1),
            python_path,
            timeout_config: TimeoutConfig::default(),
        }
    }

    /// Set the timeout configuration.
    pub fn with_timeout(mut self, config: TimeoutConfig) -> Self {
        self.timeout_config = config;
        self
    }

    /// Returns the number of workers configured for this pool.
    pub fn num_workers(&self) -> usize {
        self.num_workers
    }

    /// Returns the Python path used by this pool.
    pub fn python_path(&self) -> &str {
        &self.python_path
    }

    /// Execute a batch of tests using the subprocess pool.
    ///
    /// Tests are distributed across workers using a work-stealing injector queue.
    /// Each worker is a persistent Python process running the worker harness.
    pub fn execute(&self, tests: &[TestItem]) -> Vec<TestResult> {
        if tests.is_empty() {
            return Vec::new();
        }

        // Write the harness to a temp file
        let harness_file = match write_harness_to_temp() {
            Ok(f) => f,
            Err(e) => {
                return tests
                    .iter()
                    .map(|t| TestResult {
                        test_id: t.id.clone(),
                        outcome: TestOutcome::Error {
                            message: format!("Failed to write worker harness: {e}"),
                        },
                        duration: Duration::ZERO,
                        output: String::new(),
                        error: Some(e.to_string()),
                        stdout: String::new(),
                        stderr: String::new(),
                    })
                    .collect();
            }
        };

        // Set up the work-stealing injector
        let injector = Injector::new();
        for (idx, test) in tests.iter().enumerate() {
            injector.push((idx, test.clone()));
        }

        let actual_workers = self.num_workers.min(tests.len());
        let mut results: Vec<Option<TestResult>> = vec![None; tests.len()];

        // Spawn workers and process tests
        // We use a simple approach: each thread takes from the injector
        let results_lock = parking_lot::Mutex::new(&mut results);

        crossbeam::scope(|scope| {
            for _ in 0..actual_workers {
                let injector = &injector;
                let results_lock = &results_lock;
                let python_path = &self.python_path;
                let harness_path = harness_file.path();
                let timeout = &self.timeout_config;

                scope.spawn(move |_| {
                    // Spawn a persistent worker process
                    let mut worker = match spawn_worker(python_path, &harness_path.to_string_lossy()) {
                        Ok(w) => w,
                        Err(e) => {
                            // Drain remaining work and mark as errors
                            loop {
                                match injector.steal() {
                                    Steal::Success((idx, test)) => {
                                        let result = TestResult {
                                            test_id: test.id.clone(),
                                            outcome: TestOutcome::Error {
                                                message: format!("Worker spawn failed: {e}"),
                                            },
                                            duration: Duration::ZERO,
                                            output: String::new(),
                                            error: Some(e.to_string()),
                                            stdout: String::new(),
                                            stderr: String::new(),
                                        };
                                        let mut guard = results_lock.lock();
                                        guard[idx] = Some(result);
                                    }
                                    Steal::Empty => break,
                                    Steal::Retry => continue,
                                }
                            }
                            return;
                        }
                    };

                    loop {
                        let (idx, test) = match injector.steal() {
                            Steal::Success(item) => item,
                            Steal::Empty => break,
                            Steal::Retry => continue,
                        };

                        let result = execute_on_worker(&mut worker, &test, timeout);

                        let mut guard = results_lock.lock();
                        guard[idx] = Some(result);
                    }

                    // Signal worker to exit
                    if let Some(ref mut stdin) = worker.stdin {
                        let _ = writeln!(stdin, "EXIT");
                    }
                    let _ = worker.wait();
                });
            }
        })
        .expect("Worker threads panicked");

        // Collect results, filling in any gaps with errors
        results
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                r.unwrap_or_else(|| TestResult {
                    test_id: tests[i].id.clone(),
                    outcome: TestOutcome::Error {
                        message: "Test was not executed".into(),
                    },
                    duration: Duration::ZERO,
                    output: String::new(),
                    error: Some("Test was not executed by any worker".into()),
                    stdout: String::new(),
                    stderr: String::new(),
                })
            })
            .collect()
    }
}

/// Write the worker harness to a temporary file and return the handle.
fn write_harness_to_temp() -> Result<tempfile::NamedTempFile, std::io::Error> {
    let mut file = tempfile::Builder::new()
        .prefix("fastest_worker_")
        .suffix(".py")
        .tempfile()?;
    file.write_all(WORKER_HARNESS.as_bytes())?;
    file.flush()?;
    Ok(file)
}

/// Spawn a persistent Python worker process.
fn spawn_worker(python_path: &str, harness_path: &str) -> Result<Child, std::io::Error> {
    Command::new(python_path)
        .arg(harness_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// Send a test to a worker process and read back the result.
fn execute_on_worker(
    worker: &mut Child,
    test: &TestItem,
    timeout: &TimeoutConfig,
) -> TestResult {
    let start = Instant::now();

    let input = WorkerInput::from_test_item(test);
    let json_input = match serde_json::to_string(&input) {
        Ok(j) => j,
        Err(e) => {
            return TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Error {
                    message: format!("Failed to serialize test input: {e}"),
                },
                duration: start.elapsed(),
                output: String::new(),
                error: Some(e.to_string()),
                stdout: String::new(),
                stderr: String::new(),
            };
        }
    };

    // Write test to worker stdin
    if let Some(ref mut stdin) = worker.stdin {
        if let Err(e) = writeln!(stdin, "{}", json_input) {
            return TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Error {
                    message: format!("Failed to write to worker stdin: {e}"),
                },
                duration: start.elapsed(),
                output: String::new(),
                error: Some(e.to_string()),
                stdout: String::new(),
                stderr: String::new(),
            };
        }
        if let Err(e) = stdin.flush() {
            return TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Error {
                    message: format!("Failed to flush worker stdin: {e}"),
                },
                duration: start.elapsed(),
                output: String::new(),
                error: Some(e.to_string()),
                stdout: String::new(),
                stderr: String::new(),
            };
        }
    }

    // Read result from worker stdout
    if let Some(ref mut stdout) = worker.stdout {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    match parse_worker_result(&line) {
                        Ok(wr) => {
                            let mut result = wr.into_test_result(&test.id);
                            // Check timeout
                            if timeout.is_expired(start.elapsed()) {
                                result.outcome = TestOutcome::Error {
                                    message: format!(
                                        "Test exceeded timeout of {:?}",
                                        timeout.per_test
                                    ),
                                };
                            }
                            return result;
                        }
                        Err(e) => {
                            return TestResult {
                                test_id: test.id.clone(),
                                outcome: TestOutcome::Error {
                                    message: format!("Failed to parse worker output: {e}"),
                                },
                                duration: start.elapsed(),
                                output: line,
                                error: Some(e.to_string()),
                                stdout: String::new(),
                                stderr: String::new(),
                            };
                        }
                    }
                }
                Err(e) => {
                    return TestResult {
                        test_id: test.id.clone(),
                        outcome: TestOutcome::Error {
                            message: format!("Failed to read worker output: {e}"),
                        },
                        duration: start.elapsed(),
                        output: String::new(),
                        error: Some(e.to_string()),
                        stdout: String::new(),
                        stderr: String::new(),
                    };
                }
            }
        }
    }

    TestResult {
        test_id: test.id.clone(),
        outcome: TestOutcome::Error {
            message: "No output received from worker".into(),
        },
        duration: start.elapsed(),
        output: String::new(),
        error: Some("Worker process produced no output".into()),
        stdout: String::new(),
        stderr: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_python() {
        // This should succeed in any environment with Python installed
        let python = find_python();
        assert!(
            python.is_some(),
            "Expected to find a Python interpreter on the system"
        );
    }

    #[test]
    fn test_parse_worker_result_passed() {
        let json = r#"{"test_id":"test::a","outcome":"Passed","duration_ms":42,"error":null,"stdout":"hello\n","stderr":""}"#;
        let result = parse_worker_result(json).unwrap();
        assert_eq!(result.test_id.as_deref(), Some("test::a"));
        assert_eq!(result.outcome.as_deref(), Some("Passed"));
        assert_eq!(result.duration_ms, Some(42));
        assert!(result.error.is_none());
        assert_eq!(result.stdout.as_deref(), Some("hello\n"));
    }

    #[test]
    fn test_parse_worker_result_failed() {
        let json = r#"{"test_id":"test::b","outcome":"Failed","duration_ms":10,"error":"AssertionError","stdout":"","stderr":"traceback..."}"#;
        let result = parse_worker_result(json).unwrap();
        assert_eq!(result.outcome.as_deref(), Some("Failed"));
        assert_eq!(result.error.as_deref(), Some("AssertionError"));
        assert_eq!(result.stderr.as_deref(), Some("traceback..."));
    }

    #[test]
    fn test_parse_worker_result_into_test_result() {
        let json = r#"{"test_id":"test::c","outcome":"Passed","duration_ms":5,"error":null,"stdout":"","stderr":""}"#;
        let wr = parse_worker_result(json).unwrap();
        let result = wr.into_test_result("fallback_id");
        assert_eq!(result.test_id, "test::c");
        assert_eq!(result.outcome, TestOutcome::Passed);
        assert_eq!(result.duration, Duration::from_millis(5));
    }

    #[test]
    fn test_parse_worker_result_error_only() {
        let json = r#"{"error":"something broke"}"#;
        let wr = parse_worker_result(json).unwrap();
        let result = wr.into_test_result("fallback");
        assert_eq!(result.test_id, "fallback");
        assert!(matches!(result.outcome, TestOutcome::Error { .. }));
    }

    #[test]
    fn test_worker_input_serialization() {
        let item = TestItem {
            id: "test.py::test_func".into(),
            path: std::path::PathBuf::from("test.py"),
            function_name: "test_func".into(),
            line_number: Some(1),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
            name: "test_func".into(),
        };
        let input = WorkerInput::from_test_item(&item);
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("test_func"));
        assert!(json.contains("test.py"));
    }

    #[test]
    fn test_subprocess_pool_defaults() {
        let pool = SubprocessPool::new(Some(4));
        assert_eq!(pool.num_workers(), 4);
        assert!(!pool.python_path().is_empty());
    }

    #[test]
    fn test_subprocess_pool_minimum_one_worker() {
        let pool = SubprocessPool::new(Some(0));
        assert_eq!(pool.num_workers(), 1);
    }
}
