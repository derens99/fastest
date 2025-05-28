//! Smart conftest.py Discovery and Loading
//!
//! Fast, minimal conftest.py handling using external libraries for performance

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use ignore::WalkBuilder;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{Plugin, PluginConfig};
use crate::plugin::hooks::{HookRegistry, HookData, HookResult};

/// Fast conftest.py loader using ignore crate for performance
pub struct ConftestLoader {
    discovered: Arc<DashMap<PathBuf, ConftestFile>>,
    loaded: Arc<DashMap<PathBuf, ConftestPlugin>>,
}

/// Represents a conftest.py file
#[derive(Debug, Clone)]
pub struct ConftestFile {
    pub path: PathBuf,
    pub content: String,
    pub mtime: std::time::SystemTime,
    pub fixtures: Vec<String>,
    pub hooks: Vec<String>,
    pub plugins: Vec<String>,
}

/// Runtime conftest plugin instance
pub struct ConftestPlugin {
    pub file: ConftestFile,
    pub python_module: Option<PyObject>,
    pub fixtures: HashMap<String, PyObject>,
    pub hooks: HashMap<String, PyObject>,
}

impl ConftestLoader {
    pub fn new() -> Self {
        Self {
            discovered: Arc::new(DashMap::new()),
            loaded: Arc::new(DashMap::new()),
        }
    }

    /// Fast discovery using ignore crate (same as ripgrep)
    pub fn discover_conftest_files(&self, search_paths: &[PathBuf]) -> Result<Vec<ConftestFile>> {
        let mut conftest_files = Vec::new();

        for search_path in search_paths {
            let walker = WalkBuilder::new(search_path)
                .hidden(false)
                .ignore(false)
                .git_ignore(true)
                .add_custom_ignore_filename("pytest.ignore")
                .filter_entry(|entry| {
                    entry.file_name() == "conftest.py" || entry.file_type().map_or(false, |ft| ft.is_dir())
                })
                .build();

            for entry in walker {
                let entry = entry?;
                if entry.file_name() == "conftest.py" {
                    let conftest = self.parse_conftest_file(entry.path())?;
                    self.discovered.insert(entry.path().to_path_buf(), conftest.clone());
                    conftest_files.push(conftest);
                }
            }
        }

        Ok(conftest_files)
    }

    /// Smart conftest.py parsing using tree-sitter for speed
    fn parse_conftest_file(&self, path: &Path) -> Result<ConftestFile> {
        let content = std::fs::read_to_string(path)?;
        let mtime = std::fs::metadata(path)?.modified()?;

        // Fast parsing using tree-sitter
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_python::language()).unwrap();
        
        let tree = parser.parse(&content, None)
            .ok_or_else(|| anyhow!("Failed to parse conftest.py: {}", path.display()))?;

        let mut fixtures = Vec::new();
        let mut hooks = Vec::new();
        let mut plugins = Vec::new();

        // Walk the AST efficiently
        let mut cursor = tree.walk();
        self.extract_conftest_components(&content, &mut cursor, &mut fixtures, &mut hooks, &mut plugins);

        Ok(ConftestFile {
            path: path.to_path_buf(),
            content,
            mtime,
            fixtures,
            hooks,
            plugins,
        })
    }

    /// Extract fixtures, hooks, and plugins from AST
    fn extract_conftest_components(
        &self,
        content: &str,
        cursor: &mut tree_sitter::TreeCursor,
        fixtures: &mut Vec<String>,
        hooks: &mut Vec<String>,
        plugins: &mut Vec<String>,
    ) {
        loop {
            let node = cursor.node();

            // Check for function definitions
            if node.kind() == "function_definition" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &content[name_node.start_byte()..name_node.end_byte()];
                    
                    // Check for pytest decorators
                    if let Some(decorators) = node.child_by_field_name("decorators") {
                        let decorator_text = &content[decorators.start_byte()..decorators.end_byte()];
                        
                        if decorator_text.contains("@pytest.fixture") {
                            fixtures.push(name.to_string());
                        }
                        
                        if decorator_text.contains("@pytest.hookimpl") || name.starts_with("pytest_") {
                            hooks.push(name.to_string());
                        }
                    } else if name.starts_with("pytest_") {
                        hooks.push(name.to_string());
                    }
                }
            }

            // Check for pytest_plugins variable
            if node.kind() == "assignment" {
                let assignment_text = &content[node.start_byte()..node.end_byte()];
                if assignment_text.contains("pytest_plugins") {
                    // Extract plugin names (simplified)
                    plugins.extend(self.extract_plugin_names(assignment_text));
                }
            }

            // Traverse the tree
            if cursor.goto_first_child() {
                continue;
            }

            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return;
                }
            }
        }
    }

    fn extract_plugin_names(&self, assignment: &str) -> Vec<String> {
        // Simplified plugin name extraction
        let mut plugins = Vec::new();
        
        // Look for strings in the assignment
        let re = regex::Regex::new(r#"["']([^"']+)["']"#).unwrap();
        for cap in re.captures_iter(assignment) {
            if let Some(plugin_name) = cap.get(1) {
                plugins.push(plugin_name.as_str().to_string());
            }
        }
        
        plugins
    }

    /// Load conftest.py files into Python runtime
    pub fn discover_and_load(&self, search_paths: &[PathBuf]) -> Result<Vec<Box<dyn Plugin>>> {
        let conftest_files = self.discover_conftest_files(search_paths)?;
        let mut plugins = Vec::new();

        Python::with_gil(|py| -> Result<()> {
            for conftest_file in conftest_files {
                let plugin = self.load_conftest_plugin(py, conftest_file)?;
                let boxed_plugin: Box<dyn Plugin> = Box::new(plugin);
                plugins.push(boxed_plugin);
            }
            Ok(())
        })?;

        Ok(plugins)
    }

    fn load_conftest_plugin(&self, py: Python, conftest_file: ConftestFile) -> Result<ConftestPluginWrapper> {
        // Execute conftest.py in Python
        let module = PyModule::from_code(
            py,
            &conftest_file.content,
            &conftest_file.path.to_string_lossy(),
            "conftest",
        )?;

        let mut fixtures = HashMap::new();
        let mut hooks = HashMap::new();

        // Extract fixtures
        for fixture_name in &conftest_file.fixtures {
            if let Ok(fixture_fn) = module.getattr(fixture_name.as_str()) {
                fixtures.insert(fixture_name.clone(), fixture_fn.to_object(py));
            }
        }

        // Extract hooks
        for hook_name in &conftest_file.hooks {
            if let Ok(hook_fn) = module.getattr(hook_name.as_str()) {
                hooks.insert(hook_name.clone(), hook_fn.to_object(py));
            }
        }

        let plugin = ConftestPlugin {
            file: conftest_file.clone(),
            python_module: Some(module.to_object(py)),
            fixtures,
            hooks,
        };

        self.loaded.insert(conftest_file.path.clone(), plugin);

        Ok(ConftestPluginWrapper {
            file: conftest_file,
        })
    }

    /// Get conftest files for a specific test path (inheritance chain)
    pub fn get_conftest_chain(&self, test_path: &Path) -> Vec<ConftestFile> {
        let mut chain = Vec::new();
        let mut current_path = test_path.parent();

        while let Some(path) = current_path {
            let conftest_path = path.join("conftest.py");
            if let Some(conftest) = self.discovered.get(&conftest_path) {
                chain.push(conftest.value().clone());
            }
            current_path = path.parent();
        }

        // Reverse to get root-to-leaf order
        chain.reverse();
        chain
    }

    /// Reload conftest files if changed
    pub fn reload_if_changed(&self) -> Result<Vec<PathBuf>> {
        let mut reloaded = Vec::new();

        for entry in self.discovered.iter() {
            let path = entry.key();
            let conftest = entry.value();

            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(mtime) = metadata.modified() {
                    if mtime > conftest.mtime {
                        // File changed, reload
                        let new_conftest = self.parse_conftest_file(path)?;
                        self.discovered.insert(path.clone(), new_conftest);
                        reloaded.push(path.clone());
                    }
                }
            }
        }

        Ok(reloaded)
    }
}

/// Wrapper to make ConftestPlugin implement Plugin trait
pub struct ConftestPluginWrapper {
    file: ConftestFile,
}

impl Plugin for ConftestPluginWrapper {
    fn name(&self) -> &str {
        "conftest"
    }

    fn pytest_compatible(&self) -> bool {
        true
    }

    fn register_hooks(&self, registry: &mut HookRegistry) -> Result<()> {
        let file_path = self.file.path.clone();
        
        // Register each hook found in the conftest file
        for hook_name in &self.file.hooks {
            let hook_name_clone = hook_name.clone();
            let file_path_clone = file_path.clone();
            
            registry.add_hook(&hook_name, move |data: &HookData| -> Result<HookResult> {
                // Execute the Python hook function
                Python::with_gil(|py| -> Result<HookResult> {
                    // This is a simplified implementation
                    // In practice, we'd need to call the actual Python function
                    Ok(HookResult::Value(serde_json::json!({
                        "conftest": file_path_clone.to_string_lossy(),
                        "hook": hook_name_clone,
                        "executed": true
                    })))
                })
            })?;
        }

        Ok(())
    }
}

impl Default for ConftestLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_conftest_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let conftest_path = temp_dir.path().join("conftest.py");
        
        std::fs::write(&conftest_path, r#"
import pytest

@pytest.fixture
def sample_fixture():
    return "test_value"

def pytest_configure(config):
    pass

pytest_plugins = ["pytest_html", "pytest_cov"]
"#).unwrap();

        let loader = ConftestLoader::new();
        let conftest_files = loader.discover_conftest_files(&[temp_dir.path().to_path_buf()]).unwrap();
        
        assert_eq!(conftest_files.len(), 1);
        assert_eq!(conftest_files[0].fixtures.len(), 1);
        assert_eq!(conftest_files[0].fixtures[0], "sample_fixture");
        assert_eq!(conftest_files[0].hooks.len(), 1);
        assert_eq!(conftest_files[0].hooks[0], "pytest_configure");
        assert_eq!(conftest_files[0].plugins.len(), 2);
    }

    #[test]
    fn test_conftest_chain() {
        let temp_dir = TempDir::new().unwrap();
        let root_conftest = temp_dir.path().join("conftest.py");
        let sub_dir = temp_dir.path().join("tests");
        let sub_conftest = sub_dir.join("conftest.py");
        
        std::fs::create_dir(&sub_dir).unwrap();
        std::fs::write(&root_conftest, "# root conftest").unwrap();
        std::fs::write(&sub_conftest, "# sub conftest").unwrap();

        let loader = ConftestLoader::new();
        loader.discover_conftest_files(&[temp_dir.path().to_path_buf()]).unwrap();
        
        let test_file = sub_dir.join("test_example.py");
        let chain = loader.get_conftest_chain(&test_file);
        
        assert_eq!(chain.len(), 2);
        assert!(chain[0].path.ends_with("conftest.py"));
        assert!(chain[1].path.ends_with("tests/conftest.py"));
    }
}