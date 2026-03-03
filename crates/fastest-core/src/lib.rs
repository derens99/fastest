//! Core types, discovery, parsing, and configuration for Fastest test runner

pub mod config;
pub mod discovery;
pub mod error;
pub mod fixtures;
pub mod incremental;
pub mod markers;
pub mod model;
pub mod parametrize;
pub mod plugins;
pub mod watch;

pub use config::Config;
pub use discovery::discover_tests;
pub use error::{Error, Result};
pub use fixtures::{
    discover_conftest_fixtures, generate_builtin_code, is_builtin, resolve_fixture_order, Fixture,
    FixtureCache, FixtureScope,
};
pub use incremental::IncrementalTester;
pub use markers::{classify_marker, filter_by_keyword, filter_by_markers, BuiltinMarker};
pub use model::{TestItem, TestOutcome, TestResult};
pub use parametrize::expand_parametrized_tests;
pub use plugins::{HookArgs, HookResult, Plugin, PluginManager, PluginMetadata};
pub use watch::TestWatcher;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
