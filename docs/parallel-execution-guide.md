# Parallel Test Execution Implementation Guide

## Overview

This guide walks through implementing parallel test execution for the fastest test runner, which is the next major feature on the roadmap.

## Step 1: Update Dependencies

### `crates/fastest-core/Cargo.toml`
```toml
[dependencies]
# ... existing dependencies ...
rayon = "1.8"
dashmap = "5.5"
num_cpus = "1.16"
```

## Step 2: Create Parallel Executor Module

### Create `crates/fastest-core/src/executor/parallel.rs`

```rust
use std::sync::Arc;
use rayon::prelude::*;
use dashmap::DashMap;
use crate::{TestItem, TestResult};
use super::python::PythonExecutor;

pub struct ParallelExecutor {
    num_workers: usize,
    results: Arc<DashMap<String, TestResult>>,
}

impl ParallelExecutor {
    pub fn new(num_workers: Option<usize>) -> Self {
        let num_workers = num_workers.unwrap_or_else(num_cpus::get);
        Self {
            num_workers,
            results: Arc::new(DashMap::new()),
        }
    }

    pub fn execute(&self, tests: Vec<TestItem>) -> Vec<TestResult> {
        // Group tests by module to minimize import overhead
        let grouped = self.group_by_module(tests);
        
        // Set up thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_workers)
            .build()
            .unwrap();
        
        // Execute in parallel
        pool.install(|| {
            grouped.par_iter().for_each(|(module, tests)| {
                self.execute_module_tests(module, tests);
            });
        });
        
        // Collect results
        self.results.iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    fn group_by_module(&self, tests: Vec<TestItem>) -> HashMap<String, Vec<TestItem>> {
        let mut grouped = HashMap::new();
        for test in tests {
            let module = test.path.parent().unwrap().to_string_lossy().to_string();
            grouped.entry(module).or_insert_with(Vec::new).push(test);
        }
        grouped
    }
    
    fn execute_module_tests(&self, module: &str, tests: &[TestItem]) {
        // Create a single subprocess for all tests in this module
        let executor = PythonExecutor::new();
        
        for test in tests {
            let result = executor.run_test(test);
            self.results.insert(test.id.clone(), result);
        }
    }
}
```

## Step 3: Update CLI to Support Parallel Execution

### Update `crates/fastest-cli/src/main.rs`

```rust
#[derive(Parser)]
struct Cli {
    // ... existing fields ...
    
    /// Number of parallel workers (default: auto-detect)
    #[arg(short = 'n', long = "num-workers")]
    num_workers: Option<usize>,
}
```

### Update run command handler:

```rust
fn handle_run_command(args: &Cli) -> Result<()> {
    let tests = discover_tests(&args.path)?;
    
    let results = if args.num_workers.is_some() || args.num_workers == Some(0) {
        // Use parallel execution
        let executor = ParallelExecutor::new(args.num_workers);
        executor.execute(tests)
    } else {
        // Use sequential execution (existing code)
        run_tests_batch(tests)
    };
    
    display_results(&results);
    Ok(())
}
```

## Step 4: Update Python Bindings

### Update `crates/fastest-python/src/lib.rs`

```rust
#[pyfunction]
#[pyo3(signature = (tests, num_workers=None))]
fn run_tests_parallel(
    py: Python,
    tests: Vec<TestItem>,
    num_workers: Option<usize>
) -> PyResult<Vec<TestResult>> {
    py.allow_threads(|| {
        let executor = ParallelExecutor::new(num_workers);
        Ok(executor.execute(tests))
    })
}

#[pymodule]
fn fastest(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // ... existing functions ...
    m.add_function(wrap_pyfunction!(run_tests_parallel, m)?)?;
    Ok(())
}
```

## Step 5: Add Progress Reporting

### Create `crates/fastest-core/src/executor/progress.rs`

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct ProgressReporter {
    total: usize,
    completed: Arc<AtomicUsize>,
    passed: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
}

impl ProgressReporter {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: Arc::new(AtomicUsize::new(0)),
            passed: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn report_result(&self, result: &TestResult) {
        self.completed.fetch_add(1, Ordering::SeqCst);
        if result.passed {
            self.passed.fetch_add(1, Ordering::SeqCst);
        } else {
            self.failed.fetch_add(1, Ordering::SeqCst);
        }
        
        // Print progress
        let completed = self.completed.load(Ordering::SeqCst);
        let passed = self.passed.load(Ordering::SeqCst);
        let failed = self.failed.load(Ordering::SeqCst);
        
        print!("\r[{}/{}] {} passed, {} failed", 
               completed, self.total, passed, failed);
        std::io::stdout().flush().unwrap();
    }
}
```

## Step 6: Add Tests

### Create `crates/fastest-core/src/executor/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parallel_execution() {
        let tests = vec![
            create_test_item("test1.py", "test_one"),
            create_test_item("test1.py", "test_two"),
            create_test_item("test2.py", "test_three"),
            create_test_item("test2.py", "test_four"),
        ];
        
        let executor = ParallelExecutor::new(Some(2));
        let results = executor.execute(tests);
        
        assert_eq!(results.len(), 4);
        assert!(results.iter().all(|r| r.passed));
    }
    
    #[test]
    fn test_grouping_by_module() {
        let executor = ParallelExecutor::new(None);
        let tests = vec![
            create_test_item("dir1/test1.py", "test_one"),
            create_test_item("dir1/test1.py", "test_two"),
            create_test_item("dir2/test2.py", "test_three"),
        ];
        
        let grouped = executor.group_by_module(tests);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get("dir1").unwrap().len(), 2);
        assert_eq!(grouped.get("dir2").unwrap().len(), 1);
    }
}
```

## Step 7: Benchmark Implementation

### Create `benchmarks/benchmark_parallel.py`

```python
#!/usr/bin/env python3
"""Benchmark parallel execution performance."""

import fastest
import time
import multiprocessing

def benchmark_parallel_execution():
    # Create large test suite
    test_dir = create_large_test_suite(num_files=50, tests_per_file=20)
    tests = fastest.discover_tests(test_dir)
    
    print(f"Benchmarking with {len(tests)} tests")
    print(f"CPU cores available: {multiprocessing.cpu_count()}")
    
    # Sequential execution
    start = time.time()
    results_seq = fastest.run_tests_batch(tests)
    seq_time = time.time() - start
    
    # Parallel execution with different worker counts
    for workers in [2, 4, 8, None]:  # None = auto-detect
        start = time.time()
        results_par = fastest.run_tests_parallel(tests, num_workers=workers)
        par_time = time.time() - start
        
        speedup = seq_time / par_time
        print(f"\nWorkers: {workers or 'auto'}")
        print(f"Sequential: {seq_time:.2f}s")
        print(f"Parallel: {par_time:.2f}s")
        print(f"Speedup: {speedup:.2f}x")
```

## Implementation Checklist

- [ ] Add dependencies to Cargo.toml
- [ ] Implement ParallelExecutor
- [ ] Update CLI with -n option
- [ ] Update Python bindings
- [ ] Add progress reporting
- [ ] Write unit tests
- [ ] Create benchmarks
- [ ] Update documentation
- [ ] Test on different platforms
- [ ] Handle edge cases (empty test list, single test, etc.)

## Next Steps After This Feature

1. **Optimize work distribution**: Implement test timing history to balance work better
2. **Add `--fail-fast` mode**: Stop all workers on first failure
3. **Implement test ordering**: Some tests may have dependencies
4. **Memory optimization**: Limit concurrent Python processes based on available RAM
5. **Better error handling**: Graceful degradation if parallel execution fails 