pub mod cache;
pub mod config;
pub mod discovery;
pub mod error;
pub mod executor;
pub mod fixtures;
pub mod markers;
pub mod parser;
pub mod utils;

pub use discovery::{discover_tests, discover_tests_cached, discover_tests_ast, discover_tests_and_fixtures, TestItem, DiscoveryResult};
pub use error::{Error, Result};
pub use parser::{parse_test_file, parse_fixtures_and_tests, TestFunction, FixtureDefinition, AstParser};
pub use cache::{DiscoveryCache, default_cache_path};

// Re-export config types
pub use config::Config;

// Re-export fixture types
pub use fixtures::{
    Fixture, FixtureManager, FixtureScope, extract_fixture_deps,
    FixtureExecutor, generate_test_code_with_fixtures,
    is_builtin_fixture, generate_builtin_fixture_code
};

// Re-export marker types
pub use markers::{Marker, BuiltinMarker, MarkerExpr, filter_by_markers};

// Re-export from executor module
pub use executor::{
    TestResult,
    run_test,
    BatchExecutor,
    ParallelExecutor,
    ProgressReporter,
    ProcessPool,
};