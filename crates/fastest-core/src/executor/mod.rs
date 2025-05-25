pub mod parallel;
pub mod single;
pub mod batch;
pub mod process_pool;

use std::time::Duration;

/// Result of running a test
#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_id: String,
    pub passed: bool,
    pub duration: Duration,
    pub output: String,
    pub error: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

pub use parallel::{ParallelExecutor, ProgressReporter};
pub use single::run_test;
pub use batch::BatchExecutor;
pub use process_pool::ProcessPool; 