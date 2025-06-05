//! Infrastructure and supporting systems
//!
//! This module contains essential supporting systems for test execution including
//! parallel execution, output capture, and timeout handling.

pub mod parallel;
pub mod capture; 
pub mod timeout;
pub mod fixtures;
pub mod fixture_manager;

// Re-export main types from this module
pub use parallel::{MassiveParallelExecutor, MassiveExecutionStats};
pub use capture::{CaptureConfig, CaptureManager, CaptureResult, ExceptionInfo};
pub use timeout::{
    UltraFastTimeoutManager, TimeoutConfig, TimeoutHandle, 
    TimeoutEvent, TimeoutEventType, TimeoutStatistics
};
pub use fixtures::FixtureExecutor;
pub use fixture_manager::CompleteFixtureManager;