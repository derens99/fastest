//! Pytest Compatibility Plugins
//!
//! This module provides compatibility layers for popular pytest plugins.

pub mod pytest_mock;
pub mod pytest_cov;

pub use pytest_mock::MockPlugin;
pub use pytest_cov::CoveragePlugin;