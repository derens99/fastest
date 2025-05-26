pub mod cache;
pub mod config;
pub mod discovery;
pub mod error;
pub mod executor;
pub mod fixtures;
pub mod incremental;
pub mod markers;
pub mod parser;
pub mod parametrize;
pub mod plugin;
pub mod reporter;
pub mod utils;
pub mod watch;
pub mod coverage;
pub mod assertions;

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

// Re-export plugin types
pub use plugin::{Plugin, PluginManager};

// Re-export incremental types
pub use incremental::{DependencyTracker, IncrementalTestRunner};

// Re-export coverage types  
pub use coverage::{CoverageRunner, CoverageReport, CoverageFormat};

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
