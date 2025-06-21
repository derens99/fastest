//! Example Fastest Plugin
//!
//! This demonstrates how to create a custom plugin for Fastest.

use fastest_plugins::{
    api::{Plugin, PluginMetadata, PluginBuilder},
    hooks::{Hook, HookArgs, HookReturn, HookResult},
    impl_plugin,
};

/// Example plugin that adds timing information
pub struct TimingPlugin {
    metadata: PluginMetadata,
    start_times: std::collections::HashMap<String, std::time::Instant>,
}

impl TimingPlugin {
    pub fn new() -> Self {
        let metadata = PluginBuilder::new("example.timing")
            .version("0.1.0")
            .description("Adds detailed timing information to test reports")
            .author("Your Name")
            .priority(50)
            .build();
        
        Self {
            metadata,
            start_times: std::collections::HashMap::new(),
        }
    }
}

impl std::fmt::Debug for TimingPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimingPlugin")
            .field("metadata", &self.metadata)
            .finish()
    }
}

// Implement the Plugin trait
impl_plugin!(TimingPlugin);

/// Hook to record test start time
struct TimingSetupHook {
    plugin: std::sync::Arc<std::sync::Mutex<TimingPlugin>>,
}

impl Hook for TimingSetupHook {
    fn name(&self) -> &str {
        "pytest_runtest_setup"
    }
    
    fn execute(&self, args: HookArgs) -> HookResult<HookReturn> {
        if let Some(test_id) = args.get::<String>("test_id") {
            let mut plugin = self.plugin.lock().unwrap();
            plugin.start_times.insert(test_id.clone(), std::time::Instant::now());
        }
        Ok(HookReturn::None)
    }
}

impl std::fmt::Debug for TimingSetupHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimingSetupHook").finish()
    }
}

/// Hook to calculate and report test duration
struct TimingTeardownHook {
    plugin: std::sync::Arc<std::sync::Mutex<TimingPlugin>>,
}

impl Hook for TimingTeardownHook {
    fn name(&self) -> &str {
        "pytest_runtest_teardown"
    }
    
    fn execute(&self, args: HookArgs) -> HookResult<HookReturn> {
        if let Some(test_id) = args.get::<String>("test_id") {
            let plugin = self.plugin.lock().unwrap();
            if let Some(start_time) = plugin.start_times.get(test_id) {
                let duration = start_time.elapsed();
                println!("Test '{}' took {:?}", test_id, duration);
                
                // You could store this information for later reporting
            }
        }
        Ok(HookReturn::None)
    }
}

impl std::fmt::Debug for TimingTeardownHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimingTeardownHook").finish()
    }
}

/// Entry point for dynamic loading
#[no_mangle]
pub extern "C" fn fastest_plugin_entry() -> *mut dyn Plugin {
    Box::into_raw(Box::new(TimingPlugin::new()))
}

/// Example of using the plugin
#[cfg(test)]
mod tests {
    use super::*;
    use fastest_plugins::manager::PluginManager;
    
    #[test]
    fn test_timing_plugin() {
        let mut manager = PluginManager::new();
        
        // Register the plugin
        let plugin = Box::new(TimingPlugin::new());
        manager.register(plugin).unwrap();
        
        // Initialize plugins
        manager.initialize().unwrap();
        
        // Simulate test execution
        let mut args = HookArgs::new();
        args.insert("test_id", "test_example".to_string());
        
        // Call setup hook
        manager.hook_caller("pytest_runtest_setup")
            .arg("test_id", "test_example")
            .call()
            .unwrap();
        
        // Simulate test execution
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // Call teardown hook
        manager.hook_caller("pytest_runtest_teardown")
            .arg("test_id", "test_example")
            .call()
            .unwrap();
    }
}