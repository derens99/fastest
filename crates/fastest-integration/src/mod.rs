//! Integration modules for external tools and systems

pub mod plugin;
pub mod ide;
pub mod dev_tools;
pub mod compatibility;

// Re-export common IDE integration types
pub use ide::{IdeTestItem, SimpleIdeIntegration, TestStatus};

// Re-export plugin types  
pub use plugin::{Plugin, PluginManager};

// Re-export compatibility types
pub use compatibility::{PluginCompatibilityConfig, PluginCompatibilityManager};

// Re-export dev tools
pub use dev_tools::{DevExperienceConfig, DevExperienceManager};
