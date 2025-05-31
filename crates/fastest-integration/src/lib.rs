//! IDE integration and developer tools
//!
//! This crate provides IDE integration capabilities and enhanced developer experience features.

pub mod ide;
pub mod dev_tools;
pub mod compatibility;

// Re-export main types
pub use ide::{IdeTestItem, SimpleIdeIntegration, TestStatus};
pub use dev_tools::{
    parse_dev_args, DevExperienceConfig, DevExperienceManager, EnhancedTestResult,
};
pub use compatibility::{
    parse_plugin_args, AsyncioManager, CoverageManager, MockManager, 
    PluginCompatibilityConfig, PluginCompatibilityManager, XdistManager,
};