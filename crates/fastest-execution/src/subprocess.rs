//! Subprocess pool executor with work-stealing.
//!
//! Spawns N persistent Python worker processes that communicate via
//! JSON over stdin/stdout. Uses `crossbeam_deque::Injector` for
//! fair work distribution across workers.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use crossbeam_deque::{Injector, Steal};
use fastest_core::markers::{classify_marker, BuiltinMarker};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<WorkerParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    markers: Option<Vec<WorkerMarker>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixture_deps: Option<Vec<String>>,
}

/// A marker serialized for the worker process.
#[derive(Debug, Serialize)]
struct WorkerMarker {
    name: String,
    args: Vec<serde_json::Value>,
    kwargs: std::collections::HashMap<String, serde_json::Value>,
}

/// Parametrize values sent to worker processes.
#[derive(Debug, Serialize)]
struct WorkerParameters {
    names: Vec<String>,
    values: std::collections::HashMap<String, serde_json::Value>,
}

impl WorkerInput {
    fn from_test_item(item: &TestItem) -> Self {
        let parameters = item.parameters.as_ref().map(|p| WorkerParameters {
            names: p.names.clone(),
            values: p.values.clone(),
        });
        let fixture_deps = if item.fixture_deps.is_empty() {
            None
        } else {
            Some(item.fixture_deps.clone())
        };
        let markers = if item.markers.is_empty() {
            None
        } else {
            Some(
                item.markers
                    .iter()
                    .map(|m| WorkerMarker {
                        name: m.name.clone(),
                        args: m.args.clone(),
                        kwargs: m.kwargs.clone(),
                    })
                    .collect(),
            )
        };
        Self {
            id: item.id.clone(),
            path: item.path.to_string_lossy().to_string(),
            function_name: item.function_name.clone(),
            class_name: item.class_name.clone(),
            parameters,
            markers,
            fixture_deps,
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
    pub reason: Option<String>,
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

        // Extract error message before consuming self.error in the outcome match.
        let error_msg = self
            .error
            .as_deref()
            .unwrap_or("Unknown worker error")
            .to_string();

        let outcome = match self.outcome.as_deref() {
            Some("Passed") => TestOutcome::Passed,
            Some("Failed") => TestOutcome::Failed,
            Some("Skipped") => TestOutcome::Skipped {
                reason: self.reason,
            },
            Some("XFailed") => TestOutcome::XFailed {
                reason: self.reason,
            },
            Some("XPassed") => TestOutcome::XPassed,
            _ => TestOutcome::Error { message: error_msg },
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
///
/// Checks (in order): `PYO3_PYTHON` env var, active virtual environment,
/// then common executable names via `PATH`.  On Windows, candidates found
/// via PATH are validated by running `--version` to skip the Windows Store
/// app execution aliases which are stubs, not real interpreters.
pub fn find_python() -> Option<String> {
    // Check PYO3_PYTHON env var first (explicit override)
    if let Ok(python) = std::env::var("PYO3_PYTHON") {
        return Some(python);
    }

    // Prefer the active virtual environment's Python
    for env_var in &["VIRTUAL_ENV", "CONDA_PREFIX"] {
        if let Ok(venv) = std::env::var(env_var) {
            let venv_path = std::path::PathBuf::from(&venv);
            // Unix: bin/python, Windows: Scripts/python.exe
            let candidates = if cfg!(windows) {
                vec![venv_path.join("Scripts").join("python.exe")]
            } else {
                vec![
                    venv_path.join("bin").join("python3"),
                    venv_path.join("bin").join("python"),
                ]
            };
            for candidate in candidates {
                if candidate.is_file() {
                    return Some(candidate.to_string_lossy().into_owned());
                }
            }
        }
    }

    // Try common Python executable names.
    // Use which_all to iterate over ALL matching paths, not just the first,
    // because the first match may be a Windows Store stub.
    let candidates = [
        "python3",
        "python",
        "python3.13",
        "python3.12",
        "python3.11",
    ];
    for candidate in &candidates {
        if let Ok(paths) = which::which_all(candidate) {
            for path in paths {
                let path_str = path.to_string_lossy().to_string();
                if cfg!(windows) {
                    // Validate the candidate actually works — the Windows Store
                    // "app execution aliases" appear on PATH but are stubs that
                    // print an error instead of running Python.
                    if let Ok(output) = std::process::Command::new(&path_str)
                        .arg("--version")
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .output()
                    {
                        if output.status.success() {
                            return Some(path_str);
                        }
                    }
                } else {
                    return Some(path_str);
                }
            }
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

        // Write the harness to a temp file (persisted so subprocesses can open it)
        let harness_path = match write_harness_to_temp() {
            Ok(p) => p,
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
                let harness_path = &harness_path;
                let timeout = &self.timeout_config;

                scope.spawn(move |_| {
                    // Spawn a persistent worker process
                    let mut worker =
                        match PersistentWorker::spawn(python_path, &harness_path.to_string_lossy())
                        {
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
                    worker.shutdown();
                });
            }
        })
        .expect("Worker threads panicked");

        // Clean up the persisted harness file
        let _ = std::fs::remove_file(&harness_path);

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

/// Write the worker harness to a temporary file and return the persisted path.
///
/// On Windows, `NamedTempFile` uses `FILE_FLAG_DELETE_ON_CLOSE` which prevents
/// other processes from opening the file. We persist it so Python subprocesses
/// can read it, and callers must delete the file when done.
fn write_harness_to_temp() -> Result<std::path::PathBuf, std::io::Error> {
    let mut file = tempfile::Builder::new()
        .prefix("fastest_worker_")
        .suffix(".py")
        .tempfile()?;
    file.write_all(WORKER_HARNESS.as_bytes())?;
    file.flush()?;
    let (_, path) = file.keep().map_err(|e| e.error)?;
    Ok(path)
}

/// A persistent Python worker process with a buffered stdout reader.
///
/// Wrapping the `BufReader` alongside the `Child` ensures the internal read
/// buffer is preserved across calls, preventing data loss when the reader
/// buffers ahead past one result line.
struct PersistentWorker {
    child: Child,
    reader: BufReader<ChildStdout>,
}

impl PersistentWorker {
    /// Spawn a new persistent worker.  Stderr is discarded to avoid deadlocks
    /// when the worker writes enough to fill the OS pipe buffer.
    fn spawn(python_path: &str, harness_path: &str) -> Result<Self, std::io::Error> {
        let mut child = Command::new(python_path)
            .arg(harness_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // prevent deadlock from full stderr pipe
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let reader = BufReader::new(stdout);
        Ok(Self { child, reader })
    }

    /// Signal the worker to exit and wait for process to finish.
    fn shutdown(&mut self) {
        if let Some(ref mut stdin) = self.child.stdin {
            let _ = writeln!(stdin, "EXIT");
        }
        let _ = self.child.wait();
    }
}

/// Send a test to a worker process and read back the result.
fn execute_on_worker(
    worker: &mut PersistentWorker,
    test: &TestItem,
    timeout: &TimeoutConfig,
) -> TestResult {
    // Pre-check skip markers in Rust (avoid worker overhead)
    for marker in &test.markers {
        if let BuiltinMarker::Skip { reason } = classify_marker(marker) {
            return TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Skipped { reason },
                duration: Duration::ZERO,
                output: String::new(),
                error: None,
                stdout: String::new(),
                stderr: String::new(),
            };
        }
    }

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
    let stdin = match worker.child.stdin.as_mut() {
        Some(s) => s,
        None => {
            return TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Error {
                    message: "Worker stdin unavailable (pipe closed)".into(),
                },
                duration: start.elapsed(),
                output: String::new(),
                error: Some("Worker stdin pipe is closed".into()),
                stdout: String::new(),
                stderr: String::new(),
            };
        }
    };
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

    // Read exactly one result line from the persistent BufReader.
    // The BufReader is stored alongside the worker so its internal buffer
    // is preserved across calls — no data is lost between invocations.
    loop {
        let mut line = String::new();
        match worker.reader.read_line(&mut line) {
            Ok(0) => {
                // EOF — worker process has exited
                break;
            }
            Ok(_) => {
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
                                message: format!("Test exceeded timeout of {:?}", timeout.per_test),
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
