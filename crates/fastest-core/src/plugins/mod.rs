//! Plugin system for extending the test runner.
//!
//! Provides the [`Plugin`] trait that all plugins implement, a [`PluginManager`]
//! that owns and orchestrates plugins, and supporting types ([`HookArgs`],
//! [`HookResult`], [`PluginMetadata`]).

pub mod builtin;
pub mod hooks;

use crate::error::{Error, Result};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Metadata describing a plugin.
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Human-readable plugin name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Short description of the plugin's purpose.
    pub description: String,
    /// Execution priority — higher values run first.
    pub priority: i32,
}

/// Arguments passed to a hook invocation.
#[derive(Debug, Default)]
pub struct HookArgs {
    /// Arbitrary key/value data available to the plugin.
    pub data: HashMap<String, serde_json::Value>,
}

impl HookArgs {
    /// Create an empty `HookArgs`.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Insert a key/value pair, returning `&mut Self` for chaining.
    pub fn insert(&mut self, key: impl Into<String>, value: serde_json::Value) -> &mut Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Retrieve a value by key.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }
}

/// Result data returned by a plugin from a hook invocation.
#[derive(Debug, Default)]
pub struct HookResult {
    /// Arbitrary key/value data produced by the plugin.
    pub data: HashMap<String, serde_json::Value>,
}

impl HookResult {
    /// Create an empty `HookResult`.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Insert a key/value pair, returning `&mut Self` for chaining.
    pub fn insert(&mut self, key: impl Into<String>, value: serde_json::Value) -> &mut Self {
        self.data.insert(key.into(), value);
        self
    }
}

// ---------------------------------------------------------------------------
// Plugin trait
// ---------------------------------------------------------------------------

/// Trait that every plugin must implement.
///
/// Plugins receive lifecycle callbacks ([`initialize`](Plugin::initialize),
/// [`shutdown`](Plugin::shutdown)) and hook notifications via
/// [`on_hook`](Plugin::on_hook).
pub trait Plugin: Debug + Send + Sync {
    /// Return metadata describing this plugin.
    fn metadata(&self) -> &PluginMetadata;

    /// Called once when the plugin is first activated.
    fn initialize(&mut self) -> Result<()>;

    /// Called once when the plugin is being deactivated.
    fn shutdown(&mut self) -> Result<()>;

    /// Called for every hook invocation.
    ///
    /// Return `Ok(None)` if the plugin has nothing to contribute for this
    /// particular hook.
    fn on_hook(&mut self, hook: &str, args: &HookArgs) -> Result<Option<HookResult>>;

    /// Downcast helper for concrete-type access.
    fn as_any(&self) -> &dyn Any;
}

// ---------------------------------------------------------------------------
// PluginManager
// ---------------------------------------------------------------------------

/// Owns a set of plugins and dispatches hook calls to them.
#[derive(Debug)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    /// Create an empty manager with no plugins registered.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Create a manager pre-loaded with all built-in plugins.
    pub fn with_builtins() -> Result<Self> {
        let mut manager = Self::new();
        manager.register(Box::new(builtin::FixturePlugin::new()))?;
        manager.register(Box::new(builtin::MarkerPlugin::new()))?;
        manager.register(Box::new(builtin::ReportingPlugin::new()))?;
        manager.register(Box::new(builtin::CapturePlugin::new()))?;
        Ok(manager)
    }

    /// Register a plugin and maintain descending priority order.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        self.plugins.push(plugin);
        self.plugins
            .sort_by(|a, b| b.metadata().priority.cmp(&a.metadata().priority));
        Ok(())
    }

    /// Call [`Plugin::initialize`] on every registered plugin.
    pub fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialize().map_err(|e| {
                Error::Plugin(format!(
                    "failed to initialize plugin '{}': {}",
                    plugin.metadata().name,
                    e
                ))
            })?;
        }
        Ok(())
    }

    /// Call [`Plugin::shutdown`] on every registered plugin.
    pub fn shutdown_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.shutdown().map_err(|e| {
                Error::Plugin(format!(
                    "failed to shut down plugin '{}': {}",
                    plugin.metadata().name,
                    e
                ))
            })?;
        }
        Ok(())
    }

    /// Dispatch a hook to every registered plugin, collecting results.
    ///
    /// Plugins are called in priority order (highest first). Only non-`None`
    /// results are included in the returned vector.
    pub fn call_hook(&mut self, hook: &str, args: &HookArgs) -> Result<Vec<HookResult>> {
        let mut results = Vec::new();
        for plugin in &mut self.plugins {
            if let Some(result) = plugin.on_hook(hook, args)? {
                results.push(result);
            }
        }
        Ok(results)
    }

    /// Return the number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Return an iterator over registered plugins (in priority order).
    pub fn plugins(&self) -> impl Iterator<Item = &dyn Plugin> {
        self.plugins.iter().map(|p| p.as_ref())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_with_builtins() {
        let manager = PluginManager::with_builtins().unwrap();
        assert_eq!(manager.plugin_count(), 4);
    }

    #[test]
    fn test_plugin_priority_ordering() {
        let manager = PluginManager::with_builtins().unwrap();
        let priorities: Vec<i32> = manager.plugins().map(|p| p.metadata().priority).collect();
        // Must be sorted descending
        for window in priorities.windows(2) {
            assert!(
                window[0] >= window[1],
                "plugins not sorted by descending priority: {:?}",
                priorities,
            );
        }
        assert_eq!(priorities, vec![100, 90, 80, 70]);
    }

    #[test]
    fn test_call_hook() {
        let mut manager = PluginManager::with_builtins().unwrap();
        manager.initialize_all().unwrap();

        let args = HookArgs::new();
        let results = manager.call_hook(hooks::COLLECTION_START, &args).unwrap();

        // Every built-in plugin responds to every hook with a result
        assert_eq!(results.len(), 4);
        for result in &results {
            assert!(result.data.contains_key("plugin"));
            assert!(result.data.contains_key("hook"));
        }

        manager.shutdown_all().unwrap();
    }

    #[test]
    fn test_hook_args() {
        let mut args = HookArgs::new();
        args.insert("count", serde_json::json!(42));
        args.insert("name", serde_json::json!("test_example"));

        assert_eq!(args.get("count"), Some(&serde_json::json!(42)));
        assert_eq!(args.get("name"), Some(&serde_json::json!("test_example")));
        assert_eq!(args.get("missing"), None);
    }

    #[test]
    fn test_hook_result() {
        let mut result = HookResult::new();
        result.insert("status", serde_json::json!("ok"));
        assert_eq!(result.data.get("status"), Some(&serde_json::json!("ok")));
    }

    #[test]
    fn test_plugin_metadata() {
        let meta = PluginMetadata {
            name: "test-plugin".into(),
            version: "0.1.0".into(),
            description: "A test plugin".into(),
            priority: 50,
        };
        assert_eq!(meta.name, "test-plugin");
        assert_eq!(meta.priority, 50);
    }

    #[test]
    fn test_empty_manager() {
        let mut manager = PluginManager::new();
        assert_eq!(manager.plugin_count(), 0);

        let results = manager
            .call_hook(hooks::RUNTEST_CALL, &HookArgs::new())
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_register_maintains_order() {
        let mut manager = PluginManager::new();
        // Register in non-priority order
        manager
            .register(Box::new(builtin::CapturePlugin::new()))
            .unwrap();
        manager
            .register(Box::new(builtin::FixturePlugin::new()))
            .unwrap();
        manager
            .register(Box::new(builtin::ReportingPlugin::new()))
            .unwrap();
        manager
            .register(Box::new(builtin::MarkerPlugin::new()))
            .unwrap();

        let names: Vec<&str> = manager
            .plugins()
            .map(|p| p.metadata().name.as_str())
            .collect();
        assert_eq!(names, vec!["fixture", "marker", "reporting", "capture"]);
    }
}
