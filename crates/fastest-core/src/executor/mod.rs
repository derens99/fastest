pub mod batch;
pub mod lightning;
pub mod optimized;
pub mod parallel;
pub mod persistent_pool;
pub mod process_pool;
pub mod simple;
pub mod single;
pub mod ultra_fast;

use serde::{Deserialize, Serialize};
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

pub use batch::BatchExecutor;
pub use lightning::LightningExecutor;
pub use optimized::OptimizedExecutor;
pub use parallel::{ParallelExecutor, ProgressReporter};
pub use persistent_pool::PersistentWorkerPool;
pub use process_pool::ProcessPool;
pub use simple::SimpleExecutor;
pub use single::run_test;
pub use ultra_fast::UltraFastExecutor;
