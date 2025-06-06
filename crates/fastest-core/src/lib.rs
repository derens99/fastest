//! Core types and test discovery for Fastest test runner
//!
//! This crate provides the fundamental building blocks for the Fastest test runner:
//! - Test discovery and parsing
//! - Basic types and configuration
//! - Caching infrastructure
//! - Error handling

// Core modules
pub mod cache;
pub mod config;
pub mod debug;
pub mod error;
pub mod utils;

// Test-related functionality
pub mod test {
    pub mod discovery;
    pub mod fixtures;
    pub mod markers;
    pub mod parametrize;
    pub mod parser;
}

// Re-export core types
pub use cache::{default_cache_path, DiscoveryCache};
pub use config::Config;
pub use error::{Error, Result};
pub use test::discovery::{discover_tests, discover_tests_with_filtering, TestItem};
pub use test::parser::{FixtureDefinition, Parser, TestFunction};

// Re-export fixture types
pub use test::fixtures::{
    extract_fixture_deps, generate_builtin_fixture_code, generate_test_code_with_fixtures,
    is_builtin_fixture, Fixture, FixtureExecutor, FixtureManager, FixtureScope,
};

// Re-export marker types
pub use test::markers::{filter_by_markers, BuiltinMarker, Marker, MarkerExpr};

// Re-export parametrize module
pub use test::parametrize::{expand_parametrized_tests, parse_parametrize_decorator};

// Re-export debug types
pub use debug::{DebugConfig, DebugManager, EnhancedError};

// Version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        // Just verify VERSION is accessible and contains semantic version pattern
        assert!(VERSION.contains('.'));
    }
}
