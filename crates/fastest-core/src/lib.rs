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
pub mod update;
pub mod utils;
pub mod watch;

pub use cache::{default_cache_path, DiscoveryCache};
pub use discovery::{
    discover_tests, discover_tests_and_fixtures, discover_tests_ast, discover_tests_cached,
    DiscoveryResult, TestItem,
};
pub use error::{Error, Result};
pub use parser::{FixtureDefinition, Parser, TestFunction};

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
    CaptureConfig, CaptureManager, CaptureResult, ExceptionInfo, OptimizedExecutor, PythonRuntime,
    RuntimeConfig, SimpleExecutor, TestResult, UltraFastExecutor,
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
pub use assertions::{
    format_assertion_error, AssertionConfig, AssertionHelpers, AssertionRewriter,
};

// Re-export advanced features (Phase 3)
pub use advanced::phase3::{Phase3Config, Phase3Manager};

// Re-export Developer Experience features
pub use debug::{DebugConfig, DebugManager, EnhancedError};
pub use developer_experience::{
    parse_dev_args, DevExperienceConfig, DevExperienceManager, EnhancedTestResult,
};
pub use enhanced_reporter::{EnhancedReporter, FailureReport, ReporterConfig};
pub use ide::{IdeTestItem, SimpleIdeIntegration, TestStatus as IdeTestStatus};
pub use timeout::{AsyncTestResult, TimeoutConfig, TimeoutManager};

// Re-export Plugin Compatibility features (Phase 5A)
pub use plugin_compatibility::{
    parse_plugin_args, AsyncioManager, CoverageManager, MockManager, PluginCompatibilityConfig,
    PluginCompatibilityManager, XdistManager,
};

// Re-export update types
pub use update::{check_for_updates, UpdateChecker};

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
