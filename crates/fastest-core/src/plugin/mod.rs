//! Plugin system for Fastest test runner
//!
//! This module provides a hook-based plugin architecture similar to pytest,
//! allowing external packages and conftest.py files to extend Fastest's functionality.

pub mod hooks;
pub mod manager;
pub mod registry;
pub mod conftest;

use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub use hooks::{Hook, HookCaller, HookResult};
pub use manager::PluginManager;
pub use registry::HookRegistry;
pub use conftest::ConftestLoader;

/// Represents a loaded plugin
#[derive(Debug, Clone)]
pub struct Plugin {
    /// Unique name of the plugin
    pub name: String,
    /// Version of the plugin
    pub version: Option<String>,
    /// Description of the plugin
    pub description: Option<String>,
    /// Source of the plugin (package name or conftest.py path)
    pub source: PluginSource,
    /// Whether the plugin is enabled
    pub enabled: bool,
}

/// Source of a plugin
#[derive(Debug, Clone, PartialEq)]
pub enum PluginSource {
    /// Plugin loaded from an installed package
    Package(String),
    /// Plugin loaded from a conftest.py file
    Conftest(PathBuf),
    /// Built-in plugin
    Builtin,
}

/// Configuration passed to plugins during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Test paths being used
    pub test_paths: Vec<PathBuf>,
    /// Command line arguments
    pub args: HashMap<String, String>,
    /// Additional configuration options
    pub options: HashMap<String, serde_json::Value>,
}

/// Trait that all plugins must implement
pub trait FastestPlugin: Any + Send + Sync {
    /// Get the name of the plugin
    fn name(&self) -> &str;
    
    /// Get the version of the plugin
    fn version(&self) -> Option<&str> {
        None
    }
    
    /// Get the description of the plugin
    fn description(&self) -> Option<&str> {
        None
    }
    
    /// Register hooks with the registry
    fn register_hooks(&self, registry: &mut HookRegistry);
    
    /// Initialize the plugin with configuration
    fn initialize(&mut self, config: &PluginConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Default implementation does nothing
        Ok(())
    }
    
    /// Clean up resources when plugin is unloaded
    fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Default implementation does nothing
        Ok(())
    }
}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Errors that can occur in the plugin system
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    
    #[error("Plugin already registered: {0}")]
    PluginAlreadyRegistered(String),
    
    #[error("Failed to load plugin: {0}")]
    LoadError(String),
    
    #[error("Hook execution failed: {0}")]
    HookError(String),
    
    #[error("Invalid plugin configuration: {0}")]
    ConfigError(String),
    
    #[error("Plugin initialization failed: {0}")]
    InitializationError(String),
    
    #[error("Conftest parse error: {0}")]
    ConftestError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Macro to help define plugins
#[macro_export]
macro_rules! define_plugin {
    ($name:ident, $struct_name:ident) => {
        pub struct $struct_name;
        
        impl $crate::plugin::FastestPlugin for $struct_name {
            fn name(&self) -> &str {
                stringify!($name)
            }
            
            fn register_hooks(&self, registry: &mut $crate::plugin::HookRegistry) {
                // Plugin implementation registers its hooks here
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_source() {
        let package_source = PluginSource::Package("pytest-mock".to_string());
        assert!(matches!(package_source, PluginSource::Package(_)));
        
        let conftest_source = PluginSource::Conftest(PathBuf::from("conftest.py"));
        assert!(matches!(conftest_source, PluginSource::Conftest(_)));
    }
    
    #[test]
    fn test_plugin_config() {
        let config = PluginConfig {
            test_paths: vec![PathBuf::from("tests")],
            args: HashMap::new(),
            options: HashMap::new(),
        };
        
        assert_eq!(config.test_paths.len(), 1);
    }
}