//! Plugin Manager - Central orchestrator for all plugins
//!
//! The PluginManager handles plugin lifecycle, dependency resolution,
//! and hook coordination.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use parking_lot::RwLock;
use indexmap::IndexMap;

use crate::api::{Plugin, PluginError, PluginInfo, PluginMetadata, PluginResult, PluginType};
use crate::hooks::{HookRegistry, HookCaller};
use crate::loader::PluginLoader;
use crate::registry::PluginRegistry;

/// Plugin manager configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// Enable plugin discovery from installed packages
    pub discover_installed: bool,
    
    /// Enable loading from conftest.py files
    pub load_conftest: bool,
    
    /// Additional plugin directories
    pub plugin_dirs: Vec<std::path::PathBuf>,
    
    /// Disabled plugins
    pub disabled: HashSet<String>,
    
    /// Plugin load timeout (ms)
    pub load_timeout: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            discover_installed: true,
            load_conftest: true,
            plugin_dirs: Vec::new(),
            disabled: HashSet::new(),
            load_timeout: 5000,
        }
    }
}

/// Central plugin manager
pub struct PluginManager {
    /// Configuration
    config: PluginConfig,
    
    /// Loaded plugins
    plugins: Arc<RwLock<IndexMap<String, Box<dyn Plugin>>>>,
    
    /// Plugin metadata
    metadata: Arc<RwLock<HashMap<String, PluginInfo>>>,
    
    /// Hook registry
    hooks: Arc<HookRegistry>,
    
    /// Plugin loader
    loader: PluginLoader,
    
    /// Initialization state
    initialized: bool,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self::with_config(PluginConfig::default())
    }
    
    /// Create with custom configuration
    pub fn with_config(config: PluginConfig) -> Self {
        Self {
            config,
            plugins: Arc::new(RwLock::new(IndexMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            hooks: Arc::new(HookRegistry::new()),
            loader: PluginLoader::new(),
            initialized: false,
        }
    }
    
    /// Register a plugin manually
    pub fn register(&mut self, mut plugin: Box<dyn Plugin>) -> PluginResult<()> {
        let metadata = plugin.metadata().clone();
        
        // Check if already registered
        if self.plugins.read().contains_key(&metadata.name) {
            return Err(PluginError::Conflict(
                format!("Plugin '{}' already registered", metadata.name)
            ));
        }
        
        // Check if disabled
        if self.config.disabled.contains(&metadata.name) {
            return Ok(());
        }
        
        // Validate dependencies
        self.validate_dependencies(&metadata)?;
        
        // Initialize plugin
        plugin.initialize()?;
        
        // Store plugin info
        let info = PluginInfo {
            metadata: metadata.clone(),
            plugin_type: PluginType::Builtin,
            source: "manual".to_string(),
        };
        
        self.metadata.write().insert(metadata.name.clone(), info);
        self.plugins.write().insert(metadata.name.clone(), plugin);
        
        Ok(())
    }
    
    /// Initialize the plugin system
    pub fn initialize(&mut self) -> PluginResult<()> {
        if self.initialized {
            return Ok(());
        }
        
        // Discover and load plugins
        if self.config.discover_installed {
            self.discover_installed_plugins()?;
        }
        
        if self.config.load_conftest {
            self.load_conftest_plugins()?;
        }
        
        // Load plugins from additional directories
        for dir in &self.config.plugin_dirs.clone() {
            self.load_plugins_from_dir(dir)?;
        }
        
        // Sort plugins by dependency order
        let sorted_names = self.topological_sort()?;
        
        // Initialize plugins in order
        for name in sorted_names {
            if let Some(mut plugin) = self.plugins.write().get_mut(&name) {
                plugin.initialize()?;
            }
        }
        
        self.initialized = true;
        
        // Call configure hook
        self.hook_caller("pytest_configure").call()?;
        
        Ok(())
    }
    
    /// Shutdown all plugins
    pub fn shutdown(&mut self) -> PluginResult<()> {
        // Call session finish hook
        let _ = self.hook_caller("pytest_sessionfinish").call();
        
        // Shutdown plugins in reverse order
        let names: Vec<_> = self.plugins.read().keys().cloned().collect();
        for name in names.iter().rev() {
            if let Some(mut plugin) = self.plugins.write().get_mut(name) {
                let _ = plugin.shutdown();
            }
        }
        
        self.initialized = false;
        Ok(())
    }
    
    /// Get a plugin by name
    pub fn get<T: Plugin + 'static>(&self, name: &str) -> Option<&T> {
        let plugins = self.plugins.read();
        plugins.get(name)?.as_any().downcast_ref::<T>()
    }
    
    /// Get mutable plugin reference
    pub fn get_mut<T: Plugin + 'static>(&mut self, name: &str) -> Option<&mut T> {
        let mut plugins = self.plugins.write();
        plugins.get_mut(name)?.as_any_mut().downcast_mut::<T>()
    }
    
    /// Get the hook registry
    pub fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }
    
    /// Create a hook caller
    pub fn hook_caller(&self, hook_name: &str) -> HookCaller {
        HookCaller::new(&self.hooks, hook_name)
    }
    
    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.metadata.read().values().cloned().collect()
    }
    
    /// Validate plugin dependencies
    fn validate_dependencies(&self, metadata: &PluginMetadata) -> PluginResult<()> {
        let plugins = self.plugins.read();
        
        // Check required dependencies
        for required in &metadata.requires {
            if !plugins.contains_key(required) {
                return Err(PluginError::NotFound(
                    format!("Required plugin '{}' not found", required)
                ));
            }
        }
        
        // Check conflicts
        for conflict in &metadata.conflicts {
            if plugins.contains_key(conflict) {
                return Err(PluginError::Conflict(
                    format!("Plugin conflicts with '{}'", conflict)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Topological sort for dependency order
    fn topological_sort(&self) -> PluginResult<Vec<String>> {
        let metadata = self.metadata.read();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        
        // Build dependency graph
        for (name, info) in metadata.iter() {
            graph.entry(name.clone()).or_insert_with(Vec::new);
            in_degree.entry(name.clone()).or_insert(0);
            
            for dep in &info.metadata.requires {
                graph.entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(name.clone());
                *in_degree.get_mut(name).unwrap() += 1;
            }
        }
        
        // Kahn's algorithm
        let mut queue: Vec<_> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();
        
        let mut sorted = Vec::new();
        
        while let Some(name) = queue.pop() {
            sorted.push(name.clone());
            
            if let Some(deps) = graph.get(&name) {
                for dep in deps {
                    let deg = in_degree.get_mut(dep).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(dep.clone());
                    }
                }
            }
        }
        
        if sorted.len() != metadata.len() {
            return Err(PluginError::Invalid(
                "Circular dependency detected".to_string()
            ));
        }
        
        Ok(sorted)
    }
    
    /// Discover installed plugins
    fn discover_installed_plugins(&mut self) -> PluginResult<()> {
        let discovered = self.loader.discover_entry_points()?;
        
        for info in discovered {
            if self.config.disabled.contains(&info.metadata.name) {
                continue;
            }
            
            match self.loader.load_plugin(&info) {
                Ok(plugin) => {
                    self.register(plugin)?;
                }
                Err(e) => {
                    eprintln!("Failed to load plugin '{}': {}", info.metadata.name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Load conftest plugins
    fn load_conftest_plugins(&mut self) -> PluginResult<()> {
        let conftest_files = self.loader.find_conftest_files()?;
        
        for path in conftest_files {
            match self.loader.load_conftest(&path) {
                Ok(plugins) => {
                    for plugin in plugins {
                        self.register(plugin)?;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load conftest {:?}: {}", path, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Load plugins from a directory
    fn load_plugins_from_dir(&mut self, dir: &std::path::Path) -> PluginResult<()> {
        let plugins = self.loader.load_from_directory(dir)?;
        
        for plugin in plugins {
            self.register(plugin)?;
        }
        
        Ok(())
    }
}

/// Plugin manager builder
pub struct PluginManagerBuilder {
    config: PluginConfig,
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManagerBuilder {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
            plugins: Vec::new(),
        }
    }
    
    pub fn discover_installed(mut self, enabled: bool) -> Self {
        self.config.discover_installed = enabled;
        self
    }
    
    pub fn load_conftest(mut self, enabled: bool) -> Self {
        self.config.load_conftest = enabled;
        self
    }
    
    pub fn plugin_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.config.plugin_dirs.push(dir.into());
        self
    }
    
    pub fn disable(mut self, name: impl Into<String>) -> Self {
        self.config.disabled.insert(name.into());
        self
    }
    
    pub fn with_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
        self.plugins.push(plugin);
        self
    }
    
    pub fn build(self) -> PluginResult<PluginManager> {
        let mut manager = PluginManager::with_config(self.config);
        
        // Register provided plugins
        for plugin in self.plugins {
            manager.register(plugin)?;
        }
        
        Ok(manager)
    }
}