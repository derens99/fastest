pub mod cache;
pub mod config;
pub mod discovery;
pub mod error;
pub mod executor;
pub mod fixtures;
pub mod markers;
pub mod parser;
pub mod utils;

pub use discovery::{discover_tests, discover_tests_cached, discover_tests_ast, TestItem};
pub use error::{Error, Result};
pub use parser::{parse_test_file, TestFunction, AstParser};
pub use cache::{DiscoveryCache, default_cache_path};

// Re-export config types
pub use config::Config;

// Re-export fixture types
pub use fixtures::{Fixture, FixtureManager, FixtureScope};

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