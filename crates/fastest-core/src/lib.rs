pub mod cache;
pub mod config;
pub mod discovery;
pub mod error;
pub mod executor;
pub mod fixtures;
pub mod markers;
pub mod parser;
pub mod parametrize;
pub mod utils;

pub use cache::{default_cache_path, DiscoveryCache};
pub use discovery::{
    discover_tests, discover_tests_and_fixtures, discover_tests_ast, discover_tests_cached,
    DiscoveryResult, TestItem,
};
pub use error::{Error, Result};
pub use parser::{
    parse_fixtures_and_tests, parse_test_file, AstParser, FixtureDefinition, TestFunction,
};

// Re-export config types
pub use config::Config;

// Re-export fixture types
pub use fixtures::{
    extract_fixture_deps, generate_builtin_fixture_code, generate_test_code_with_fixtures,
    is_builtin_fixture, Fixture, FixtureExecutor, FixtureManager, FixtureScope,
};

// Re-export marker types
pub use markers::{filter_by_markers, BuiltinMarker, Marker, MarkerExpr};

// Re-export from executor module
pub use executor::{
    run_test, BatchExecutor, ParallelExecutor, ProcessPool, ProgressReporter, TestResult,
};

// Re-export parametrize module
pub use parametrize::{expand_parametrized_tests, parse_parametrize_decorator};

// Version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
