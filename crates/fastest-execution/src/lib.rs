//! Revolutionary Ultra-Fast Python Test Execution Engine
//!
//! Streamlined execution crate with SIMD-accelerated discovery consolidated into core.
//! Single UltraFastExecutor implementation for maximum performance and simplicity.
//!
//! ## Architecture
//!
//! The crate is organized into three main modules:
//!
//! - **`core`**: Core execution functionality including strategies, runtime, and fixture execution
//! - **`infrastructure`**: Supporting systems for parallel execution, output capture, and timeouts  
//! - **`experimental`**: Cutting-edge optimizations including zero-copy, work-stealing, and JIT compilation

pub mod core; // Core execution functionality
pub mod error; // Error types for execution
pub mod experimental; // Experimental optimizations
pub mod infrastructure; // Supporting systems
pub mod utils; // Utility modules including SIMD optimizations

// Re-export the main execution module that was in mod.rs
use serde::{Deserialize, Serialize};

use std::time::Duration;

/// Outcome of a test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
    XFailed { reason: Option<String> },
    XPassed, // Expected to fail but passed
}

/// Result of running a test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub outcome: TestOutcome,
    pub duration: Duration,
    pub output: String,
    pub error: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

impl TestResult {
    /// Helper method for backward compatibility
    pub fn passed(&self) -> bool {
        matches!(
            self.outcome,
            TestOutcome::Passed | TestOutcome::XFailed { .. }
        )
    }
}

// Re-export main types from organized modules
pub use core::{
    PythonRuntime, RuntimeConfig, UltraFastExecutor,
};
pub use error::{ExecutionError, Result};
pub use experimental::{
    convert_zero_copy_results, create_zero_copy_executor_with_arena, NativeDetailedStats,
    NativeExecutionType, NativeTestExecutor, NativeTestResult, TestPattern, TranspilationStats,
    WorkStealingExecutor, WorkStealingStats, WorkerMetrics, ZeroCopyExecutor, ZeroCopyStats,
    ZeroCopyTestResult,
};
pub use infrastructure::{
    CaptureConfig, CaptureManager, CaptureResult, ExceptionInfo, MassiveExecutionStats,
    MassiveParallelExecutor, TimeoutConfig, TimeoutEvent, TimeoutEventType, TimeoutHandle,
    TimeoutStatistics, UltraFastTimeoutManager,
};
pub use utils::{
    benchmark_json_performance, init_simd_json, init_simd_json_with_config, is_simd_json_available,
    SimdJsonConfig, SimdJsonStats,
};

// 🧹 REMOVED: Legacy executor wrappers eliminated for cleaner architecture
// All execution now uses UltraFastExecutor directly for maximum performance

// 🎯 CONSOLIDATED: Use UltraFastExecutor directly instead of wrapper types
//
// Migration guide:
// - OptimizedExecutor::new(workers, verbose) -> UltraFastExecutor::new(verbose)
// - SimpleExecutor::new(verbose) -> UltraFastExecutor::new(verbose)
// - BatchExecutor::new() -> UltraFastExecutor::new(false)
// - ParallelExecutor::new(...) -> UltraFastExecutor::new(verbose)
// - All .execute() methods work the same on UltraFastExecutor

// ✅ LEGACY EXECUTORS REMOVED: Use UltraFastExecutor directly
// This eliminates ~200 lines of redundant wrapper code

// Progress reporter trait
pub trait ProgressReporter: Send + Sync {
    fn report_progress(&self, _completed: usize, _total: usize) {}
}
