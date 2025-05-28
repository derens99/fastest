pub mod advanced;
pub mod assertions;
pub mod cache;
pub mod config;
pub mod coverage;
pub mod debug;
pub mod developer_experience;
pub mod discovery;
pub mod enhanced_reporter;
pub mod error;
pub mod executor;
pub mod fixtures;
pub mod ide;
pub mod incremental;
pub mod markers;
pub mod parametrize;
pub mod parser;
pub mod plugin;
pub mod plugin_compatibility;
pub mod reporter;
pub mod timeout;
pub mod utils;
pub mod watch;

pub use cache::{default_cache_path, DiscoveryCache};
pub use discovery::{
    discover_tests, discover_tests_and_fixtures, discover_tests_ast, discover_tests_cached,
    DiscoveryResult, TestItem,
};
pub use error::{Error, Result};
pub use parser::{
    Parser, FixtureDefinition, TestFunction,
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
    OptimizedExecutor, SimpleExecutor, TestResult, UltraFastExecutor,
    PythonRuntime, RuntimeConfig, CaptureManager, CaptureConfig, CaptureResult, ExceptionInfo
};

// Re-export parametrize module
pub use parametrize::{expand_parametrized_tests, parse_parametrize_decorator};

// Re-export plugin types
pub use plugin::{Plugin, PluginManager};

// Re-export incremental types
pub use incremental::{DependencyTracker, IncrementalTestRunner};

// Re-export coverage types
pub use coverage::{CoverageFormat, CoverageReport, CoverageRunner};

// Re-export assertion types
pub use assertions::{AssertionConfig, AssertionRewriter, AssertionHelpers, format_assertion_error};

// Re-export advanced features (Phase 3)
pub use advanced::phase3::{Phase3Manager, Phase3Config};

// Re-export Developer Experience features
pub use debug::{DebugManager, DebugConfig, EnhancedError};
pub use ide::{SimpleIdeIntegration, IdeTestItem, TestStatus as IdeTestStatus};
pub use enhanced_reporter::{EnhancedReporter, ReporterConfig, FailureReport};
pub use timeout::{TimeoutManager, TimeoutConfig, AsyncTestResult};
pub use developer_experience::{DevExperienceManager, DevExperienceConfig, EnhancedTestResult, parse_dev_args};

// Re-export Plugin Compatibility features (Phase 5A)
pub use plugin_compatibility::{
    PluginCompatibilityManager, PluginCompatibilityConfig, XdistManager, CoverageManager, 
    MockManager, AsyncioManager, parse_plugin_args
};

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
