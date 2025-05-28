pub mod ultra_fast;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::discovery::TestItem;
use crate::error::Result;
use std::path::PathBuf;

/// Result of running a test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub passed: bool,
    pub duration: Duration,
    pub output: String,
    pub error: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

pub use ultra_fast::UltraFastExecutor;

// Legacy compatibility wrappers

/// OptimizedExecutor - wrapper for backwards compatibility
pub struct OptimizedExecutor(UltraFastExecutor);

impl OptimizedExecutor {
    pub fn new(_num_workers: Option<usize>, verbose: bool) -> Self {
        Self(UltraFastExecutor::new(verbose))
    }
    
    pub fn with_coverage(self, _source_dirs: Vec<PathBuf>) -> Self {
        Self(self.0.with_coverage(_source_dirs))
    }
    
    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
    
    pub fn execute_with_fixtures(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
    
    pub fn execute_with_cache(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

/// OptimizedExecutorV2 - alias for OptimizedExecutor
pub type OptimizedExecutorV2 = OptimizedExecutor;

/// SimpleExecutor - wrapper for backwards compatibility
pub struct SimpleExecutor(UltraFastExecutor);

impl SimpleExecutor {
    pub fn new(verbose: bool) -> Self {
        Self(UltraFastExecutor::new(verbose))
    }
    
    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

/// BatchExecutor - wrapper for backwards compatibility
pub struct BatchExecutor(UltraFastExecutor);

impl BatchExecutor {
    pub fn new() -> Self {
        Self(UltraFastExecutor::new(false))
    }
    
    pub fn execute_tests(&self, tests: Vec<TestItem>) -> Vec<TestResult> {
        self.0.execute_tests(tests)
    }
}

impl Default for BatchExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// ParallelExecutor - wrapper for backwards compatibility
pub struct ParallelExecutor(UltraFastExecutor);

impl ParallelExecutor {
    pub fn new(_num_workers: Option<usize>, verbose: bool) -> Self {
        Self(UltraFastExecutor::new(verbose))
    }
    
    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

/// LightningExecutor - wrapper for backwards compatibility
pub struct LightningExecutor(UltraFastExecutor);

impl LightningExecutor {
    pub fn new(verbose: bool) -> Self {
        Self(UltraFastExecutor::new(verbose))
    }
    
    pub fn execute(&self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

// Stub for ProgressReporter to maintain compatibility
pub trait ProgressReporter: Send + Sync {
    fn report_progress(&self, _completed: usize, _total: usize) {}
}
