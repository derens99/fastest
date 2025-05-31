//! Built-in Plugins for pytest Compatibility
//!
//! Smart, minimal built-in plugins using external libraries

use anyhow::Result;
use serde_json::json;

use super::Plugin;
use super::hooks::{HookRegistry, HookResult};
use crate::register_plugin;

/// Markers plugin for pytest compatibility
pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn name(&self) -> &str {
        "markers"
    }

    fn register_hooks(&self, registry: &mut HookRegistry) -> Result<()> {
        // Register marker processing hooks
        registry.add_hook("pytest_configure", |_data| {
            // Initialize marker configuration
            Ok(HookResult::Value(json!({
                "markers_configured": true,
                "builtin_markers": ["skip", "skipif", "xfail", "parametrize"]
            })))
        })?;

        registry.add_hook("pytest_collection_modifyitems", |data| {
            // Process markers on collected items
            if let Some(items) = data.args.get("items") {
                if let Some(items_array) = items.as_array() {
                    let processed_items: Vec<_> = items_array
                        .iter()
                        .map(|item| {
                            let modified_item = item.clone();
                            // Add marker processing logic here
                            modified_item
                        })
                        .collect();

                    return Ok(HookResult::Modified(json!({
                        "items": processed_items
                    })));
                }
            }
            Ok(HookResult::None)
        })?;

        Ok(())
    }
}

/// Parametrize plugin for pytest compatibility
pub struct ParametrizePlugin;

impl Plugin for ParametrizePlugin {
    fn name(&self) -> &str {
        "parametrize"
    }

    fn register_hooks(&self, registry: &mut HookRegistry) -> Result<()> {
        registry.add_hook("pytest_collection_modifyitems", |data| {
            // Expand parametrized tests
            if let Some(items) = data.args.get("items") {
                if let Some(items_array) = items.as_array() {
                    let mut expanded_items = Vec::new();

                    for item in items_array {
                        // Check for parametrize markers
                        if let Some(markers) = item.get("markers") {
                            if markers.as_array().map_or(false, |m| {
                                m.iter().any(|marker| {
                                    marker.get("name").and_then(|n| n.as_str())
                                        == Some("parametrize")
                                })
                            }) {
                                // Expand parametrized test
                                let expanded = expand_parametrized_item(item);
                                expanded_items.extend(expanded);
                            } else {
                                expanded_items.push(item.clone());
                            }
                        } else {
                            expanded_items.push(item.clone());
                        }
                    }

                    return Ok(HookResult::Modified(json!({
                        "items": expanded_items
                    })));
                }
            }
            Ok(HookResult::None)
        })?;

        Ok(())
    }
}

/// Capture plugin for output handling
pub struct CapturePlugin;

impl Plugin for CapturePlugin {
    fn name(&self) -> &str {
        "capture"
    }

    fn register_hooks(&self, registry: &mut HookRegistry) -> Result<()> {
        registry.add_hook("pytest_runtest_setup", |_data| {
            // Setup capture for test
            Ok(HookResult::Value(json!({
                "capture_setup": true,
                "stdout_capture": true,
                "stderr_capture": true
            })))
        })?;

        registry.add_hook("pytest_runtest_teardown", |_data| {
            // Cleanup capture after test
            Ok(HookResult::Value(json!({
                "capture_cleanup": true
            })))
        })?;

        Ok(())
    }
}

/// Fixtures plugin for pytest compatibility
pub struct FixturesPlugin;

impl Plugin for FixturesPlugin {
    fn name(&self) -> &str {
        "fixtures"
    }

    fn register_hooks(&self, registry: &mut HookRegistry) -> Result<()> {
        registry.add_hook("pytest_fixture_setup", |data| {
            // Handle fixture setup
            if let Some(fixturedef) = data.args.get("fixturedef") {
                Ok(HookResult::Value(json!({
                    "fixture_setup": true,
                    "fixture_name": fixturedef.get("argname"),
                    "fixture_scope": fixturedef.get("scope")
                })))
            } else {
                Ok(HookResult::None)
            }
        })?;

        registry.add_hook("pytest_runtest_setup", |data| {
            // Setup fixtures for test
            if let Some(item) = data.args.get("item") {
                let fixtures_needed = extract_fixture_dependencies(item);
                Ok(HookResult::Value(json!({
                    "fixtures_setup": true,
                    "fixtures": fixtures_needed
                })))
            } else {
                Ok(HookResult::None)
            }
        })?;

        Ok(())
    }
}

/// Terminal plugin for output formatting
pub struct TerminalPlugin;

impl Plugin for TerminalPlugin {
    fn name(&self) -> &str {
        "terminal"
    }

    fn register_hooks(&self, registry: &mut HookRegistry) -> Result<()> {
        registry.add_hook("pytest_report_header", |_data| {
            Ok(HookResult::Value(json!({
                "header": "Fastest Test Runner - pytest compatible mode",
                "version": env!("CARGO_PKG_VERSION")
            })))
        })?;

        registry.add_hook("pytest_terminal_summary", |data| {
            Ok(HookResult::Value(json!({
                "summary": "Test execution completed",
                "stats": data.args.get("stats")
            })))
        })?;

        Ok(())
    }
}

// Helper functions

fn expand_parametrized_item(item: &serde_json::Value) -> Vec<serde_json::Value> {
    // Simplified parametrize expansion
    if let Some(markers) = item.get("markers").and_then(|m| m.as_array()) {
        for marker in markers {
            if marker.get("name").and_then(|n| n.as_str()) == Some("parametrize") {
                if let Some(args) = marker.get("args").and_then(|a| a.as_array()) {
                    if args.len() >= 2 {
                        let _param_names = &args[0];
                        let param_values = &args[1];

                        if let Some(values_array) = param_values.as_array() {
                            return values_array
                                .iter()
                                .enumerate()
                                .map(|(i, values)| {
                                    let mut expanded_item = item.clone();

                                    // Add parameter info
                                    if let Some(item_obj) = expanded_item.as_object_mut() {
                                        item_obj.insert("param_index".to_string(), json!(i));
                                        item_obj.insert("param_values".to_string(), values.clone());

                                        // Modify test ID
                                        if let Some(test_id) =
                                            item_obj.get("id").and_then(|id| id.as_str())
                                        {
                                            let new_id = format!("{}[{}]", test_id, i);
                                            item_obj.insert("id".to_string(), json!(new_id));
                                        }
                                    }

                                    expanded_item
                                })
                                .collect();
                        }
                    }
                }
            }
        }
    }

    vec![item.clone()]
}

fn extract_fixture_dependencies(item: &serde_json::Value) -> Vec<String> {
    // Extract fixture dependencies from test item
    let mut fixtures = Vec::new();

    // Check function signature or other metadata
    if let Some(fixtures_list) = item.get("fixtures").and_then(|f| f.as_array()) {
        for fixture in fixtures_list {
            if let Some(name) = fixture.as_str() {
                fixtures.push(name.to_string());
            }
        }
    }

    fixtures
}

// Register all built-in plugins using inventory
register_plugin!(|| Ok(Box::new(MarkersPlugin) as Box<dyn Plugin>));
register_plugin!(|| Ok(Box::new(ParametrizePlugin) as Box<dyn Plugin>));
register_plugin!(|| Ok(Box::new(CapturePlugin) as Box<dyn Plugin>));
register_plugin!(|| Ok(Box::new(FixturesPlugin) as Box<dyn Plugin>));
register_plugin!(|| Ok(Box::new(TerminalPlugin) as Box<dyn Plugin>));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::hooks::HookRegistry;

    #[test]
    fn test_markers_plugin() {
        let mut registry = HookRegistry::new();
        let plugin = MarkersPlugin;

        plugin.register_hooks(&mut registry).unwrap();
        assert_eq!(plugin.name(), "markers");
    }

    #[test]
    fn test_parametrize_expansion() {
        let item = json!({
            "id": "test_example",
            "markers": [{
                "name": "parametrize",
                "args": [
                    ["x", "y"],
                    [[1, 2], [3, 4]]
                ]
            }]
        });

        let expanded = expand_parametrized_item(&item);
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0]["id"], "test_example[0]");
        assert_eq!(expanded[1]["id"], "test_example[1]");
    }
}
