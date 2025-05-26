use crate::discovery::TestItem;
use crate::executor::TestResult;
use crate::error::Result;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin trait that defines hooks for extending Fastest behavior
pub trait Plugin: Send + Sync {
    /// Get the name of the plugin
    fn name(&self) -> &str;
    
    /// Called at the start of test collection
    fn pytest_collection_start(&self, _config: &crate::Config) -> Result<()> {
        Ok(())
    }
    
    /// Called after test collection is complete
    fn pytest_collection_finish(&self, _tests: &mut Vec<TestItem>) -> Result<()> {
        Ok(())
    }
    
    /// Called to modify test items after collection
    fn pytest_collection_modifyitems(&self, _tests: &mut Vec<TestItem>) -> Result<()> {
        Ok(())
    }
    
    /// Called before running tests
    fn pytest_runtest_protocol(&self, _item: &TestItem) -> Result<Option<TestResult>> {
        Ok(None)
    }
    
    /// Called before each test
    fn pytest_runtest_setup(&self, _item: &TestItem) -> Result<()> {
        Ok(())
    }
    
    /// Called after each test
    fn pytest_runtest_teardown(&self, _item: &TestItem, _result: &TestResult) -> Result<()> {
        Ok(())
    }
    
    /// Called when a test passes
    fn pytest_runtest_logreport(&self, _item: &TestItem, _result: &TestResult) -> Result<()> {
        Ok(())
    }
    
    /// Called at the end of test session
    fn pytest_sessionfinish(&self, _results: &[TestResult]) -> Result<()> {
        Ok(())
    }
    
    /// Get plugin configuration
    fn get_config(&self) -> Option<Box<dyn Any>> {
        None
    }
}

/// Plugin manager that handles plugin registration and hook calling
pub struct PluginManager {
    plugins: Vec<Arc<dyn Plugin>>,
    plugin_map: HashMap<String, Arc<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            plugin_map: HashMap::new(),
        }
    }
    
    /// Register a plugin
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) {
        let name = plugin.name().to_string();
        self.plugin_map.insert(name, plugin.clone());
        self.plugins.push(plugin);
    }
    
    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&Arc<dyn Plugin>> {
        self.plugin_map.get(name)
    }
    
    /// Call collection start hook on all plugins
    pub fn hook_collection_start(&self, config: &crate::Config) -> Result<()> {
        for plugin in &self.plugins {
            plugin.pytest_collection_start(config)?;
        }
        Ok(())
    }
    
    /// Call collection finish hook on all plugins
    pub fn hook_collection_finish(&self, tests: &mut Vec<TestItem>) -> Result<()> {
        for plugin in &self.plugins {
            plugin.pytest_collection_finish(tests)?;
        }
        Ok(())
    }
    
    /// Call collection modify items hook on all plugins
    pub fn hook_collection_modifyitems(&self, tests: &mut Vec<TestItem>) -> Result<()> {
        for plugin in &self.plugins {
            plugin.pytest_collection_modifyitems(tests)?;
        }
        Ok(())
    }
    
    /// Call runtest protocol hook on all plugins
    pub fn hook_runtest_protocol(&self, item: &TestItem) -> Result<Option<TestResult>> {
        for plugin in &self.plugins {
            if let Some(result) = plugin.pytest_runtest_protocol(item)? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }
    
    /// Call runtest setup hook on all plugins
    pub fn hook_runtest_setup(&self, item: &TestItem) -> Result<()> {
        for plugin in &self.plugins {
            plugin.pytest_runtest_setup(item)?;
        }
        Ok(())
    }
    
    /// Call runtest teardown hook on all plugins
    pub fn hook_runtest_teardown(&self, item: &TestItem, result: &TestResult) -> Result<()> {
        for plugin in &self.plugins {
            plugin.pytest_runtest_teardown(item, result)?;
        }
        Ok(())
    }
    
    /// Call runtest logreport hook on all plugins
    pub fn hook_runtest_logreport(&self, item: &TestItem, result: &TestResult) -> Result<()> {
        for plugin in &self.plugins {
            plugin.pytest_runtest_logreport(item, result)?;
        }
        Ok(())
    }
    
    /// Call session finish hook on all plugins
    pub fn hook_sessionfinish(&self, results: &[TestResult]) -> Result<()> {
        for plugin in &self.plugins {
            plugin.pytest_sessionfinish(results)?;
        }
        Ok(())
    }
    
    /// Load built-in plugins
    pub fn load_builtin_plugins(&mut self) {
        // Load marker plugin
        self.register(Arc::new(MarkerPlugin::new()));
        
        // Load timeout plugin
        self.register(Arc::new(TimeoutPlugin::new(None)));
        
        // Load output capture plugin
        self.register(Arc::new(CapturePlugin::new()));
    }
    
    /// Load plugins from config
    pub fn load_from_config(&mut self, config: &crate::Config) -> Result<()> {
        // TODO: Load plugins specified in config.required_plugins
        Ok(())
    }
}

/// Built-in marker plugin
struct MarkerPlugin {
    // Plugin state if needed
}

impl MarkerPlugin {
    fn new() -> Self {
        Self {}
    }
}

impl Plugin for MarkerPlugin {
    fn name(&self) -> &str {
        "markers"
    }
    
    fn pytest_collection_modifyitems(&self, tests: &mut Vec<TestItem>) -> Result<()> {
        // Filter tests based on markers
        // This is already handled in the main code, but could be moved here
        Ok(())
    }
}

/// Built-in timeout plugin
struct TimeoutPlugin {
    timeout: Option<u64>,
}

impl TimeoutPlugin {
    fn new(timeout: Option<u64>) -> Self {
        Self { timeout }
    }
}

impl Plugin for TimeoutPlugin {
    fn name(&self) -> &str {
        "timeout"
    }
    
    fn pytest_runtest_setup(&self, _item: &TestItem) -> Result<()> {
        // TODO: Set up timeout for test
        Ok(())
    }
}

/// Built-in output capture plugin
struct CapturePlugin {
    capture_enabled: bool,
}

impl CapturePlugin {
    fn new() -> Self {
        Self {
            capture_enabled: true,
        }
    }
}

impl Plugin for CapturePlugin {
    fn name(&self) -> &str {
        "capture"
    }
    
    fn pytest_runtest_setup(&self, _item: &TestItem) -> Result<()> {
        // TODO: Start capturing stdout/stderr
        Ok(())
    }
    
    fn pytest_runtest_teardown(&self, _item: &TestItem, _result: &TestResult) -> Result<()> {
        // TODO: Stop capturing and attach to result
        Ok(())
    }
}

/// Example third-party plugin structure
pub struct ExamplePlugin {
    name: String,
    config: HashMap<String, String>,
}

impl ExamplePlugin {
    pub fn new(name: String) -> Self {
        Self {
            name,
            config: HashMap::new(),
        }
    }
}

impl Plugin for ExamplePlugin {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn pytest_collection_start(&self, _config: &crate::Config) -> Result<()> {
        println!("Example plugin: collection starting");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_manager() {
        let mut manager = PluginManager::new();
        manager.load_builtin_plugins();
        
        assert!(manager.get_plugin("markers").is_some());
        assert!(manager.get_plugin("timeout").is_some());
        assert!(manager.get_plugin("capture").is_some());
    }
    
    #[test]
    fn test_custom_plugin() {
        let mut manager = PluginManager::new();
        let plugin = Arc::new(ExamplePlugin::new("example".to_string()));
        
        manager.register(plugin);
        assert!(manager.get_plugin("example").is_some());
    }
} 