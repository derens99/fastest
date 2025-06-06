//! Core execution functionality
//!
//! This module contains the primary execution strategies, Python runtime integration,
//! and advanced fixture execution systems that form the heart of the fastest execution engine.

pub mod execution;
pub mod fixture_integration;
pub mod lifecycle;
pub mod runtime;
pub mod strategies;

// Re-export main types from this module
pub use fixture_integration::generate_fixture_aware_worker_code;
pub use runtime::{PythonRuntime, RuntimeConfig};
pub use strategies::{DevExperienceConfig, PluginCompatibilityConfig, UltraFastExecutor};
