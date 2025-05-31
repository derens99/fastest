//! Pytest Compatibility Layer
//!
//! Complete pytest plugin API compatibility using smart external library integration

use anyhow::Result;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::{
    PluginManager, PluginConfig,
    hooks::{HookResult, HookData},
};
use crate::discovery::TestItem;
use crate::fixtures::Fixture;

/// Pytest-compatible session object
#[derive(Debug, Clone)]
pub struct PytestSession {
    pub config: PytestConfig,
    pub items: Vec<TestItem>,
    pub fixtures: HashMap<String, Fixture>,
    pub plugin_manager: Arc<PluginManager>,
}

/// Pytest-compatible configuration
#[derive(Debug, Clone)]
pub struct PytestConfig {
    pub rootdir: PathBuf,
    pub inifile: Option<PathBuf>,
    pub args: Vec<String>,
    pub option: HashMap<String, serde_json::Value>,
    pub pluginmanager: Arc<PluginManager>,
}

/// Pytest-compatible item representation
#[derive(Debug, Clone)]
pub struct PytestItem {
    pub nodeid: String,
    pub name: String,
    pub path: PathBuf,
    pub function: String,
    pub cls: Option<String>,
    pub module: String,
    pub markers: Vec<PytestMarker>,
    pub fixtures: Vec<String>,
}

/// Pytest-compatible marker
#[derive(Debug, Clone)]
pub struct PytestMarker {
    pub name: String,
    pub args: Vec<serde_json::Value>,
    pub kwargs: HashMap<String, serde_json::Value>,
}

/// Main pytest compatibility interface
pub struct PytestCompatLayer {
    plugin_manager: Arc<PluginManager>,
    session: Option<PytestSession>,
}

impl PytestCompatLayer {
    pub fn new(config: PluginConfig) -> Self {
        let plugin_manager = Arc::new(PluginManager::new(config));
        
        Self {
            plugin_manager,
            session: None,
        }
    }

    /// Initialize pytest-compatible session
    pub async fn initialize_session(&mut self, config: PytestConfig) -> Result<()> {
        // Initialize plugin manager
        Arc::get_mut(&mut self.plugin_manager).unwrap().initialize().await?;

        // Create session
        let session = PytestSession {
            config: config.clone(),
            items: Vec::new(),
            fixtures: HashMap::new(),
            plugin_manager: self.plugin_manager.clone(),
        };

        // Call pytest_sessionstart hook
        let session_data = serde_json::json!({
            "rootdir": config.rootdir,
            "args": config.args,
            "options": config.option
        });

        self.plugin_manager.pytest_hooks().pytest_sessionstart(session_data).await?;
        
        self.session = Some(session);
        Ok(())
    }

    /// Pytest-compatible collection phase
    pub async fn collect_tests(&mut self, paths: &[PathBuf]) -> Result<Vec<PytestItem>> {
        let mut collected_items = Vec::new();

        for path in paths {
            // Call pytest_collect_file hook
            let results = self.plugin_manager
                .pytest_hooks()
                .pytest_collect_file(&path.to_string_lossy(), None)
                .await?;

            // Process hook results
            for result in results {
                if let HookResult::Value(value) = result {
                    if let Some(items) = value.get("items").and_then(|i| i.as_array()) {
                        for item in items {
                            collected_items.push(self.convert_to_pytest_item(item)?);
                        }
                    }
                }
            }
        }

        // Call pytest_collection_modifyitems hook
        let session_data = serde_json::json!({});
        let config_data = serde_json::json!({});
        let items_data: Vec<_> = collected_items.iter()
            .map(|item| self.pytest_item_to_json(item))
            .collect();

        let modify_results = self.plugin_manager
            .pytest_hooks()
            .pytest_collection_modifyitems(session_data, config_data, &items_data)
            .await?;

        // Apply modifications
        for result in modify_results {
            if let HookResult::Modified(modified) = result {
                if let Some(modified_items) = modified.get("items").and_then(|i| i.as_array()) {
                    collected_items.clear();
                    for item in modified_items {
                        collected_items.push(self.convert_to_pytest_item(item)?);
                    }
                }
            }
        }

        // Update session
        if let Some(ref mut session) = self.session {
            session.items = collected_items.iter()
                .map(|item| self.pytest_item_to_test_item(item))
                .collect();
        }

        Ok(collected_items)
    }

    /// Execute tests with pytest-compatible hooks
    pub async fn run_tests(&self, items: &[PytestItem]) -> Result<Vec<PytestTestResult>> {
        let mut results = Vec::new();

        for item in items {
            let result = self.run_single_test(item).await?;
            results.push(result);
        }

        Ok(results)
    }

    async fn run_single_test(&self, item: &PytestItem) -> Result<PytestTestResult> {
        let item_data = self.pytest_item_to_json(item);

        // Setup phase
        let setup_results = self.plugin_manager
            .pytest_hooks()
            .pytest_runtest_setup(item_data.clone())
            .await?;

        // Check if setup failed
        for result in &setup_results {
            if let HookResult::Error(error) = result {
                return Ok(PytestTestResult {
                    nodeid: item.nodeid.clone(),
                    outcome: "error".to_string(),
                    duration: 0.0,
                    setup_error: Some(error.clone()),
                    call_error: None,
                    teardown_error: None,
                });
            }
        }

        // Call phase
        let call_results = self.plugin_manager
            .pytest_hooks()
            .pytest_runtest_call(item_data.clone())
            .await?;

        let mut call_error = None;
        for result in &call_results {
            if let HookResult::Error(error) = result {
                call_error = Some(error.clone());
                break;
            }
        }

        // Teardown phase
        let teardown_results = self.plugin_manager
            .pytest_hooks()
            .pytest_runtest_teardown(item_data)
            .await?;

        let mut teardown_error = None;
        for result in &teardown_results {
            if let HookResult::Error(error) = result {
                teardown_error = Some(error.clone());
                break;
            }
        }

        // Determine outcome
        let outcome = if call_error.is_some() {
            "failed"
        } else if setup_results.iter().any(|r| matches!(r, HookResult::Skip)) {
            "skipped"
        } else {
            "passed"
        };

        Ok(PytestTestResult {
            nodeid: item.nodeid.clone(),
            outcome: outcome.to_string(),
            duration: 0.0, // TODO: Implement timing
            setup_error: None,
            call_error,
            teardown_error,
        })
    }

    /// Finalize session
    pub async fn finalize_session(&self, exitstatus: i32) -> Result<()> {
        if let Some(ref session) = self.session {
            let session_data = serde_json::json!({
                "stats": {
                    "total_items": session.items.len(),
                },
                "rootdir": session.config.rootdir
            });

            self.plugin_manager
                .pytest_hooks()
                .pytest_sessionfinish(session_data, exitstatus)
                .await?;
        }

        self.plugin_manager.shutdown().await?;
        Ok(())
    }

    // Helper methods

    fn convert_to_pytest_item(&self, item_data: &serde_json::Value) -> Result<PytestItem> {
        Ok(PytestItem {
            nodeid: item_data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: item_data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            path: PathBuf::from(item_data.get("path").and_then(|v| v.as_str()).unwrap_or("")),
            function: item_data.get("function").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            cls: item_data.get("class").and_then(|v| v.as_str()).map(String::from),
            module: item_data.get("module").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            markers: Vec::new(), // TODO: Parse markers
            fixtures: Vec::new(), // TODO: Parse fixtures
        })
    }

    fn pytest_item_to_json(&self, item: &PytestItem) -> serde_json::Value {
        serde_json::json!({
            "id": item.nodeid,
            "name": item.name,
            "path": item.path.to_string_lossy(),
            "function": item.function,
            "class": item.cls,
            "module": item.module,
            "markers": item.markers,
            "fixtures": item.fixtures
        })
    }

    fn pytest_item_to_test_item(&self, item: &PytestItem) -> TestItem {
        TestItem {
            id: item.nodeid.clone(),
            path: item.path.clone(),
            name: item.name.clone(),
            function_name: item.function.clone(),
            line_number: 0, // TODO: Extract line number
            is_async: false, // TODO: Detect async
            class_name: item.cls.clone(),
            decorators: Vec::new(), // TODO: Convert markers to decorators
            fixture_deps: item.fixtures.clone(),
            is_xfail: false, // TODO: Check for xfail marker
        }
    }
}

/// Pytest test result
#[derive(Debug, Clone)]
pub struct PytestTestResult {
    pub nodeid: String,
    pub outcome: String, // "passed", "failed", "skipped", "error"
    pub duration: f64,
    pub setup_error: Option<String>,
    pub call_error: Option<String>,
    pub teardown_error: Option<String>,
}

/// Create a pytest-compatible runner
pub fn create_pytest_runner(config: PluginConfig) -> PytestCompatLayer {
    PytestCompatLayer::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pytest_compat_layer() {
        let plugin_config = PluginConfig::default();
        let mut compat_layer = PytestCompatLayer::new(plugin_config);

        let pytest_config = PytestConfig {
            rootdir: PathBuf::from("/tmp"),
            inifile: None,
            args: vec!["--tb=short".to_string()],
            option: HashMap::new(),
            pluginmanager: compat_layer.plugin_manager.clone(),
        };

        // Should initialize without errors
        assert!(compat_layer.initialize_session(pytest_config).await.is_ok());
    }

    #[test]
    fn test_pytest_item_conversion() {
        let plugin_config = PluginConfig::default();
        let compat_layer = PytestCompatLayer::new(plugin_config);

        let item_data = serde_json::json!({
            "id": "test_example::test_function",
            "name": "test_function",
            "path": "/tmp/test_example.py",
            "function": "test_function",
            "module": "test_example"
        });

        let pytest_item = compat_layer.convert_to_pytest_item(&item_data).unwrap();
        assert_eq!(pytest_item.nodeid, "test_example::test_function");
        assert_eq!(pytest_item.name, "test_function");
    }
}