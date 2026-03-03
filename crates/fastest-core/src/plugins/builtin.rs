//! Built-in plugins shipped with the test runner.
//!
//! Each plugin focuses on a single concern and records which hooks it has
//! processed so that callers (or tests) can inspect activity.

use super::{HookArgs, HookResult, Plugin, PluginMetadata};
use crate::error::Result;
use std::any::Any;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a standard [`HookResult`] that every built-in plugin returns.
fn standard_result(plugin_name: &str, hook: &str) -> HookResult {
    let mut result = HookResult::new();
    result.insert("plugin", serde_json::json!(plugin_name));
    result.insert("hook", serde_json::json!(hook));
    result
}

// ---------------------------------------------------------------------------
// FixturePlugin (priority 100)
// ---------------------------------------------------------------------------

/// Manages fixture setup / teardown coordination.
#[derive(Debug)]
pub struct FixturePlugin {
    metadata: PluginMetadata,
    initialized: bool,
    hooks_received: Vec<String>,
}

impl Default for FixturePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl FixturePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "fixture".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Manages fixture setup and teardown coordination".into(),
                priority: 100,
            },
            initialized: false,
            hooks_received: Vec::new(),
        }
    }

    /// Return a snapshot of hook names this plugin has processed.
    pub fn hooks_received(&self) -> &[String] {
        &self.hooks_received
    }
}

impl Plugin for FixturePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }

    fn on_hook(&mut self, hook: &str, _args: &HookArgs) -> Result<Option<HookResult>> {
        self.hooks_received.push(hook.to_string());
        Ok(Some(standard_result(&self.metadata.name, hook)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// MarkerPlugin (priority 90)
// ---------------------------------------------------------------------------

/// Processes pytest markers (skip, xfail, parametrize, etc.).
#[derive(Debug)]
pub struct MarkerPlugin {
    metadata: PluginMetadata,
    initialized: bool,
    hooks_received: Vec<String>,
}

impl Default for MarkerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkerPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "marker".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Processes pytest markers for test selection and behavior".into(),
                priority: 90,
            },
            initialized: false,
            hooks_received: Vec::new(),
        }
    }

    /// Return a snapshot of hook names this plugin has processed.
    pub fn hooks_received(&self) -> &[String] {
        &self.hooks_received
    }
}

impl Plugin for MarkerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }

    fn on_hook(&mut self, hook: &str, _args: &HookArgs) -> Result<Option<HookResult>> {
        self.hooks_received.push(hook.to_string());
        Ok(Some(standard_result(&self.metadata.name, hook)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// ReportingPlugin (priority 80)
// ---------------------------------------------------------------------------

/// Collects and formats test results for output.
#[derive(Debug)]
pub struct ReportingPlugin {
    metadata: PluginMetadata,
    initialized: bool,
    hooks_received: Vec<String>,
}

impl Default for ReportingPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportingPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "reporting".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Collects and formats test results for output".into(),
                priority: 80,
            },
            initialized: false,
            hooks_received: Vec::new(),
        }
    }

    /// Return a snapshot of hook names this plugin has processed.
    pub fn hooks_received(&self) -> &[String] {
        &self.hooks_received
    }
}

impl Plugin for ReportingPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }

    fn on_hook(&mut self, hook: &str, _args: &HookArgs) -> Result<Option<HookResult>> {
        self.hooks_received.push(hook.to_string());
        Ok(Some(standard_result(&self.metadata.name, hook)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// CapturePlugin (priority 70)
// ---------------------------------------------------------------------------

/// Captures stdout/stderr during test execution.
#[derive(Debug)]
pub struct CapturePlugin {
    metadata: PluginMetadata,
    initialized: bool,
    hooks_received: Vec<String>,
}

impl Default for CapturePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CapturePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "capture".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                description: "Captures stdout and stderr during test execution".into(),
                priority: 70,
            },
            initialized: false,
            hooks_received: Vec::new(),
        }
    }

    /// Return a snapshot of hook names this plugin has processed.
    pub fn hooks_received(&self) -> &[String] {
        &self.hooks_received
    }
}

impl Plugin for CapturePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }

    fn on_hook(&mut self, hook: &str, _args: &HookArgs) -> Result<Option<HookResult>> {
        self.hooks_received.push(hook.to_string());
        Ok(Some(standard_result(&self.metadata.name, hook)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::hooks;

    #[test]
    fn test_fixture_plugin_lifecycle() {
        let mut plugin = FixturePlugin::new();
        assert_eq!(plugin.metadata().name, "fixture");
        assert_eq!(plugin.metadata().priority, 100);

        plugin.initialize().unwrap();

        let args = HookArgs::new();
        let result = plugin
            .on_hook(hooks::RUNTEST_SETUP, &args)
            .unwrap()
            .expect("expected Some(HookResult)");
        assert_eq!(result.data["plugin"], serde_json::json!("fixture"));
        assert_eq!(result.data["hook"], serde_json::json!(hooks::RUNTEST_SETUP));
        assert_eq!(plugin.hooks_received(), &[hooks::RUNTEST_SETUP]);

        plugin.shutdown().unwrap();
    }

    #[test]
    fn test_marker_plugin_lifecycle() {
        let mut plugin = MarkerPlugin::new();
        assert_eq!(plugin.metadata().name, "marker");
        assert_eq!(plugin.metadata().priority, 90);

        plugin.initialize().unwrap();
        let args = HookArgs::new();
        plugin
            .on_hook(hooks::COLLECTION_MODIFY_ITEMS, &args)
            .unwrap();
        assert_eq!(plugin.hooks_received().len(), 1);
        plugin.shutdown().unwrap();
    }

    #[test]
    fn test_reporting_plugin_lifecycle() {
        let mut plugin = ReportingPlugin::new();
        assert_eq!(plugin.metadata().name, "reporting");
        assert_eq!(plugin.metadata().priority, 80);

        plugin.initialize().unwrap();
        let args = HookArgs::new();
        plugin.on_hook(hooks::RUNTEST_LOGREPORT, &args).unwrap();
        assert_eq!(plugin.hooks_received().len(), 1);
        plugin.shutdown().unwrap();
    }

    #[test]
    fn test_capture_plugin_lifecycle() {
        let mut plugin = CapturePlugin::new();
        assert_eq!(plugin.metadata().name, "capture");
        assert_eq!(plugin.metadata().priority, 70);

        plugin.initialize().unwrap();
        let args = HookArgs::new();
        plugin.on_hook(hooks::RUNTEST_CALL, &args).unwrap();
        assert_eq!(plugin.hooks_received().len(), 1);
        plugin.shutdown().unwrap();
    }

    #[test]
    fn test_as_any_downcast() {
        let plugin = FixturePlugin::new();
        let any_ref = plugin.as_any();
        assert!(any_ref.downcast_ref::<FixturePlugin>().is_some());
        assert!(any_ref.downcast_ref::<MarkerPlugin>().is_none());
    }

    #[test]
    fn test_multiple_hooks_recorded() {
        let mut plugin = CapturePlugin::new();
        plugin.initialize().unwrap();
        let args = HookArgs::new();

        plugin.on_hook(hooks::RUNTEST_SETUP, &args).unwrap();
        plugin.on_hook(hooks::RUNTEST_CALL, &args).unwrap();
        plugin.on_hook(hooks::RUNTEST_TEARDOWN, &args).unwrap();

        assert_eq!(
            plugin.hooks_received(),
            &[
                hooks::RUNTEST_SETUP,
                hooks::RUNTEST_CALL,
                hooks::RUNTEST_TEARDOWN,
            ]
        );
        plugin.shutdown().unwrap();
    }
}
