use super::TestResult;
use crate::discovery::TestItem;
use crate::error::{Error, Result};
use crate::markers::{extract_markers, BuiltinMarker};
use crate::utils::PYTHON_CMD;
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

// Global thread pool for reuse across executions
static GLOBAL_THREAD_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();

/// Ultra-optimized test executor using advanced techniques
pub struct OptimizedExecutor {
    num_workers: usize,
    batch_size: usize,
    verbose: bool,
    coverage_enabled: bool,
    coverage_source: Vec<PathBuf>,
    fast_mode: bool,
}

impl OptimizedExecutor {
    pub fn new(num_workers: Option<usize>, verbose: bool) -> Self {
        let num_workers = num_workers.unwrap_or_else(|| {
            // Use 2x CPU cores for I/O bound tests
            num_cpus::get()
        });

        Self {
            num_workers,
            batch_size: 50, // Optimal batch size based on benchmarks
            verbose,
            coverage_enabled: false,
            coverage_source: Vec::new(),
            fast_mode: false,
        }
    }

    /// Enable coverage collection
    pub fn with_coverage(mut self, source_dirs: Vec<PathBuf>) -> Self {
        self.coverage_enabled = true;
        self.coverage_source = source_dirs;
        self
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
            eprintln!(
                "⚡ Executing {} tests in {} batches with {} workers",
                test_batches.iter().map(|b| b.len()).sum::<usize>(),
                test_batches.len(),
                self.num_workers
            );
        }

        // Step 3: Execute in parallel with work stealing
        let results = Arc::new(DashMap::new());
        let results_clone = results.clone();

        // Use global thread pool or create if not exists
        let pool = GLOBAL_THREAD_POOL.get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(self.num_workers)
                .thread_name(|idx| format!("fastest-worker-{}", idx))
                .build()
                .expect("Failed to create global thread pool")
        });

        pool.install(|| {
            test_batches.into_par_iter().for_each(|batch| {
                match self.execute_batch_optimized(batch.clone()) {
                    Ok(batch_results) => {
                        for result in batch_results {
                            results_clone.insert(result.test_id.clone(), result);
                        }
                    }
                    Err(e) => {
                        // If batch execution fails, mark all tests in the batch as failed
                        if self.verbose {
                            eprintln!("Batch execution failed: {}", e);
                        }
                        for test in batch {
                            let result = TestResult {
                                test_id: test.id.clone(),
                                passed: false,
                                duration: Duration::from_secs(0),
                                output: "FAILED".to_string(),
                                error: Some(format!("Batch execution failed: {}", e)),
                                stdout: String::new(),
                                stderr: String::new(),
                            };
                            results_clone.insert(result.test_id.clone(), result);
                        }
                    }
                }
            });
        });

        // Collect results - wait for all parallel tasks to complete
        // Note: We don't drop the pool as it's global and reused

        // Convert Arc<DashMap> to owned DashMap
        let results_map = match Arc::try_unwrap(results) {
            Ok(map) => map,
            Err(arc) => {
                // If we can't unwrap, clone the data
                if self.verbose {
                    eprintln!(
                        "Warning: Had to clone results map, some threads may still be running"
                    );
                }
                (*arc).clone()
            }
        };
        let mut all_results: Vec<TestResult> = results_map.into_iter().map(|(_, v)| v).collect();

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
        let total_tests: usize = module_groups.values().map(|v| v.len()).sum();

        // Optimize batch size based on subprocess overhead
        // Subprocess overhead is ~25-140ms, average test execution is ~1ms
        // So we want large batches to amortize the overhead
        let optimal_batch_size = if total_tests < 100 {
            // For small test counts, use single batch to minimize overhead
            total_tests
        } else if total_tests < 500 {
            // For medium counts, use fewer larger batches
            (total_tests / self.num_workers).max(50)
        } else {
            // For large counts, balance parallelism with overhead
            100 // Fixed size that amortizes overhead well
        };

        // Create a single batch for very small test counts
        if total_tests <= optimal_batch_size {
            batches.push(module_groups.into_values().flatten().collect());
            return batches;
        }

        for (_, module_tests) in module_groups {
            // Keep modules together when possible
            if module_tests.len() <= optimal_batch_size {
                batches.push(module_tests);
            } else {
                for chunk in module_tests.chunks(optimal_batch_size) {
                    batches.push(chunk.to_vec());
                }
            }
        }

        batches
    }

    fn execute_batch_optimized(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        // Build ultra-optimized Python code
        let runner_code = self.build_ultra_fast_runner(&tests);

        // Execute in subprocess
        self.execute_subprocess(&runner_code)
    }

    fn build_ultra_fast_runner(&self, tests: &[TestItem]) -> String {
        let base_code = self.build_test_runner_code(tests);

        if self.coverage_enabled {
            self.wrap_with_coverage(&base_code)
        } else {
            base_code
        }
    }

    fn wrap_with_coverage(&self, code: &str) -> String {
        format!(
            r#"
import coverage
import sys
import os

# Start coverage collection
cov = coverage.Coverage(
    data_file='.coverage.fastest.{{}}',
    source={:?},
    omit=['*/test_*.py', '*/tests/*', '*/conftest.py']
)
cov.start()

try:
{}
finally:
    # Stop coverage and save
    cov.stop()
    cov.save()
"#,
            self.coverage_source
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            code.lines()
                .map(|line| format!("    {}", line))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn build_test_runner_code(&self, tests: &[TestItem]) -> String {
        // Check if we can use the fast path
        let has_fixtures = tests.iter().any(|t| !t.fixture_deps.is_empty());
        let has_async = tests.iter().any(|t| t.is_async);
        let has_classes = tests.iter().any(|t| t.class_name.is_some());
        let has_parametrize = tests
            .iter()
            .any(|t| t.decorators.iter().any(|d| d.starts_with("__params__=")));

        // Use fast path for simple tests (much more aggressive)
        if !has_fixtures && !has_async && !has_classes && !has_parametrize {
            return self.build_fast_runner(tests);
        }

        // Use full path for complex tests
        self.build_full_runner(tests)
    }

    fn build_fast_runner(&self, tests: &[TestItem]) -> String {
        // Ultra-minimal Python code for maximum speed
        let mut modules = std::collections::HashSet::new();
        let mut test_functions = Vec::new();
        let mut test_dirs = std::collections::HashSet::new();

        for test in tests {
            let module = test.id.split("::").next().unwrap_or("test").to_string();
            modules.insert(module.clone());

            let func_name = test.id.split("::").nth(1).unwrap_or(&test.function_name).to_string();
            test_functions.push((module, func_name, test.id.clone()));
            
            // Collect test directories
            if let Some(parent) = test.path.parent() {
                test_dirs.insert(parent.to_string_lossy().to_string());
            }
        }

        // Build minimal code with pre-allocated results
        let mut code = String::with_capacity(1024 + tests.len() * 256);

        code.push_str("import sys,json,time,io\n");
        code.push_str("from contextlib import redirect_stdout,redirect_stderr\n");
        
        // Add test directories to sys.path
        code.push_str("import os\n");
        code.push_str("sys.path.insert(0,os.getcwd())\n");
        for dir in &test_dirs {
            let escaped_dir = dir.replace("\\", "\\\\").replace("'", "\\'");
            code.push_str(&format!("sys.path.insert(0,'{}')\n", escaped_dir));
        }

        // Import all modules at once
        for module in &modules {
            code.push_str(&format!("import {}\n", module));
        }

        // Pre-allocate results list
        code.push_str(&format!("r=[None]*{}\n", tests.len()));
        code.push_str("p=time.perf_counter\n"); // Alias for speed

        // Execute tests with minimal overhead
        for (idx, (module, func_name, test_id)) in test_functions.iter().enumerate() {
            code.push_str(&format!(
                "stdout_buf=io.StringIO();stderr_buf=io.StringIO()\ntry:\n    s=p()\n    with redirect_stdout(stdout_buf),redirect_stderr(stderr_buf):\n        {}.{}()\n    r[{}]={{'id':'{}','passed':True,'duration':p()-s,'stdout':stdout_buf.getvalue(),'stderr':stderr_buf.getvalue()}}\nexcept Exception as e:\n    r[{}]={{'id':'{}','passed':False,'duration':p()-s,'stdout':stdout_buf.getvalue(),'stderr':stderr_buf.getvalue(),'error':str(e)}}\n",
                module, func_name, idx, test_id, idx, test_id
            ));
        }

        code.push_str("print(json.dumps({'results':r}))\n");
        
        if self.verbose {
            eprintln!("Generated fast runner code:\n{}", code);
        }
        
        code
    }

    fn build_full_runner(&self, tests: &[TestItem]) -> String {
        // Group tests by module for single import
        let mut module_tests: HashMap<String, Vec<&TestItem>> = HashMap::new();
        let mut test_dirs = std::collections::HashSet::new();
        let mut all_fixture_deps = std::collections::HashSet::new();

        for test in tests {
            // Extract module name from test ID, handling leading dots
            let test_id = test.id.trim_start_matches('.');
            let module = test_id.split("::").next().unwrap_or("test");
            module_tests
                .entry(module.to_string())
                .or_default()
                .push(test);

            // Collect unique directories containing test files
            if let Some(parent) = test.path.parent() {
                test_dirs.insert(parent.to_string_lossy().to_string());
            }

            // Collect all fixture dependencies
            for dep in &test.fixture_deps {
                all_fixture_deps.insert(dep.clone());
            }
        }

        let mut imports = String::new();
        let mut test_map = String::new();
        let mut fixture_setup_code = String::new();

        // Generate code for fixtures
        let mut user_fixture_setup = String::new();
        let mut fixture_modules = std::collections::HashSet::new();

        for fixture_name in &all_fixture_deps {
            // Validate fixture name before checking if built-in
            if !fixture_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_')
            {
                if self.verbose {
                    eprintln!("Warning: Invalid fixture name: {}", fixture_name);
                }
                continue;
            }

            if crate::fixtures::is_builtin_fixture(fixture_name) {
                // Handle built-in fixtures
                if fixture_name == "capsys" {
                    if let Some(code) = crate::fixtures::generate_builtin_fixture_code(fixture_name)
                    {
                        fixture_setup_code.push_str(&format!(
                            "\n# Built-in fixture class: {}\n{}",
                            fixture_name, code
                        ));
                    }
                } else {
                    // For other built-in fixtures, define and instantiate them globally for now.
                    if let Some(code) = crate::fixtures::generate_builtin_fixture_code(fixture_name)
                    {
                        fixture_setup_code
                            .push_str(&format!("\n# Built-in fixture: {}\n{}", fixture_name, code));
                        fixture_setup_code.push_str(&format!("\nif '{}' not in fixture_instances: fixture_instances['{}'] = {}_fixture()\n", fixture_name, fixture_name, fixture_name));
                    }
                }
            } else {
                // Handle user-defined fixtures
                // Collect modules that might contain fixtures
                for (module, _) in &module_tests {
                    fixture_modules.insert(module.clone());
                }
            }
        }

        // Always generate code for test instance management (needed for class-based tests)
        user_fixture_setup.push_str("\n# Store test instances to reuse for class fixtures\n");
        user_fixture_setup.push_str("_test_instances = {}\n\n");

        user_fixture_setup.push_str("def get_or_create_test_instance(test_class, test_id):\n");
        user_fixture_setup.push_str("    '''Get or create test instance with autouse fixtures run'''\n");
        user_fixture_setup.push_str("    cache_key = (test_class, test_id)\n");
        user_fixture_setup.push_str("    if cache_key in _test_instances:\n");
        user_fixture_setup.push_str("        return _test_instances[cache_key]\n");
        user_fixture_setup.push_str("    \n");
        user_fixture_setup.push_str("    instance = test_class()\n");
        user_fixture_setup.push_str("    \n");
        user_fixture_setup.push_str("    # Run autouse fixtures\n");
        user_fixture_setup.push_str("    for attr_name in dir(test_class):\n");
        user_fixture_setup.push_str("        attr = getattr(test_class, attr_name)\n");
        user_fixture_setup.push_str("        if hasattr(attr, '_pytestfixturefunction') and hasattr(attr._pytestfixturefunction, 'autouse') and attr._pytestfixturefunction.autouse:\n");
        user_fixture_setup.push_str("            bound_method = getattr(instance, attr_name)\n");
        user_fixture_setup.push_str("            if hasattr(bound_method, '__wrapped__'):\n");
        user_fixture_setup.push_str("                bound_method = bound_method.__wrapped__.__get__(instance, test_class)\n");
        user_fixture_setup.push_str("            bound_method()\n");
        user_fixture_setup.push_str("    \n");
        user_fixture_setup.push_str("    _test_instances[cache_key] = instance\n");
        user_fixture_setup.push_str("    return instance\n\n");

        // Generate code to execute user fixtures
        if !fixture_modules.is_empty() {
            user_fixture_setup.push_str("# Find and execute user-defined fixtures\n");
            user_fixture_setup
                .push_str("# Track fixture execution to detect circular dependencies\n");
            user_fixture_setup.push_str("_fixture_executing = set()\n\n");

            user_fixture_setup.push_str("def get_fixture_value(fixture_name, test_module, test_class=None, test_id=None):\n");
            user_fixture_setup.push_str(
                "    '''Get fixture value with dependency resolution and scope caching'''\n",
            );
            user_fixture_setup.push_str("    # Try to find fixture in different scopes\n");
            user_fixture_setup.push_str("    fixture_obj = None\n");
            user_fixture_setup.push_str("    fixture_scope_info = None\n");
            user_fixture_setup.push_str("    needs_self = False\n");
            user_fixture_setup.push_str("    fixture_scope = 'function'  # default scope\n");
            user_fixture_setup.push_str("    \n");
            user_fixture_setup.push_str("    # 1. Check class fixtures (if test_class provided)\n");
            user_fixture_setup
                .push_str("    if test_class and hasattr(test_class, fixture_name):\n");
            user_fixture_setup
                .push_str("        fixture_obj = getattr(test_class, fixture_name)\n");
            user_fixture_setup.push_str("        fixture_scope_info = ('class', test_class)\n");
            user_fixture_setup.push_str("    # 2. Check module fixtures\n");
            user_fixture_setup.push_str("    elif hasattr(test_module, fixture_name):\n");
            user_fixture_setup
                .push_str("        fixture_obj = getattr(test_module, fixture_name)\n");
            user_fixture_setup.push_str("        fixture_scope_info = ('module', test_module)\n");
            user_fixture_setup.push_str("    \n");
            user_fixture_setup.push_str("    if not fixture_obj:\n");
            user_fixture_setup.push_str("        return None\n");
            user_fixture_setup.push_str("    \n");
            user_fixture_setup.push_str("    # Get fixture scope from decorator\n");
            user_fixture_setup.push_str("    if hasattr(fixture_obj, '_pytestfixturefunction'):\n");
            user_fixture_setup
                .push_str("        fixture_func = fixture_obj._pytestfixturefunction\n");
            user_fixture_setup.push_str("        if hasattr(fixture_func, 'scope'):\n");
            user_fixture_setup.push_str("            fixture_scope = fixture_func.scope\n");
            user_fixture_setup.push_str("    \n");
            user_fixture_setup.push_str("    # Determine cache key based on scope\n");
            user_fixture_setup.push_str("    cache_key = None\n");
            user_fixture_setup.push_str("    if fixture_scope == 'session':\n");
            user_fixture_setup.push_str("        cache_key = fixture_name\n");
            user_fixture_setup.push_str("        if cache_key in session_fixture_cache:\n");
            user_fixture_setup.push_str("            return session_fixture_cache[cache_key]\n");
            user_fixture_setup.push_str("    elif fixture_scope == 'module':\n");
            user_fixture_setup
                .push_str("        cache_key = (test_module.__name__, fixture_name)\n");
            user_fixture_setup.push_str("        if cache_key in module_fixture_cache:\n");
            user_fixture_setup.push_str("            return module_fixture_cache[cache_key]\n");
            user_fixture_setup.push_str("    elif fixture_scope == 'class' and test_class:\n");
            user_fixture_setup.push_str("        cache_key = (test_class, fixture_name)\n");
            user_fixture_setup.push_str("        if cache_key in class_fixture_cache:\n");
            user_fixture_setup.push_str("            return class_fixture_cache[cache_key]\n");
            user_fixture_setup.push_str("    elif fixture_scope == 'function':\n");
            user_fixture_setup.push_str("        # Function-scoped fixtures are cached per test\n");
            user_fixture_setup.push_str("        if fixture_name in fixture_instances:\n");
            user_fixture_setup.push_str("            return fixture_instances[fixture_name]\n");
            user_fixture_setup.push_str("    \n");
            user_fixture_setup.push_str("    # Check for circular dependencies\n");
            user_fixture_setup.push_str("    if fixture_name in _fixture_executing:\n");
            user_fixture_setup.push_str("        raise ValueError(f'Circular fixture dependency detected: {fixture_name}')\n");
            user_fixture_setup.push_str("    \n");
            user_fixture_setup.push_str("    _fixture_executing.add(fixture_name)\n");
            user_fixture_setup.push_str("    try:\n");
            user_fixture_setup
                .push_str("        # Get the actual function from pytest fixture wrapper\n");
            user_fixture_setup.push_str("        actual_func = None\n");
            user_fixture_setup.push_str("        fixture_metadata = {}\n");
            user_fixture_setup.push_str("        \n");
            user_fixture_setup.push_str("        if hasattr(fixture_obj, '__wrapped__'):\n");
            user_fixture_setup.push_str("            actual_func = fixture_obj.__wrapped__\n");
            user_fixture_setup.push_str("            # Try to get fixture metadata\n");
            user_fixture_setup
                .push_str("            if hasattr(fixture_obj, '_pytestfixturefunction'):\n");
            user_fixture_setup.push_str("                fixture_metadata = getattr(fixture_obj._pytestfixturefunction, '__dict__', {})\n");
            user_fixture_setup.push_str("        elif hasattr(fixture_obj, 'func'):\n");
            user_fixture_setup.push_str("            actual_func = fixture_obj.func\n");
            user_fixture_setup.push_str("        elif callable(fixture_obj) and not hasattr(fixture_obj, '_pytestfixturefunction'):\n");
            user_fixture_setup.push_str("            actual_func = fixture_obj\n");
            user_fixture_setup.push_str("        \n");
            user_fixture_setup.push_str("        if actual_func:\n");
            user_fixture_setup
                .push_str("            # Get fixture dependencies from function signature\n");
            user_fixture_setup.push_str("            import inspect\n");
            user_fixture_setup.push_str("            try:\n");
            user_fixture_setup.push_str("                sig = inspect.signature(actual_func)\n");
            user_fixture_setup.push_str("                kwargs = {}\n");
            user_fixture_setup.push_str("                \n");
            user_fixture_setup.push_str("                # Recursively resolve dependencies\n");
            user_fixture_setup.push_str("                for param_name in sig.parameters:\n");
            user_fixture_setup.push_str("                    if param_name == 'request':\n");
            user_fixture_setup
                .push_str("                        # Create a simple request object for parametrized fixtures\n");
            user_fixture_setup.push_str("                        class SimpleRequest:\n");
            user_fixture_setup.push_str("                            def __init__(self, param):\n");
            user_fixture_setup.push_str("                                self.param = param\n");
            user_fixture_setup.push_str("                        \n");
            user_fixture_setup.push_str("                        # Check if this is a parametrized fixture\n");
            user_fixture_setup.push_str("                        if hasattr(fixture_obj, 'params') or (hasattr(fixture_obj, '_pytestfixturefunction') and hasattr(fixture_obj._pytestfixturefunction, 'params')):\n");
            user_fixture_setup.push_str("                            # For now, use the first param value\n");
            user_fixture_setup.push_str("                            params = getattr(fixture_obj, 'params', None) or getattr(fixture_obj._pytestfixturefunction, 'params', [1])\n");
            user_fixture_setup.push_str("                            if params:\n");
            user_fixture_setup.push_str("                                kwargs['request'] = SimpleRequest(params[0])\n");
            user_fixture_setup.push_str("                        continue\n");
            user_fixture_setup.push_str("                    if param_name == 'self' and fixture_scope_info and fixture_scope_info[0] == 'class':\n");
            user_fixture_setup.push_str(
                "                        # Skip 'self' for class fixtures, will be handled below\n",
            );
            user_fixture_setup.push_str("                        continue\n");
            user_fixture_setup.push_str("                    # Recursively get fixture value\n");
            user_fixture_setup.push_str("                    dep_value = get_fixture_value(param_name, test_module, test_class, test_id)\n");
            user_fixture_setup.push_str("                    if dep_value is not None:\n");
            user_fixture_setup.push_str("                        kwargs[param_name] = dep_value\n");
            user_fixture_setup.push_str("                \n");
            user_fixture_setup.push_str("                # Execute the fixture\n");
            user_fixture_setup.push_str("                fixture_value = None\n");
            user_fixture_setup
                .push_str("                if inspect.isgeneratorfunction(actual_func):\n");
            user_fixture_setup.push_str("                    # Handle yield fixtures\n");
            user_fixture_setup.push_str("                    gen = actual_func(**kwargs)\n");
            user_fixture_setup.push_str("                    fixture_value = next(gen)\n");
            user_fixture_setup
                .push_str("                    # TODO: Store generator for teardown\n");
            user_fixture_setup.push_str("                else:\n");
            user_fixture_setup
                .push_str("                    # Handle class fixtures that need self\n");
            user_fixture_setup.push_str(
                "                    if fixture_scope_info and fixture_scope_info[0] == 'class':\n",
            );
            user_fixture_setup
                .push_str("                        # Use cached instance with autouse fixtures already run\n");
            user_fixture_setup.push_str("                        instance = get_or_create_test_instance(fixture_scope_info[1], test_id)\n");
            user_fixture_setup.push_str("                        bound_method = getattr(instance, fixture_name)\n");
            user_fixture_setup
                .push_str("                        if hasattr(bound_method, '__wrapped__'):\n");
            user_fixture_setup.push_str("                            bound_method = bound_method.__wrapped__.__get__(instance, fixture_scope_info[1])\n");
            user_fixture_setup
                .push_str("                        fixture_value = bound_method(**kwargs)\n");
            user_fixture_setup.push_str("                    else:\n");
            user_fixture_setup
                .push_str("                        fixture_value = actual_func(**kwargs)\n");
            user_fixture_setup.push_str("                \n");
            user_fixture_setup
                .push_str("                # Cache the fixture value based on scope\n");
            user_fixture_setup.push_str("                if fixture_value is not None:\n");
            user_fixture_setup.push_str("                    if fixture_scope == 'session':\n");
            user_fixture_setup.push_str(
                "                        session_fixture_cache[cache_key] = fixture_value\n",
            );
            user_fixture_setup.push_str("                    elif fixture_scope == 'module':\n");
            user_fixture_setup.push_str(
                "                        module_fixture_cache[cache_key] = fixture_value\n",
            );
            user_fixture_setup
                .push_str("                    elif fixture_scope == 'class' and cache_key:\n");
            user_fixture_setup.push_str(
                "                        class_fixture_cache[cache_key] = fixture_value\n",
            );
            user_fixture_setup.push_str("                    elif fixture_scope == 'function':\n");
            user_fixture_setup.push_str(
                "                        fixture_instances[fixture_name] = fixture_value\n",
            );
            user_fixture_setup.push_str("                \n");
            user_fixture_setup.push_str("                return fixture_value\n");
            user_fixture_setup.push_str("            except Exception as e:\n");
            user_fixture_setup.push_str("                import traceback\n");
            user_fixture_setup.push_str("                print(f'Failed to execute fixture {fixture_name}: {e}', file=sys.stderr)\n");
            user_fixture_setup
                .push_str("                print(traceback.format_exc(), file=sys.stderr)\n");
            user_fixture_setup.push_str("                return None\n");
            user_fixture_setup.push_str("        return None\n");
            user_fixture_setup.push_str("    finally:\n");
            user_fixture_setup.push_str("        _fixture_executing.discard(fixture_name)\n\n");
        }

        for (module, tests) in module_tests {
            // Convert module path to Python import format, with validation
            let import_module = module.replace(['/', '\\'], ".");
            // Validate module name to prevent code injection
            if !import_module
                .chars()
                .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
            {
                eprintln!("Warning: Skipping invalid module name: {}", import_module);
                continue;
            }
            imports.push_str(&format!("import {}\n", import_module));

            for test in tests {
                // Extract base function name (without parameter suffix)
                let method_name = if let Some(bracket_pos) = test.function_name.find('[') {
                    &test.function_name[..bracket_pos]
                } else {
                    &test.function_name
                };
                let base_function_name = method_name;

                // Check for @pytest.mark.xfail
                // let is_xfail = BuiltinMarker::is_xfail(&extract_markers(&test.decorators));

                // Check if this is a parametrized test
                let params_decorator = test
                    .decorators
                    .iter()
                    .find(|d| d.starts_with("__params__="))
                    .map(|d| d.trim_start_matches("__params__="));

                // For class methods, extract just the method name without class prefix
                let method_name = if test.class_name.is_some() {
                    // If function_path contains "::", extract only the method name
                    if let Some(pos) = base_function_name.rfind("::") {
                        &base_function_name[pos + 2..]
                    } else {
                        base_function_name
                    }
                } else {
                    base_function_name
                };

                if let Some(params_json) = params_decorator {
                    // This is a parametrized test
                    test_map.push_str(&format!(
                        "    {}: {{ 'func': {}.{}, 'async': {}, 'xfail': {}, 'params': json.loads({}), 'fixtures': {:?}, 'class_name': {} }},\n",
                        serde_json::to_string(&test.id).unwrap_or_else(|_| format!("{:?}", test.id)),
                        import_module,
                        if let Some(class) = &test.class_name {
                            format!("{}.{}", class, method_name)
                        } else {
                            base_function_name.to_string()
                        },
                        if test.is_async { "True" } else { "False" },
                        if test.is_xfail { "True" } else { "False" },
                        serde_json::to_string(params_json).unwrap_or_else(|_| "'{}'".to_string()),
                        test.fixture_deps,
                        if let Some(class) = &test.class_name {
                            format!("'{}'", class)
                        } else {
                            "None".to_string()
                        }
                    ));
                } else {
                    // Non-parametrized test
                    test_map.push_str(&format!(
                        "    {}: {{ 'func': {}.{}, 'async': {}, 'xfail': {}, 'fixtures': {:?}, 'class_name': {} }},\n",
                        serde_json::to_string(&test.id).unwrap_or_else(|_| format!("{:?}", test.id)),
                        import_module,
                        if let Some(class) = &test.class_name {
                            format!("{}.{}", class, method_name)
                        } else {
                            base_function_name.to_string()
                        },
                        if test.is_async { "True" } else { "False" },
                        if test.is_xfail { "True" } else { "False" },
                        test.fixture_deps,
                        if let Some(class) = &test.class_name {
                            format!("'{}'", class)
                        } else {
                            "None".to_string()
                        }
                    ));
                }
            }
        }

        // Build sys.path additions for test directories with proper escaping
        let mut sys_path_additions = String::new();
        for dir in test_dirs {
            // Use JSON serialization for proper escaping
            let escaped_dir = serde_json::to_string(&dir)
                .unwrap_or_else(|_| format!("r'{}'", dir.replace("'", "\\'")))
                .trim_matches('"')
                .to_string();
            sys_path_additions.push_str(&format!("sys.path.insert(0, \"{}\")\n", escaped_dir));
        }

        // Format imports with proper indentation for the with block
        let indented_imports = imports
            .lines()
            .map(|line| format!("    {}", line))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"
import sys
import os
import json
import time
import traceback
from io import StringIO
from contextlib import redirect_stdout, redirect_stderr
import inspect

# Add current directory to Python path
sys.path.insert(0, os.getcwd())

# Add test directories to Python path
{}

# Pre-import all modules with output capture
import_stdout = StringIO()
import_stderr = StringIO()
with redirect_stdout(import_stdout), redirect_stderr(import_stderr):
{}

# Discard any output from imports/autouse fixtures
import_stdout.close()
import_stderr.close()

# Fixture instances with scope caching
fixture_instances = {{}}  # Function-scoped fixtures (recreated each test)
module_fixture_cache = {{}}  # Module-scoped fixtures
session_fixture_cache = {{}}  # Session-scoped fixtures
class_fixture_cache = {{}}  # Class-scoped fixtures

# Fixture Setup Code (defines SimpleCapsys class, etc.)
{}

{}

# Pre-compiled test map
test_map = {{
{}
}}

# Run tests synchronously for now to avoid async complexity
results = []

for test_id, test_info in test_map.items():
    # Clear function-scoped fixtures for each test
    fixture_instances.clear()
    _test_instances.clear()  # Clear test instances for each test
    
    stdout_buf = StringIO()
    stderr_buf = StringIO()
    
    kwargs = {{}}
    
    # Add params to kwargs
    if 'params' in test_info:
        params_data = test_info['params']
        if isinstance(params_data, list) and 'param_names' in test_info:
             for i, name in enumerate(test_info['param_names']):
                if i < len(params_data):
                    kwargs[name] = params_data[i]
        elif isinstance(params_data, dict):
            kwargs.update(params_data)

    # Instantiate and add requested fixtures to kwargs
    for fix_name in test_info.get('fixtures', []):
        if fix_name == 'capsys':
            # Create a new SimpleCapsys instance for each test
            kwargs['capsys'] = SimpleCapsys(stdout_buf, stderr_buf)
        elif fix_name in fixture_instances:
            kwargs[fix_name] = fixture_instances[fix_name]
        else:
            # Try to find and execute user-defined fixture
            test_module = sys.modules.get(test_id.split('::')[0])
            if test_module:
                # Check if this is a class-based test
                test_class = None
                if '::' in test_id and test_id.count('::') >= 2:
                    # Extract class name
                    parts = test_id.split('::')
                    if len(parts) >= 3:
                        class_name = parts[1]
                        if hasattr(test_module, class_name):
                            test_class = getattr(test_module, class_name)
                
                fixture_value = get_fixture_value(fix_name, test_module, test_class, test_id)
                if fixture_value is not None:
                    kwargs[fix_name] = fixture_value
        
    start = time.perf_counter()
    try:
        with redirect_stdout(stdout_buf), redirect_stderr(stderr_buf):
            # Handle class-based tests
            if test_info.get('class_name'):
                test_module = sys.modules.get(test_id.split('::')[0])
                if test_module and hasattr(test_module, test_info['class_name']):
                    test_class = getattr(test_module, test_info['class_name'])
                    test_instance = get_or_create_test_instance(test_class, test_id)
                    test_method = getattr(test_instance, test_info['func'].__name__)
                    test_method(**kwargs)
                else:
                    raise AttributeError('Test class {{}} not found'.format(test_info['class_name']))
            else:
                test_info['func'](**kwargs)
        
        duration = time.perf_counter() - start
        
        if test_info['xfail']:
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

print(json.dumps({{'results': results}}))
"#,
            sys_path_additions, indented_imports, fixture_setup_code, user_fixture_setup, test_map
        )
    }

    fn execute_subprocess(&self, code: &str) -> Result<Vec<TestResult>> {
        // Debug: write code to file
        if self.verbose {
            if let Ok(mut file) = std::fs::File::create("/tmp/fastest_debug.py") {
                let _ = write!(file, "{}", code);
                eprintln!("Debug: Python code written to /tmp/fastest_debug.py");
            }
        }

        let mut cmd = Command::new(&*PYTHON_CMD);
        cmd.arg("-c")
            .arg(code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Preserve virtual environment
        if let Ok(venv) = std::env::var("VIRTUAL_ENV") {
            cmd.env("VIRTUAL_ENV", venv);
        }
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }
        if let Ok(pythonpath) = std::env::var("PYTHONPATH") {
            cmd.env("PYTHONPATH", pythonpath);
        }

        let output = cmd
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
            } else if self.verbose {
                eprintln!("No results array in JSON");
            }
        } else {
            // Try to find JSON in output (in case there's extra output)
            if let Some(json_start) = stdout.find("{\"results\":") {
                if let Some(json_end) = stdout[json_start..].find("\n").map(|i| i + json_start) {
                    let json_str = &stdout[json_start..json_end];
                    if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(results_array) = json_data["results"].as_array() {
                            if self.verbose {
                                eprintln!("Found {} results in extracted JSON", results_array.len());
                            }
                            let results = results_array
                                .iter()
                                .filter_map(|r| self.parse_test_result(r))
                                .collect();
                            return Ok(results);
                        }
                    }
                } else {
                    // Try without newline limitation
                    let json_str = &stdout[json_start..];
                    if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(results_array) = json_data["results"].as_array() {
                            if self.verbose {
                                eprintln!("Found {} results in extracted JSON (no newline)", results_array.len());
                            }
                            let results = results_array
                                .iter()
                                .filter_map(|r| self.parse_test_result(r))
                                .collect();
                            return Ok(results);
                        }
                    }
                }
            }
            
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
