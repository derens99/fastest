use anyhow::Result;
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use super::{BatchExecutor, TestResult};
use crate::TestItem;

pub struct ParallelExecutor {
    num_workers: usize,
    results: Arc<DashMap<String, TestResult>>,
    verbose: bool,
}

impl ParallelExecutor {
    pub fn new(num_workers: Option<usize>, verbose: bool) -> Self {
        let num_workers = num_workers.unwrap_or_else(num_cpus::get);
        if verbose {
            eprintln!("🚀 Parallel execution with {} workers", num_workers);
        }

        Self {
            num_workers,
            results: Arc::new(DashMap::new()),
            verbose,
        }
    }

    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        if tests.is_empty() {
            return Ok(vec![]);
        }

        // Group tests by module to minimize import overhead
        let grouped = self.group_by_module(tests);

        if self.verbose {
            eprintln!(
                "📦 Grouped {} tests into {} modules",
                grouped.values().map(|v| v.len()).sum::<usize>(),
                grouped.len()
            );
        }

        // Set up thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.num_workers)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create thread pool: {}", e))?;

        // Execute in parallel
        let results_clone = self.results.clone();
        let verbose = self.verbose;

        pool.install(|| {
            grouped.par_iter().for_each(|(module, tests)| {
                if verbose {
                    eprintln!("🧪 Running {} tests from {}", tests.len(), module);
                }

                // Use BatchExecutor to run all tests in this module efficiently
                let executor = BatchExecutor::new();
                let module_results = executor.execute_tests(tests.clone());

                // Store results
                for result in module_results {
                    results_clone.insert(result.test_id.clone(), result);
                }
            });
        });

        // Collect results in the original order
        let mut results = Vec::with_capacity(self.results.len());
        for entry in self.results.iter() {
            results.push(entry.value().clone());
        }

        Ok(results)
    }

    fn group_by_module(&self, tests: Vec<TestItem>) -> HashMap<String, Vec<TestItem>> {
        let mut grouped: HashMap<String, Vec<TestItem>> = HashMap::new();

        for test in tests {
            // Group by directory path
            let module = test
                .path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());

            grouped.entry(module).or_insert_with(Vec::new).push(test);
        }

        grouped
    }
}

// Progress reporter for parallel execution
pub struct ProgressReporter {
    total: usize,
    completed: Arc<std::sync::atomic::AtomicUsize>,
    passed: Arc<std::sync::atomic::AtomicUsize>,
    failed: Arc<std::sync::atomic::AtomicUsize>,
}

impl ProgressReporter {
    pub fn new(total: usize) -> Self {
        use std::sync::atomic::AtomicUsize;

        Self {
            total,
            completed: Arc::new(AtomicUsize::new(0)),
            passed: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn report_result(&self, result: &TestResult) {
        use std::sync::atomic::Ordering;

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

        eprint!(
            "\r[{}/{}] {} passed, {} failed",
            completed, self.total, passed, failed
        );

        // Clear line if all tests completed
        if completed == self.total {
            eprintln!(); // New line at the end
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_item(path: &str, name: &str) -> TestItem {
        TestItem {
            id: format!("{}::{}", path, name),
            path: PathBuf::from(path),
            name: name.to_string(),
            function_name: name.to_string(),
            line_number: 1,
            is_async: false,
            class_name: None,
            decorators: vec![],
            fixture_deps: vec![],
        }
    }

    #[test]
    fn test_grouping_by_module() {
        let executor = ParallelExecutor::new(None, false);
        let tests = vec![
            create_test_item("dir1/test1.py", "test_one"),
            create_test_item("dir1/test1.py", "test_two"),
            create_test_item("dir2/test2.py", "test_three"),
            create_test_item("./test_root.py", "test_four"),
        ];

        let grouped = executor.group_by_module(tests);
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped.get("dir1").unwrap().len(), 2);
        assert_eq!(grouped.get("dir2").unwrap().len(), 1);
        assert_eq!(grouped.get(".").unwrap().len(), 1);
    }

    #[test]
    fn test_empty_test_list() {
        let executor = ParallelExecutor::new(Some(2), false);
        let results = executor.execute(vec![]).unwrap();
        assert!(results.is_empty());
    }
}
