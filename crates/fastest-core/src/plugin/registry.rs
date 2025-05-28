//! Smart Plugin Registry using external libraries

use anyhow::Result;
use inventory;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

use super::Plugin;

/// Plugin information for inventory-based discovery
pub struct PluginInfo {
    pub name: &'static str,
    pub create_fn: fn() -> Result<Box<dyn Plugin>>,
    pub builtin: bool,
}

impl PluginInfo {
    pub const fn new(name: &'static str, create_fn: fn() -> Result<Box<dyn Plugin>>) -> Self {
        Self {
            name,
            create_fn,
            builtin: true,
        }
    }

    pub fn create_instance(&self) -> Result<Box<dyn Plugin>> {
        (self.create_fn)()
    }
}

inventory::collect!(PluginInfo);

/// Fast plugin registry using zero-cost discovery
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Discover all plugins using inventory (compile-time)
    pub fn discover_builtin_plugins() -> Vec<&'static PluginInfo> {
        inventory::iter::<PluginInfo>.into_iter().collect()
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();
        self.plugins.insert(name, Arc::from(plugin));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.get(name).cloned()
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    pub fn list_names(&self) -> Vec<&String> {
        self.plugins.keys().collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
