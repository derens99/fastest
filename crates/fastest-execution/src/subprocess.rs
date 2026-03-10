//! Subprocess pool executor with work-stealing.
//!
//! Spawns N persistent Python worker processes that communicate via
//! JSON over stdin/stdout. Uses `crossbeam_deque::Injector` for
//! fair work distribution across workers.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_deque::{Injector, Steal};
use fastest_core::markers::{classify_marker, BuiltinMarker};
use fastest_core::model::{TestItem, TestOutcome, TestResult};
use serde::{Deserialize, Serialize};

use crate::capture::CapturedOutput;
use crate::timeout::TimeoutConfig;

/// The embedded Python worker harness script.
const WORKER_HARNESS: &str = include_str!("worker_harness.py");

/// Protocol prefix: lines from the worker harness that start with this are JSON results.
/// Other stdout lines are captured test output.
const RESULT_PREFIX: &str = "FASTEST_RESULT:";

/// Maximum number of times a worker can be restarted after crashing.
const MAX_WORKER_RESTARTS: usize = 3;

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
    /// Whether the user explicitly specified the worker count (via -j flag).
    user_specified_workers: bool,
    python_path: String,
    timeout_config: TimeoutConfig,
    /// Names of session-scoped fixtures. Tests depending on these are routed to worker #0.
    session_fixture_names: HashSet<String>,
}

impl SubprocessPool {
    /// Create a new subprocess pool.
    ///
    /// `num_workers` defaults to the number of CPUs if `None`.
    pub fn new(num_workers: Option<usize>) -> Self {
        let user_specified = num_workers.is_some();
        let workers = num_workers.unwrap_or_else(num_cpus::get);
        let python = find_python().unwrap_or_else(|| "python3".into());
        Self {
            num_workers: workers.max(1),
            user_specified_workers: user_specified,
            python_path: python,
            timeout_config: TimeoutConfig::default(),
            session_fixture_names: HashSet::new(),
        }
    }

    /// Create a subprocess pool with a specific Python path.
    pub fn with_python(num_workers: Option<usize>, python_path: String) -> Self {
        let user_specified = num_workers.is_some();
        let workers = num_workers.unwrap_or_else(num_cpus::get);
        Self {
            num_workers: workers.max(1),
            user_specified_workers: user_specified,
            python_path,
            timeout_config: TimeoutConfig::default(),
            session_fixture_names: HashSet::new(),
        }
    }

    /// Set the timeout configuration.
    pub fn with_timeout(mut self, config: TimeoutConfig) -> Self {
        self.timeout_config = config;
        self
    }

    /// Set the session-scoped fixture names for test partitioning.
    ///
    /// Tests that depend on any of these fixtures will be routed to worker #0
    /// to ensure session fixtures are created once and shared.
    pub fn with_session_fixtures(mut self, names: HashSet<String>) -> Self {
        self.session_fixture_names = names;
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
    /// Tests are sorted by (path, class_name) to improve setup/teardown lifecycle.
    pub fn execute(&self, tests: &[TestItem]) -> Vec<TestResult> {
        self.execute_with_callback(tests, &|_| {})
    }

    /// Execute tests with a callback invoked after each test completes.
    ///
    /// The callback is called from worker threads as each result arrives,
    /// enabling live progress bars and streaming output.
    pub fn execute_with_callback(
        &self,
        tests: &[TestItem],
        on_result: &(dyn Fn(&TestResult) + Send + Sync),
    ) -> Vec<TestResult> {
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

        // Sort tests by (path, class_name) for better setup/teardown grouping.
        // This ensures that tests from the same class are processed consecutively
        // by the same worker (when possible), enabling correct teardown_class timing.
        let mut indexed_tests: Vec<(usize, &TestItem)> = tests.iter().enumerate().collect();
        indexed_tests.sort_by(|a, b| {
            let key_a = (&a.1.path, &a.1.class_name);
            let key_b = (&b.1.path, &b.1.class_name);
            key_a.cmp(&key_b)
        });

        // Partition tests: session-fixture-dependent tests go to worker #0's dedicated queue,
        // everything else goes to the shared work-stealing injector.
        let session_injector = Injector::new();
        let injector = Injector::new();
        let mut session_test_count = 0usize;

        if self.session_fixture_names.is_empty() {
            // No session fixtures — all tests go to work-stealing
            for (idx, test) in indexed_tests {
                injector.push((idx, test.clone()));
            }
        } else {
            for (idx, test) in indexed_tests {
                let is_session_bound = test
                    .fixture_deps
                    .iter()
                    .any(|dep| self.session_fixture_names.contains(dep));
                if is_session_bound {
                    session_injector.push((idx, test.clone()));
                    session_test_count += 1;
                } else {
                    injector.push((idx, test.clone()));
                }
            }
        }

        // If there are session-bound tests, we need at least one dedicated worker for them.
        // The remaining workers handle parallel tests from the shared injector.
        let has_session_tests = session_test_count > 0;
        let total_parallel_tests = tests.len() - session_test_count;
        // Auto-tune worker count when not user-specified to avoid excessive spawn overhead.
        // Each worker needs ~50 tests to amortize Python interpreter startup cost.
        let effective_workers = if !self.user_specified_workers {
            let max_by_count = (tests.len() / 50).max(1);
            self.num_workers.min(max_by_count)
        } else {
            self.num_workers
        };
        let actual_workers = if has_session_tests {
            // Worker #0 is dedicated to session tests; remaining workers handle parallel tests.
            let parallel_workers = if total_parallel_tests > 0 {
                (effective_workers - 1).max(1).min(total_parallel_tests)
            } else {
                0
            };
            1 + parallel_workers
        } else {
            effective_workers.min(tests.len())
        };
        let mut results: Vec<Option<TestResult>> = vec![None; tests.len()];

        // Spawn workers and process tests
        let results_lock = parking_lot::Mutex::new(&mut results);

        crossbeam::scope(|scope| {
            for worker_id in 0..actual_workers {
                let injector = &injector;
                let session_injector = &session_injector;
                let results_lock = &results_lock;
                let python_path = &self.python_path;
                let harness_path = &harness_path;
                let timeout = &self.timeout_config;
                let is_session_worker = has_session_tests && worker_id == 0;

                scope.spawn(move |_| {
                    let harness_str = harness_path.to_string_lossy();
                    let mut worker = match PersistentWorker::spawn(python_path, &harness_str) {
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

                    let mut restart_count = 0;
                    let mut retried_tests: std::collections::HashSet<usize> = std::collections::HashSet::new();
                    // Worker #0 drains session-bound tests first, then helps with parallel tests.
                    // Other workers only process from the shared injector.
                    let mut session_phase = is_session_worker;
                    loop {
                        let steal_result = if session_phase {
                            match session_injector.steal() {
                                Steal::Success(item) => Steal::Success(item),
                                Steal::Empty => {
                                    // Session queue drained — switch to parallel queue
                                    session_phase = false;
                                    injector.steal()
                                }
                                Steal::Retry => Steal::Retry,
                            }
                        } else {
                            injector.steal()
                        };
                        let (idx, test) = match steal_result {
                            Steal::Success(item) => item,
                            Steal::Empty => break,
                            Steal::Retry => continue,
                        };

                        let result = execute_on_worker(&mut worker, &test, timeout);
                        let worker_died = !worker.is_alive();

                        if worker_died {
                            // Distinguish timeout kill (keep result) from crash (re-enqueue)
                            let was_timeout = matches!(
                                &result.outcome,
                                TestOutcome::Error { message } if message.contains("timeout")
                            );
                            if was_timeout {
                                on_result(&result);
                                let mut guard = results_lock.lock();
                                guard[idx] = Some(result);
                            } else if retried_tests.contains(&idx) {
                                // Already retried this test — mark as error, don't re-enqueue
                                let crash_result = TestResult {
                                    test_id: test.id.clone(),
                                    outcome: TestOutcome::Error {
                                        message: "Test caused worker crash on retry".into(),
                                    },
                                    duration: result.duration,
                                    output: String::new(),
                                    error: Some("Worker process crashed while running this test".into()),
                                    stdout: result.stdout,
                                    stderr: result.stderr,
                                };
                                on_result(&crash_result);
                                let mut guard = results_lock.lock();
                                guard[idx] = Some(crash_result);
                            } else {
                                // First crash for this test — re-enqueue for retry
                                retried_tests.insert(idx);
                                injector.push((idx, test));
                            }
                        } else {
                            on_result(&result);
                            let mut guard = results_lock.lock();
                            guard[idx] = Some(result);
                        }

                        // If the worker died (crash or timeout kill), try to restart it
                        if worker_died {
                            restart_count += 1;
                            if restart_count > MAX_WORKER_RESTARTS {
                                // Too many restarts for this worker — stop it but let
                                // other workers continue processing remaining tests.
                                break;
                            }
                            match PersistentWorker::spawn(python_path, &harness_str) {
                                Ok(w) => worker = w,
                                Err(e) => {
                                    // Can't restart — drain remaining
                                    loop {
                                        match injector.steal() {
                                            Steal::Success((idx, test)) => {
                                                let result = TestResult {
                                                    test_id: test.id.clone(),
                                                    outcome: TestOutcome::Error {
                                                        message: format!(
                                                            "Worker restart failed: {e}"
                                                        ),
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
                            }
                        }
                    }

                    // Signal worker to exit gracefully
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

/// A persistent Python worker process with channel-based stdout reading
/// and background stderr capture.
struct PersistentWorker {
    child: Child,
    /// Receives lines from the stdout reader thread.
    result_rx: crossbeam::channel::Receiver<String>,
    /// Shared buffer holding the last 8KB of stderr output.
    stderr_buffer: Arc<parking_lot::Mutex<String>>,
    /// Set to false when the stdout reader thread detects EOF (worker exited).
    alive: Arc<AtomicBool>,
}

impl PersistentWorker {
    /// Spawn a new persistent worker with background stdout/stderr reader threads.
    fn spawn(python_path: &str, harness_path: &str) -> Result<Self, std::io::Error> {
        let mut child = Command::new(python_path)
            .arg(harness_path)
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        // Channel for stdout lines
        let (tx, rx) = crossbeam::channel::unbounded();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_clone = alive.clone();

        // Background thread: read stdout lines and send over channel
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break, // EOF — worker exited
                    Ok(_) => {
                        if tx.send(line.trim_end().to_string()).is_err() {
                            break; // receiver dropped
                        }
                    }
                    Err(_) => break,
                }
            }
            alive_clone.store(false, Ordering::SeqCst);
        });

        // Background thread: drain stderr into a bounded buffer
        let stderr_buffer = Arc::new(parking_lot::Mutex::new(String::new()));
        let buf_clone = stderr_buffer.clone();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let mut buf = buf_clone.lock();
                        buf.push_str(&line);
                        // Keep only the last 8KB of stderr
                        if buf.len() > 8192 {
                            let drain_to = buf.len() - 4096;
                            buf.drain(..drain_to);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            child,
            result_rx: rx,
            stderr_buffer,
            alive,
        })
    }

    /// Returns true if the worker process is still running.
    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// Returns the last captured stderr output (up to ~8KB).
    fn last_stderr(&self) -> String {
        self.stderr_buffer.lock().clone()
    }

    /// Forcefully kill the worker process.
    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.alive.store(false, Ordering::SeqCst);
    }

    /// Signal the worker to exit gracefully and wait for process to finish.
    fn shutdown(&mut self) {
        if let Some(ref mut stdin) = self.child.stdin {
            let _ = writeln!(stdin, "EXIT");
        }
        let _ = self.child.wait();
    }
}

/// Send a test to a worker process and read back the result.
///
/// Uses channel-based reading with `recv_timeout` for proper timeout enforcement.
/// Non-result stdout lines (from `print()` in tests) are captured separately.
/// On timeout, the worker process is killed.
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

    // Read result from channel with timeout.
    // Non-result lines (test stdout) are captured separately.
    let mut captured_stdout = String::new();
    loop {
        let remaining = timeout.per_test.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            // Timeout expired — kill the worker
            let stderr = worker.last_stderr();
            worker.kill();
            return TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Error {
                    message: format!("Test exceeded timeout of {:?}", timeout.per_test),
                },
                duration: start.elapsed(),
                output: String::new(),
                error: Some(format!("Timeout after {:?}", timeout.per_test)),
                stdout: captured_stdout,
                stderr,
            };
        }

        match worker.result_rx.recv_timeout(remaining) {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }
                if let Some(json_str) = line.strip_prefix(RESULT_PREFIX) {
                    // This is a result line from the worker harness
                    match parse_worker_result(json_str) {
                        Ok(wr) => {
                            let mut result = wr.into_test_result(&test.id);
                            // Merge captured stdout from non-result lines
                            if !captured_stdout.is_empty() {
                                if result.stdout.is_empty() {
                                    result.stdout = captured_stdout;
                                } else {
                                    result.stdout =
                                        format!("{}\n{}", captured_stdout, result.stdout);
                                }
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
                                stdout: captured_stdout,
                                stderr: String::new(),
                            };
                        }
                    }
                } else {
                    // Non-result line — capture as test stdout
                    if !captured_stdout.is_empty() {
                        captured_stdout.push('\n');
                    }
                    captured_stdout.push_str(&line);
                }
            }
            Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                // Timeout — kill the worker
                let stderr = worker.last_stderr();
                worker.kill();
                return TestResult {
                    test_id: test.id.clone(),
                    outcome: TestOutcome::Error {
                        message: format!("Test exceeded timeout of {:?}", timeout.per_test),
                    },
                    duration: start.elapsed(),
                    output: String::new(),
                    error: Some(format!("Timeout after {:?}", timeout.per_test)),
                    stdout: captured_stdout,
                    stderr,
                };
            }
            Err(crossbeam::channel::RecvTimeoutError::Disconnected) => {
                // Worker died — collect error info
                let stderr = worker.last_stderr();
                let exit_info = worker.child.try_wait().ok().flatten();
                let msg = if let Some(status) = exit_info {
                    format!("Worker process exited with {status}")
                } else {
                    "Worker process died unexpectedly".to_string()
                };
                return TestResult {
                    test_id: test.id.clone(),
                    outcome: TestOutcome::Error { message: msg },
                    duration: start.elapsed(),
                    output: String::new(),
                    error: if stderr.is_empty() {
                        Some("Worker process died".into())
                    } else {
                        Some(stderr.clone())
                    },
                    stdout: captured_stdout,
                    stderr,
                };
            }
        }
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
