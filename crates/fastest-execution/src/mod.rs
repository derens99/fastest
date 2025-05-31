pub mod capture;
pub mod runtime;      // Renamed from python_runtime
pub mod strategies;   // Renamed from ultra_fast
pub mod parallel;     // Renamed from massive_parallel
pub mod timeout;

// Experimental performance modules
pub mod zero_copy;           // Zero-copy memory architecture
pub mod work_stealing;       // Lock-free work-stealing parallelism  
pub mod native_transpiler;   // JIT compilation of Python tests to native code

use fastest_core::TestItem;
use fastest_core::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

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

pub use capture::{CaptureConfig, CaptureManager, CaptureResult, ExceptionInfo};
pub use runtime::{PythonRuntime, RuntimeConfig};

// Main execution strategy
pub use strategies::UltraFastExecutor;

// Parallel execution
pub use parallel::{MassiveParallelExecutor, MassiveExecutionStats};

// Timeout handling
pub use timeout::{AsyncTestResult, TimeoutConfig, TimeoutManager};

// Experimental performance optimizations
pub use zero_copy::{ZeroCopyExecutor, ZeroCopyTestResult, convert_zero_copy_results};
pub use work_stealing::{WorkStealingExecutor, WorkStealingStats};
pub use native_transpiler::{NativeTestExecutor, NativeTestResult, ExecutionType as NativeExecutionType, TranspilationStats};

// Legacy compatibility wrappers
// TODO: Consider removing these if no longer needed - they all delegate to UltraFastExecutor

/// OptimizedExecutor - wrapper for backwards compatibility
pub struct OptimizedExecutor(UltraFastExecutor);

impl OptimizedExecutor {
    pub fn new(_num_workers: Option<usize>, verbose: bool) -> Self {
        Self(UltraFastExecutor::new(verbose).expect("Failed to create revolutionary executor"))
    }

    pub fn with_coverage(self, _source_dirs: Vec<PathBuf>) -> Self {
        Self(self.0.with_coverage(_source_dirs))
    }

    pub fn execute(&mut self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }

    pub fn execute_with_fixtures(&mut self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }

    pub fn execute_with_cache(&mut self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

/// OptimizedExecutorV2 - alias for OptimizedExecutor
pub type OptimizedExecutorV2 = OptimizedExecutor;

/// SimpleExecutor - wrapper for backwards compatibility
pub struct SimpleExecutor(UltraFastExecutor);

impl SimpleExecutor {
    pub fn new(verbose: bool) -> Self {
        Self(UltraFastExecutor::new(verbose).expect("Failed to create revolutionary executor"))
    }

    pub fn execute(&mut self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

/// BatchExecutor - wrapper for backwards compatibility
pub struct BatchExecutor(UltraFastExecutor);

impl BatchExecutor {
    pub fn new() -> Self {
        Self(UltraFastExecutor::new(false).expect("Failed to create revolutionary executor"))
    }

    pub fn execute_tests(&mut self, tests: Vec<TestItem>) -> Vec<TestResult> {
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
        Self(UltraFastExecutor::new(verbose).expect("Failed to create revolutionary executor"))
    }

    pub fn execute(&mut self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

/// LightningExecutor - wrapper for backwards compatibility
pub struct LightningExecutor(UltraFastExecutor);

impl LightningExecutor {
    pub fn new(verbose: bool) -> Self {
        Self(UltraFastExecutor::new(verbose).expect("Failed to create revolutionary executor"))
    }

    pub fn execute(&mut self, tests: Vec<TestItem>) -> Result<Vec<TestResult>> {
        self.0.execute(tests)
    }
}

// Stub for ProgressReporter to maintain compatibility
pub trait ProgressReporter: Send + Sync {
    fn report_progress(&self, _completed: usize, _total: usize) {}
}
