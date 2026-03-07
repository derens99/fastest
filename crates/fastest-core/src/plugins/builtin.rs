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
// Declarative macro for built-in plugins
// ---------------------------------------------------------------------------

/// Generates a built-in plugin struct with the standard fields, constructors,
/// and `Plugin` trait implementation.  The only differences between built-in
/// plugins are the struct name, plugin name string, description, and priority.
macro_rules! builtin_plugin {
    (
        $(#[$attr:meta])*
        $struct_name:ident,
        name: $plugin_name:expr,
        description: $description:expr,
        priority: $priority:expr
    ) => {
        $(#[$attr])*
        #[derive(Debug)]
        pub struct $struct_name {
            metadata: PluginMetadata,
            initialized: bool,
            hooks_received: Vec<String>,
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $struct_name {
            pub fn new() -> Self {
                Self {
                    metadata: PluginMetadata {
                        name: $plugin_name.into(),
                        version: env!("CARGO_PKG_VERSION").into(),
                        description: $description.into(),
                        priority: $priority,
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

        impl Plugin for $struct_name {
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
    };
}

// ---------------------------------------------------------------------------
// Built-in plugin definitions
// ---------------------------------------------------------------------------

builtin_plugin!(
    /// Manages fixture setup / teardown coordination.
    FixturePlugin,
    name: "fixture",
    description: "Manages fixture setup and teardown coordination",
    priority: 100
);

builtin_plugin!(
    /// Processes pytest markers (skip, xfail, parametrize, etc.).
    MarkerPlugin,
    name: "marker",
    description: "Processes pytest markers for test selection and behavior",
    priority: 90
);

builtin_plugin!(
    /// Collects and formats test results for output.
    ReportingPlugin,
    name: "reporting",
    description: "Collects and formats test results for output",
    priority: 80
);

builtin_plugin!(
    /// Captures stdout/stderr during test execution.
    CapturePlugin,
    name: "capture",
    description: "Captures stdout and stderr during test execution",
    priority: 70
);

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
