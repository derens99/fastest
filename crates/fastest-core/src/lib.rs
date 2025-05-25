pub mod config;
pub mod discovery;
pub mod error;
pub mod parser;
pub mod cache;
pub mod executor;

pub use config::TestConfig;
pub use discovery::{discover_tests, discover_tests_cached, discover_tests_ast, TestItem};
pub use error::{Error, Result};
pub use parser::{parse_test_file, TestFunction, AstParser};
pub use cache::{DiscoveryCache, default_cache_path};

// Re-export from executor module
pub use executor::{
    TestResult,
    run_test,
    BatchExecutor,
    ParallelExecutor,
    ProgressReporter,
    ProcessPool,
};