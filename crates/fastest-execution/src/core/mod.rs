//! Core execution functionality
//!
//! This module contains the primary execution strategies, Python runtime integration,
//! and advanced fixture execution systems that form the heart of the fastest execution engine.

pub mod strategies;
pub mod runtime;
pub mod execution;
pub mod fixture_integration;
pub mod lifecycle;

// Re-export main types from this module
pub use strategies::{UltraFastExecutor, DevExperienceConfig, PluginCompatibilityConfig};
pub use runtime::{PythonRuntime, RuntimeConfig};
pub use fixture_integration::generate_fixture_aware_worker_code;