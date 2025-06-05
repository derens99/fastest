//! Minimal working plugin system implementation

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde_json::Value;

use crate::api::{Plugin, PluginMetadata, PluginError, PluginResult};

/// Simplified hook arguments
#[derive(Debug, Clone)]
pub struct HookArgs {
    args: HashMap<String, Value>,
}

impl HookArgs {
    pub fn new() -> Self {
        Self { args: HashMap::new() }
    }
    
    pub fn arg(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.args.insert(key.to_string(), value.into());
        self
    }
}

/// Simplified plugin manager
pub struct PluginManager {
    plugins: Arc<RwLock<Vec<Box<dyn Plugin>>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn plugins(&self) -> Vec<String> {
        self.plugins.read()
            .iter()
            .map(|p| p.metadata().name.clone())
            .collect()
    }
    
    pub fn initialize_all(&self) -> PluginResult<()> {
        // Initialize all plugins
        for plugin in &mut *self.plugins.write() {
            plugin.initialize()?;
        }
        Ok(())
    }
    
    pub fn call_hook(&self, name: &str, _args: HookArgs) -> PluginResult<()> {
        // For now, just log hook calls
        if std::env::var("FASTEST_DEBUG").is_ok() {
            eprintln!("[Hook] {}", name);
        }
        Ok(())
    }
    
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> PluginResult<()> {
        self.plugins.write().push(plugin);
        Ok(())
    }
}

/// Builder for plugin manager
pub struct PluginManagerBuilder {
    plugins: Vec<Box<dyn Plugin>>,
    discover_installed: bool,
    load_conftest: bool,
    plugin_dirs: Vec<std::path::PathBuf>,
    disabled_plugins: Vec<String>,
}

impl PluginManagerBuilder {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            discover_installed: false,
            load_conftest: false,
            plugin_dirs: Vec::new(),
            disabled_plugins: Vec::new(),
        }
    }
    
    pub fn discover_installed(mut self, enabled: bool) -> Self {
        self.discover_installed = enabled;
        self
    }
    
    pub fn load_conftest(mut self, enabled: bool) -> Self {
        self.load_conftest = enabled;
        self
    }
    
    pub fn add_plugin_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.plugin_dirs.push(dir);
        self
    }
    
    pub fn disable_plugin(mut self, name: String) -> Self {
        self.disabled_plugins.push(name);
        self
    }
    
    pub fn with_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
        self.plugins.push(plugin);
        self
    }
    
    pub fn build(self) -> PluginResult<PluginManager> {
        let mut manager = PluginManager::new();
        
        // Register all plugins
        for plugin in self.plugins {
            manager.register(plugin)?;
        }
        
        Ok(manager)
    }
}