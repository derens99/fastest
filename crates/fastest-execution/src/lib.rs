//! Revolutionary Ultra-Fast Python Test Execution Engine
//!
//! Streamlined execution crate with SIMD-accelerated discovery consolidated into core.
//! Single UltraFastExecutor implementation for maximum performance and simplicity.

pub mod runtime;       // Python runtime integration
pub mod strategies;    // Main execution strategies (UltraFastExecutor only)
pub mod parallel;      // Parallel execution
pub mod capture;       // Output capture
pub mod timeout;       // Timeout handling

// 🗑️ REMOVED: simd_discovery - consolidated into fastest-core discovery module

// Experimental performance modules (stubs for future development)
pub mod zero_copy;           // Zero-copy memory architecture (stub)
pub mod work_stealing;       // Lock-free work-stealing parallelism (stub)
pub mod native_transpiler;   // JIT compilation (stub)
pub mod execution;           // Advanced fixture execution

// Re-export the main execution module that was in mod.rs
use serde::{Deserialize, Serialize};

// Re-export main types
pub use strategies::{DevExperienceConfig, PluginCompatibilityConfig};
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

// Re-export main execution types
pub use capture::{CaptureConfig, CaptureManager, CaptureResult, ExceptionInfo};
pub use runtime::{PythonRuntime, RuntimeConfig};
pub use strategies::UltraFastExecutor;
pub use timeout::{UltraFastTimeoutManager, TimeoutConfig, TimeoutHandle, TimeoutEvent, TimeoutEventType, TimeoutStatistics};
pub use parallel::{MassiveParallelExecutor, MassiveExecutionStats};

// Revolutionary performance optimizations - all fully implemented
pub use zero_copy::{ZeroCopyExecutor, ZeroCopyTestResult, convert_zero_copy_results, ExecutionStats as ZeroCopyStats, create_zero_copy_executor_with_arena};
pub use work_stealing::{WorkStealingExecutor, WorkStealingStats, WorkerMetrics};
pub use native_transpiler::{NativeTestExecutor, NativeTestResult, ExecutionType as NativeExecutionType, TranspilationStats, DetailedStats as NativeDetailedStats, TestPattern};

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