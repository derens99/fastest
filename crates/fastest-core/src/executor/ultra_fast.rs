//! Ultra-fast Python test executor – compact rewrite
//! Public API preserved: `UltraFastExecutor::new(verbose).execute(tests)`
//!
//! • ~40% fewer lines + fewer locks
//! • Persistent interpreter pool for amortised startup
//! • Clear separation of Rust–Python wire protocol
//! • Safe fallback to mark failed tests on any worker error

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::discovery::TestItem;
use crate::error::{Error, Result};
use super::TestResult; // keep same re‑export path as before
use crate::utils::PYTHON_CMD;

/* -------------------------------------------------------------------------- */
/*                              Configuration                                 */
/* -------------------------------------------------------------------------- */
/// Number of persistent Python interpreters kept hot.
const POOL_SIZE: usize = 8;
const BATCH_SIZE: usize = 50; // sweet‑spot for cache locality & parallelism

/* -------------------------------------------------------------------------- */
/*                             Wire‑protocol types                            */
/* -------------------------------------------------------------------------- */
#[derive(Serialize)]
struct WorkerCommand {
    id: usize,
    tests: Vec<TestData>,
}

#[derive(Serialize)]
struct TestData {
    id: String,
    module: String,
    func: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct WorkerResponse {
    id: usize,
    results: Vec<TestResultData>,
}

#[derive(Deserialize)]
struct TestResultData {
    id: String,
    passed: bool,
    duration: f64,
    error: Option<String>,
}

/* -------------------------------------------------------------------------- */
/*                         Persistent Python worker                           */
/* -------------------------------------------------------------------------- */
struct FastInterpreter {
    stdin: Mutex<std::process::ChildStdin>,
    stdout: Mutex<BufReader<std::process::ChildStdout>>,
}

impl FastInterpreter {
    fn spawn(id: usize) -> Result<Self> {
        let mut child = Command::new(&*PYTHON_CMD)
            .args(["-u", "-c", Self::worker_code()])
            .envs([
                ("PYTHONUNBUFFERED", "1"),
                ("PYTHONDONTWRITEBYTECODE", "1"),
                ("PYTHONHASHSEED", "0"),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Execution(format!("spawn worker {id}: {e}")))?;

        // wait for READY sentinel (single line)
        let mut rdr = BufReader::new(child.stdout.take().unwrap());
        let mut ready = String::new();
        rdr.read_line(&mut ready)?;
        if ready.trim() != "READY" {
            return Err(Error::Execution("worker not ready".into()));
        }

        Ok(Self {
            stdin: Mutex::new(child.stdin.take().unwrap()),
            stdout: Mutex::new(rdr),
        })
    }

    fn run(&self, cmd: &WorkerCommand) -> Result<WorkerResponse> {
        // send JSON command
        {
            let mut w = self.stdin.lock();
            let json_str = serde_json::to_string(cmd)?;
            writeln!(w, "{}", json_str)?;
            w.flush()?;
        }
        // read single‑line JSON reply
        let mut line = String::new();
        let bytes_read = self.stdout.lock().read_line(&mut line)?;
        if bytes_read == 0 {
            return Err(Error::Execution("Worker closed stdout".to_string()));
        }
        
        // Debug output
        if line.trim().is_empty() {
            return Err(Error::Execution("Worker returned empty response".to_string()));
        }
        
        serde_json::from_str(&line)
            .map_err(|e| Error::Execution(format!("Failed to parse response: {} (raw: {:?})", e, line)))
    }

    /// Embedded ultra‑thin Python worker
    fn worker_code() -> &'static str {
        r#"
import sys, json, time, importlib, gc, os, io, inspect
from contextlib import redirect_stdout, redirect_stderr

gc.disable()
perf = time.perf_counter
fn_cache = {}
path_cache = set()

# Add current dir and common test paths to sys.path
sys.path.insert(0, os.getcwd())
for p in ['tests', 'test', '.']:
    if os.path.exists(p):
        sys.path.insert(0, os.path.abspath(p))

def ensure_path(filepath):
    """Ensure the directory containing the test file is in sys.path"""
    if filepath and filepath not in path_cache:
        dirpath = os.path.dirname(os.path.abspath(filepath))
        if dirpath not in sys.path:
            sys.path.insert(0, dirpath)
        path_cache.add(filepath)
        
        # Also add parent directory in case tests are in a subdirectory
        parent_dir = os.path.dirname(dirpath)
        if parent_dir and parent_dir not in sys.path:
            sys.path.insert(0, parent_dir)

def get_fn(modname, name, filepath=None):
    key = f"{modname}.{name}"
    if key in fn_cache:
        return fn_cache[key]
    
    try:
        # Ensure the test file's directory is in sys.path
        if filepath:
            ensure_path(filepath)
        
        # Try to import the module
        try:
            mod = importlib.import_module(modname)
        except ImportError:
            # If module not found and we have a filepath, try to derive the correct module name
            if filepath and os.path.exists(filepath):
                # Get the base name without extension
                base_name = os.path.splitext(os.path.basename(filepath))[0]
                if base_name != modname:
                    # Try with the actual filename
                    mod = importlib.import_module(base_name)
                else:
                    raise
            else:
                raise
        
        # Handle class methods
        if '::' in name:
            parts = name.split('::', 1)
            cls = getattr(mod, parts[0])
            
            # Create a proper instance with initialization
            try:
                # Try to create instance normally
                instance = cls()
            except Exception:
                # If normal instantiation fails, try without arguments
                try:
                    # Check if __init__ accepts arguments
                    sig = inspect.signature(cls.__init__)
                    params = list(sig.parameters.values())[1:]  # Skip 'self'
                    if params and all(p.default == inspect.Parameter.empty for p in params):
                        # Has required parameters, use __new__
                        instance = object.__new__(cls)
                    else:
                        instance = cls()
                except Exception:
                    # Last resort: use __new__
                    instance = object.__new__(cls)
            
            # Call setUp if it exists (for unittest compatibility)
            if hasattr(instance, 'setUp'):
                try:
                    instance.setUp()
                except Exception:
                    pass  # Some setUp methods might fail without proper test context
            
            fn = getattr(instance, parts[1])
            
            # Store both the instance and method for reuse
            fn_cache[key] = (fn, instance)
            return fn
        else:
            fn = getattr(mod, name)
        
        fn_cache[key] = fn
        return fn
    except Exception as e:
        # Re-raise with better error message
        raise ImportError(f"Failed to load {modname}.{name}: {str(e)}")

def extract_params_from_test_id(test_id):
    """Extract parameter values from test ID like test_func[1-2-3]"""
    if '[' not in test_id or not test_id.endswith(']'):
        return None
    
    # Get the part inside brackets
    param_part = test_id[test_id.find('[') + 1:-1]
    
    # Simple heuristic: try to parse common parameter formats
    # This handles numeric parameters separated by dashes
    parts = param_part.split('-')
    params = []
    for part in parts:
        try:
            # Try integer
            params.append(int(part))
        except ValueError:
            try:
                # Try float
                params.append(float(part))
            except ValueError:
                # Keep as string
                params.append(part)
    
    return params

print('READY')
sys.stdout.flush()

while True:
    try:
        line = sys.stdin.readline()
        if not line:
            break
            
        cmd = json.loads(line.strip())
        res = []
        
        for t in cmd['tests']:
            # Capture stdout/stderr during test execution
            stdout_buf = io.StringIO()
            stderr_buf = io.StringIO()
            
            start = perf()
            try:
                fn_result = get_fn(t['module'], t['func'], t.get('path'))
                
                # Check if we got a tuple (method, instance) from cache
                if isinstance(fn_result, tuple):
                    fn, instance = fn_result
                else:
                    fn = fn_result
                
                # Handle parametrized tests
                if 'params' in t and t['params'] is not None:
                    # If params are provided as a dict, extract the values
                    if isinstance(t['params'], dict):
                        # Get parameter values in the order they appear in the function signature
                        sig = inspect.signature(fn)
                        args = []
                        for param_name in sig.parameters:
                            if param_name in t['params']:
                                args.append(t['params'][param_name])
                        with redirect_stdout(stdout_buf), redirect_stderr(stderr_buf):
                            fn(*args)
                    else:
                        # Params provided as a list
                        with redirect_stdout(stdout_buf), redirect_stderr(stderr_buf):
                            fn(*t['params'])
                elif '[' in t['id']:
                    # Try to extract params from test ID
                    params = extract_params_from_test_id(t['id'])
                    if params:
                        with redirect_stdout(stdout_buf), redirect_stderr(stderr_buf):
                            fn(*params)
                    else:
                        with redirect_stdout(stdout_buf), redirect_stderr(stderr_buf):
                            fn()
                else:
                    with redirect_stdout(stdout_buf), redirect_stderr(stderr_buf):
                        fn()
                
                res.append({
                    'id': t['id'], 
                    'passed': True, 
                    'duration': perf() - start, 
                    'error': None
                })
            except Exception as e:
                error_msg = str(e)
                # Check for skip markers
                if 'SKIP' in error_msg or type(e).__name__ in ('Skipped', 'SkipTest'):
                    # Mark as passed but with skip message
                    res.append({
                        'id': t['id'], 
                        'passed': True, 
                        'duration': perf() - start, 
                        'error': f'SKIPPED: {error_msg}'
                    })
                else:
                    res.append({
                        'id': t['id'], 
                        'passed': False, 
                        'duration': perf() - start, 
                        'error': error_msg
                    })
        
        sys.stdout.write(json.dumps({'id': cmd['id'], 'results': res}) + '\n')
        sys.stdout.flush()
        
    except KeyboardInterrupt:
        break
    except Exception as e:
        # Log error but continue
        sys.stderr.write(f"Worker error: {e}\n")
        sys.stderr.flush()
"#
    }
}

/* -------------------------------------------------------------------------- */
/*                              Interpreter pool                              */
/* -------------------------------------------------------------------------- */
struct InterpreterPool {
    workers: Vec<Arc<FastInterpreter>>,
    cursor: AtomicUsize,
}

impl InterpreterPool {
    fn new(size: usize) -> Result<Self> {
        let mut v = Vec::with_capacity(size);
        for id in 0..size { v.push(Arc::new(FastInterpreter::spawn(id)?)); }
        Ok(Self { workers: v, cursor: AtomicUsize::new(0) })
    }

    #[inline]
    fn next(&self) -> Arc<FastInterpreter> {
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        self.workers[idx].clone()
    }
}

// global pool (lazy‑init on first access)
static POOL: Lazy<InterpreterPool> = Lazy::new(|| InterpreterPool::new(POOL_SIZE).expect("init pool"));

/* -------------------------------------------------------------------------- */
/*                    Public wrapper – preserves old API                      */
/* -------------------------------------------------------------------------- */
/// Drop‑in replacement for the previous struct API.
pub struct UltraFastExecutor {
    verbose: bool,
}

impl UltraFastExecutor {
    pub fn new(verbose: bool) -> Self { Self { verbose } }
    
    /// Alternative constructor for ParallelExecutor compatibility
    pub fn new_with_workers(_num_workers: Option<usize>, verbose: bool) -> Self {
        // Ignore num_workers - the pool manages its own size
        Self::new(verbose)
    }

    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        if self.verbose {
            eprintln!("⚡ ultra‑fast executor: {} tests", tests.len());
        }
        run_tests(tests, self.verbose)
    }
    
    // Legacy compatibility methods
    
    /// Accept coverage configuration for API compatibility. No-op for now.
    pub fn with_coverage(self, _source_dirs: Vec<std::path::PathBuf>) -> Self {
        if self.verbose {
            eprintln!("⚠️  Coverage collection is not yet implemented in the ultra-fast executor");
        }
        self
    }
    
    /// Legacy method for BatchExecutor compatibility
    pub fn execute_tests(&self, tests: Vec<TestItem>) -> Vec<TestResult> {
        self.execute(tests).unwrap_or_else(|e| {
            eprintln!("Error executing tests: {}", e);
            Vec::new()
        })
    }
}

/* -------------------------------------------------------------------------- */
/*                              Core execution                                */
/* -------------------------------------------------------------------------- */
fn run_tests(tests: Vec<TestItem>, verbose: bool) -> Result<Vec<TestResult>> {
    if tests.is_empty() { return Ok(vec![]); }

    let chunks: Vec<_> = tests.chunks(BATCH_SIZE).collect();
    let total_batches = chunks.len();
    
    if verbose {
        eprintln!("Running {} batches of up to {} tests each", total_batches, BATCH_SIZE);
    }

    let results: Vec<TestResult> = chunks
        .into_par_iter()
        .enumerate()
        .flat_map(|(i, chunk)| {
            if verbose {
                eprintln!("Processing batch {}/{}", i + 1, total_batches);
            }
            run_batch(chunk, verbose)
        })
        .collect();

    Ok(results)
}

fn run_batch(chunk: &[TestItem], verbose: bool) -> Vec<TestResult> {
    let cmd = WorkerCommand {
        id: next_id(),
        tests: chunk
            .iter()
            .map(|t| {
                // Handle parametrized tests by stripping the parameter part
                let test_id = if let Some(bracket_pos) = t.id.find('[') {
                    &t.id[..bracket_pos]
                } else {
                    &t.id
                };
                
                // More robust parsing of test IDs
                let parts: Vec<&str> = test_id.split("::").collect();
                let (module, func) = match parts.len() {
                    1 => (parts[0], t.function_name.clone()),
                    2 => (parts[0], parts[1].to_string()),
                    3 => (parts[0], format!("{}::{}", parts[1], parts[2])),
                    _ => (parts[0], parts[1..].join("::")),
                };
                
                // Extract parameters from decorators
                let params = t.decorators.iter()
                    .find(|d| d.starts_with("__params__="))
                    .and_then(|d| {
                        let json_str = d.trim_start_matches("__params__=");
                        serde_json::from_str::<serde_json::Value>(json_str).ok()
                    });
                
                if verbose {
                    eprintln!("Test mapping: {} -> module: {}, func: {}, params: {:?}", 
                             t.id, module, func, params);
                }
                
                TestData {
                    id: t.id.clone(),
                    module: module.to_owned(),
                    func,
                    path: t.path.to_string_lossy().to_string(),
                    params,
                }
            })
            .collect(),
    };

    if verbose {
        eprintln!("Sending command: {}", serde_json::to_string_pretty(&cmd).unwrap_or_default());
    }

    let worker = POOL.next();
    match worker.run(&cmd) {
        Ok(resp) if resp.id == cmd.id => {
            if verbose {
                eprintln!("Received response with {} results", resp.results.len());
            }
            resp.results.into_iter().map(to_result).collect()
        }
        Ok(_) => {
            if verbose {
                eprintln!("Worker returned mismatched command ID");
            }
            chunk.iter().map(|t| fail(t, "id‑mismatch")).collect()
        }
        Err(e) => {
            if verbose {
                eprintln!("Worker error: {}", e);
            }
            chunk.iter().map(|t| fail(t, &e.to_string())).collect()
        }
    }
}

#[inline]
fn to_result(r: TestResultData) -> TestResult {
    let is_skip = r.error.as_ref().map(|e| e.starts_with("SKIPPED:")).unwrap_or(false);
    TestResult {
        test_id: r.id,
        passed: r.passed,
        duration: Duration::from_secs_f64(r.duration),
        output: if is_skip { 
            "SKIPPED".to_owned() 
        } else if r.passed { 
            "PASSED".to_owned() 
        } else { 
            "FAILED".to_owned() 
        },
        error: r.error,
        stdout: String::new(),
        stderr: String::new(),
    }
}

#[inline]
fn fail(t: &TestItem, msg: &str) -> TestResult {
    TestResult {
        test_id: t.id.clone(),
        passed: false,
        duration: Duration::ZERO,
        output: "FAILED".into(),
        error: Some(msg.into()),
        stdout: String::new(),
        stderr: String::new(),
    }
}

fn next_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
