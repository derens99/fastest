//! Experimental performance optimizations
//!
//! This module contains cutting-edge performance optimizations including zero-copy
//! memory management, work-stealing algorithms, and JIT compilation. These features
//! are experimental and may be subject to change.

pub mod zero_copy;
pub mod work_stealing;
pub mod native_transpiler;

// Re-export main types from this module
pub use zero_copy::{
    ZeroCopyExecutor, ZeroCopyTestResult, convert_zero_copy_results,
    ExecutionStats as ZeroCopyStats, create_zero_copy_executor_with_arena
};
pub use work_stealing::{WorkStealingExecutor, WorkStealingStats, WorkerMetrics};
pub use native_transpiler::{
    NativeTestExecutor, NativeTestResult, ExecutionType as NativeExecutionType,
    TranspilationStats, DetailedStats as NativeDetailedStats, TestPattern
};