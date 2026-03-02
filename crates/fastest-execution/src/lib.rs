//! Hybrid test execution engine for Fastest.
//!
//! Provides two execution backends:
//! - **In-process** (PyO3): runs tests directly in the current process for low overhead.
//! - **Subprocess pool**: spawns persistent Python workers for isolation and parallelism.
//!
//! The [`HybridExecutor`] automatically selects the appropriate backend based on test count.

pub mod capture;
pub mod executor;
pub mod inprocess;
pub mod subprocess;
pub mod timeout;

pub use executor::HybridExecutor;
pub use fastest_core::model::{TestItem, TestOutcome, TestResult};
