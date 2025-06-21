//! Core types and test discovery for Fastest test runner
//!
//! High-performance foundation providing:
//! - Fast parallel test discovery with caching
//! - Optimized Python parsing with tree-sitter
//! - Efficient fixture management with dependency resolution
//! - Smart caching with compression and memory mapping
//! - Minimal allocations and lock-free data structures

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

// Plugin support
pub mod plugin;

// Re-export primary types for convenience
pub use cache::{default_cache_path, DiscoveryCache};
pub use config::Config;
pub use error::{Error, Result};

// Test discovery exports
pub use test::discovery::{
    discover_tests, 
    discover_tests_with_filtering, 
    TestItem,
    TestMetadata,
};

// Parser exports
pub use test::parser::{
    Parser,
    TestFunction,
    FixtureDefinition as ParserFixtureDefinition,
    SetupTeardownMethod,
    SetupTeardownType,
    SetupTeardownScope,
    ModuleMetadata,
    ClassMetadata,
};

// Advanced fixture exports
pub use test::fixtures::advanced::{
    FixtureDefinition,
    FixtureScope,
    FixtureRequest,
    FixtureValue,
    AdvancedFixtureManager,
};

// Basic fixture exports (for compatibility)
pub use test::fixtures::{
    extract_fixture_deps,
    generate_builtin_fixture_code,
    generate_test_code_with_fixtures,
    is_builtin_fixture,
    Fixture,
    FixtureExecutor,
    FixtureManager,
};

// Marker exports
pub use test::markers::{
    filter_by_markers,
    BuiltinMarker,
    Marker,
    MarkerExpr,
};

// Parametrize exports
pub use test::parametrize::{
    expand_parametrized_tests,
    parse_parametrize_decorator,
};

// Debug exports
pub use debug::{
    DebugConfig,
    DebugManager,
    EnhancedError,
};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Performance statistics for monitoring
#[derive(Debug, Default)]
pub struct PerformanceStats {
    pub discovery_time_ms: u64,
    pub parse_time_ms: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub tests_discovered: u64,
    pub files_processed: u64,
}

/// Initialize fastest-core with optimal settings
pub fn initialize() -> Result<()> {
    // Set up SIMD if available
    utils::simd_json::initialize_simd();
    
    // Configure rayon thread pool
    if let Err(e) = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .thread_name(|i| format!("fastest-worker-{}", i))
        .build_global()
    {
        eprintln!("Warning: Failed to configure thread pool: {}", e);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.split('.').count() >= 2);
    }
    
    #[test]
    fn test_initialize() {
        assert!(initialize().is_ok());
    }
}