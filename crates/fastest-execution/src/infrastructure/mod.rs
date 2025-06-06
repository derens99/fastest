//! Infrastructure and supporting systems
//!
//! This module contains essential supporting systems for test execution including
//! parallel execution, output capture, and timeout handling.

pub mod capture;
pub mod fixture_manager;
pub mod fixtures;
pub mod parallel;
pub mod timeout;

// Re-export main types from this module
pub use capture::{CaptureConfig, CaptureManager, CaptureResult, ExceptionInfo};
pub use fixture_manager::CompleteFixtureManager;
pub use fixtures::FixtureExecutor;
pub use parallel::{MassiveExecutionStats, MassiveParallelExecutor};
pub use timeout::{
    TimeoutConfig, TimeoutEvent, TimeoutEventType, TimeoutHandle, TimeoutStatistics,
    UltraFastTimeoutManager,
};
