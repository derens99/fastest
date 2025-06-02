//! Plugin system for Fastest test runner
//!
//! This crate provides a comprehensive plugin system with pytest compatibility,
//! dynamic loading, and hook management.

pub mod builtin;
pub mod conftest;
pub mod hooks;
pub mod manager;
pub mod registry;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// Re-export main types
pub use conftest::ConftestLoader;
pub use hooks::{HookRegistry, HookResult, PytestHooks};
pub use manager::PluginManager;
pub use registry::{PluginInfo, PluginRegistry};

/// Smart plugin loader that handles both Rust and Python plugins
pub struct SmartPluginLoader {
    #[allow(dead_code)]
    config: PluginConfig,
    #[allow(dead_code)]
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
        // Stub implementation for now
        Ok(vec![])
    }
}

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
    pub session_data: HashMap<String, String>,
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
    pub enabled: bool,
    pub plugin_dirs: Vec<PathBuf>,
    pub pytest_plugin_compatibility: bool,
    pub load_conftest: bool,
    pub plugin_timeout_ms: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            plugin_dirs: vec![],
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
            $crate::PluginInfo::new(stringify!($plugin), $plugin)
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