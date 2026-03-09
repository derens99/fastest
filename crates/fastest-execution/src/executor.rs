//! Hybrid test executor that switches between in-process and subprocess execution.
//!
//! For small test suites (<=20 tests), the in-process executor is used for
//! lower latency. For larger suites, the subprocess pool is used for better
//! isolation and parallelism.

use fastest_core::model::{TestItem, TestResult};

use crate::inprocess::InProcessExecutor;
use crate::subprocess::SubprocessPool;
use crate::timeout::TimeoutConfig;

/// Test count threshold: suites with this many tests or fewer use in-process execution.
pub const INPROCESS_THRESHOLD: usize = 20;

/// Strategy chosen by the hybrid executor for a given run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Direct in-process execution via PyO3 (low overhead, no isolation).
    InProcess,
    /// Subprocess pool with work-stealing (isolated, parallel).
    Subprocess,
}

/// Determines the execution strategy based on the number of tests.
pub fn select_strategy(test_count: usize) -> ExecutionStrategy {
    if test_count <= INPROCESS_THRESHOLD {
        ExecutionStrategy::InProcess
    } else {
        ExecutionStrategy::Subprocess
    }
}

/// A hybrid executor that automatically selects between in-process and
/// subprocess execution based on the test suite size.
pub struct HybridExecutor {
    inprocess: InProcessExecutor,
    subprocess: SubprocessPool,
}

impl HybridExecutor {
    /// Create a new hybrid executor with default settings.
    pub fn new() -> Self {
        Self {
            inprocess: InProcessExecutor::new(),
            subprocess: SubprocessPool::new(None),
        }
    }

    /// Create a hybrid executor with a specific number of subprocess workers.
    pub fn with_workers(num_workers: Option<usize>) -> Self {
        Self {
            inprocess: InProcessExecutor::new(),
            subprocess: SubprocessPool::new(num_workers),
        }
    }

    /// Create a hybrid executor with full configuration.
    pub fn with_config(num_workers: Option<usize>, timeout: TimeoutConfig) -> Self {
        Self {
            inprocess: InProcessExecutor::with_timeout(timeout.clone()),
            subprocess: SubprocessPool::new(num_workers).with_timeout(timeout),
        }
    }

    /// Returns which execution strategy will be used for the given number of tests.
    pub fn strategy_for(&self, test_count: usize) -> ExecutionStrategy {
        select_strategy(test_count)
    }

    /// Execute a batch of tests, automatically choosing the best strategy.
    pub fn execute(&self, tests: &[TestItem]) -> Vec<TestResult> {
        match select_strategy(tests.len()) {
            ExecutionStrategy::InProcess => self.inprocess.execute(tests),
            ExecutionStrategy::Subprocess => self.subprocess.execute(tests),
        }
    }

    /// Execute tests with a callback invoked after each test completes.
    ///
    /// Enables live progress bars and streaming output.
    pub fn execute_streaming(
        &self,
        tests: &[TestItem],
        on_result: &(dyn Fn(&TestResult) + Send + Sync),
    ) -> Vec<TestResult> {
        match select_strategy(tests.len()) {
            ExecutionStrategy::InProcess => self.inprocess.execute_with_callback(tests, on_result),
            ExecutionStrategy::Subprocess => {
                self.subprocess.execute_with_callback(tests, on_result)
            }
        }
    }

    /// Access the underlying in-process executor.
    pub fn inprocess(&self) -> &InProcessExecutor {
        &self.inprocess
    }

    /// Access the underlying subprocess pool.
    pub fn subprocess(&self) -> &SubprocessPool {
        &self.subprocess
    }
}

impl Default for HybridExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_selection() {
        // At or below threshold -> InProcess
        assert_eq!(select_strategy(0), ExecutionStrategy::InProcess);
        assert_eq!(select_strategy(1), ExecutionStrategy::InProcess);
        assert_eq!(select_strategy(10), ExecutionStrategy::InProcess);
        assert_eq!(select_strategy(20), ExecutionStrategy::InProcess);

        // Above threshold -> Subprocess
        assert_eq!(select_strategy(21), ExecutionStrategy::Subprocess);
        assert_eq!(select_strategy(100), ExecutionStrategy::Subprocess);
        assert_eq!(select_strategy(1000), ExecutionStrategy::Subprocess);
    }

    #[test]
    fn test_hybrid_executor_strategy() {
        let executor = HybridExecutor::new();
        assert_eq!(executor.strategy_for(5), ExecutionStrategy::InProcess);
        assert_eq!(executor.strategy_for(20), ExecutionStrategy::InProcess);
        assert_eq!(executor.strategy_for(21), ExecutionStrategy::Subprocess);
    }

    #[test]
    fn test_hybrid_executor_empty_tests() {
        let executor = HybridExecutor::new();
        let results = executor.execute(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_inprocess_threshold_constant() {
        assert_eq!(INPROCESS_THRESHOLD, 20);
    }
}
