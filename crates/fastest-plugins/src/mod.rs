//! Smart Plugin System for pytest Compatibility
//!
//! This module provides a minimal, fast plugin system that's compatible with pytest
//! while leveraging external libraries for simplicity and performance.

pub mod builtin;
pub mod conftest;
pub mod hooks;
pub mod manager;
pub mod registry;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub use conftest::ConftestLoader;
pub use hooks::{HookRegistry, HookResult, PytestHooks};
pub use manager::PluginManager;
pub use registry::{PluginInfo, PluginRegistry};

/// Plugin trait for maximum simplicity and performance
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn pytest_compatible(&self) -> bool {
        true
    }
    fn register_hooks(&self, registry: &mut HookRegistry) -> Result<()>;
}

/// Plugin context for hook execution
#[derive(Debug)]
pub struct PluginContext {
    pub config: HashMap<String, serde_json::Value>,
    pub session_data: HashMap<String, String>, // Simplified to string values
    pub test_path: Option<PathBuf>,
    pub current_test: Option<String>,
}

/// Hook call data passed between plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookData {
    pub hook_name: String,
    pub args: serde_json::Value,
    pub context: HashMap<String, serde_json::Value>,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub enabled_plugins: Vec<String>,
    pub disabled_plugins: Vec<String>,
    pub plugin_search_paths: Vec<PathBuf>,
    pub pytest_plugin_compatibility: bool,
    pub load_conftest: bool,
    pub plugin_timeout_ms: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled_plugins: vec![
                "markers".to_string(),
                "parametrize".to_string(),
                "capture".to_string(),
                "fixtures".to_string(),
            ],
            disabled_plugins: vec![],
            plugin_search_paths: vec![],
            pytest_plugin_compatibility: true,
            load_conftest: true,
            plugin_timeout_ms: 5000,
        }
    }
}

/// Built-in plugin registration using inventory for zero-cost discovery
#[macro_export]
macro_rules! register_plugin {
    ($plugin:expr) => {
        inventory::submit! {
            crate::integration::plugin::PluginInfo::new(stringify!($plugin), $plugin)
        }
    };
}

/// Hook registration macro for compile-time optimization
#[macro_export]
macro_rules! define_hook {
    ($name:ident, $($param:ident: $ty:ty),*) => {
        pub fn $name(&self, $($param: $ty),*) -> Result<HookResult> {
            let data = serde_json::json!({
                $(stringify!($param): $param,)*
            });
            self.call_hook(stringify!($name), data)
        }
    };
}

/// Plugin result types for efficient processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginResult {
    Continue,
    Stop,
    Skip,
    Modified(serde_json::Value),
    Error(String),
}

/// Smart plugin loader that handles both Rust and Python plugins
pub struct SmartPluginLoader {
    config: PluginConfig,
    registry: PluginRegistry,
}

impl SmartPluginLoader {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            registry: PluginRegistry::new(),
        }
    }

    /// Load all plugins efficiently using external libraries
    pub fn load_all(&mut self) -> Result<Vec<Box<dyn Plugin>>> {
        let mut plugins = Vec::new();

        // Load built-in plugins using inventory (zero-cost discovery)
        plugins.extend(self.load_builtin_plugins()?);

        // Load conftest.py files if enabled
        if self.config.load_conftest {
            plugins.extend(self.load_conftest_plugins()?);
        }

        // Load external plugins from search paths
        plugins.extend(self.load_external_plugins()?);

        Ok(plugins)
    }

    fn load_builtin_plugins(&self) -> Result<Vec<Box<dyn Plugin>>> {
        use inventory;

        let mut plugins = Vec::new();

        // Use inventory to collect all registered plugins at compile time
        for info in inventory::iter::<PluginInfo> {
            if self.is_plugin_enabled(&info.name) {
                plugins.push(info.create_instance()?);
            }
        }

        Ok(plugins)
    }

    fn load_conftest_plugins(&self) -> Result<Vec<Box<dyn Plugin>>> {
        let conftest_loader = ConftestLoader::new();
        conftest_loader.discover_and_load(&self.config.plugin_search_paths)
    }

    fn load_external_plugins(&self) -> Result<Vec<Box<dyn Plugin>>> {
        let mut plugins = Vec::new();

        for search_path in &self.config.plugin_search_paths {
            // Use libloading for dynamic plugin loading
            plugins.extend(self.load_plugins_from_path(search_path)?);
        }

        Ok(plugins)
    }

    fn load_plugins_from_path(&self, path: &PathBuf) -> Result<Vec<Box<dyn Plugin>>> {
        use libloading::{Library, Symbol};
        use std::ffi::OsStr;

        let mut plugins = Vec::new();

        if path.is_file() && path.extension() == Some(OsStr::new("so")) {
            // Load dynamic library plugin
            unsafe {
                let lib = Library::new(path)?;
                let create_plugin: Symbol<fn() -> Box<dyn Plugin>> = lib.get(b"create_plugin")?;
                plugins.push(create_plugin());
            }
        }

        Ok(plugins)
    }

    fn is_plugin_enabled(&self, name: &str) -> bool {
        !self.config.disabled_plugins.contains(&name.to_string())
            && (self.config.enabled_plugins.is_empty()
                || self.config.enabled_plugins.contains(&name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert!(config.pytest_plugin_compatibility);
        assert!(config.load_conftest);
        assert!(!config.enabled_plugins.is_empty());
    }

    #[test]
    fn test_smart_plugin_loader() {
        let config = PluginConfig::default();
        let loader = SmartPluginLoader::new(config);

        // Should create without errors
        assert_eq!(loader.registry.count(), 0);
    }
}
