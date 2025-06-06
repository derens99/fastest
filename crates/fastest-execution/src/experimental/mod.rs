//! Experimental performance optimizations
//!
//! This module contains cutting-edge performance optimizations including zero-copy
//! memory management, work-stealing algorithms, and JIT compilation. These features
//! are experimental and may be subject to change.

pub mod native_transpiler;
pub mod work_stealing;
pub mod zero_copy;

// Re-export main types from this module
pub use native_transpiler::{
    DetailedStats as NativeDetailedStats, ExecutionType as NativeExecutionType, NativeTestExecutor,
    NativeTestResult, TestPattern, TranspilationStats,
};
pub use work_stealing::{WorkStealingExecutor, WorkStealingStats, WorkerMetrics};
pub use zero_copy::{
    convert_zero_copy_results, create_zero_copy_executor_with_arena,
    ExecutionStats as ZeroCopyStats, ZeroCopyExecutor, ZeroCopyTestResult,
};
