//! Conftest Plugin Support - Load plugins from conftest.py files
//!
//! This module provides support for pytest's conftest.py plugin system,
//! allowing users to define hooks and fixtures in Python.

use std::path::Path;
use std::collections::HashMap;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use crate::api::{Plugin, PluginMetadata, PluginError, PluginResult};
use crate::hooks::{Hook, HookArgs, HookReturn, HookResult};

/// A plugin loaded from conftest.py
pub struct ConftestPlugin {
    metadata: PluginMetadata,
    module: PyObject,
    hooks: HashMap<String, PyObject>,
}

impl ConftestPlugin {
    /// Load a conftest.py file
    pub fn load(py: Python, path: &Path) -> PluginResult<Self> {
        // Read the conftest.py file
        let code = std::fs::read_to_string(path)
            .map_err(|e| PluginError::InitializationFailed(
                format!("Failed to read conftest.py: {}", e)
            ))?;
        
        // Create a module name from the path
        let module_name = path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("conftest");
        
        // Execute the conftest.py code
        let module = PyModule::from_code(py, &code, &format!("{}.py", module_name), module_name)
            .map_err(|e| PluginError::InitializationFailed(
                format!("Failed to execute conftest.py: {}", e)
            ))?;
        
        // Discover hooks in the module
        let hooks = Self::discover_hooks(py, module)?;
        
        let metadata = PluginMetadata {
            name: format!("conftest:{}", path.display()),
            version: "0.1.0".to_string(),
            description: format!("Conftest plugin from {}", path.display()),
            priority: -100, // Conftest plugins run after built-in plugins
            ..Default::default()
        };
        
        Ok(Self {
            metadata,
            module: module.into(),
            hooks,
        })
    }
    
    /// Discover pytest hooks in the module
    fn discover_hooks(py: Python, module: &PyModule) -> PyResult<HashMap<String, PyObject>> {
        let mut hooks = HashMap::new();
        
        // List of known pytest hooks
        let hook_names = [
            "pytest_configure",
            "pytest_unconfigure",
            "pytest_sessionstart",
            "pytest_sessionfinish",
            "pytest_collection_start",
            "pytest_collection_modifyitems",
            "pytest_collection_finish",
            "pytest_runtest_protocol",
            "pytest_runtest_setup",
            "pytest_runtest_call",
            "pytest_runtest_teardown",
            "pytest_runtest_makereport",
            "pytest_exception_interact",
            "pytest_fixture_setup",
            "pytest_fixture_post_finalizer",
            "pytest_make_parametrize_id",
            "pytest_generate_tests",
        ];
        
        // Check for each hook
        for hook_name in &hook_names {
            if let Ok(hook_fn) = module.getattr(hook_name) {
                if hook_fn.is_callable() {
                    hooks.insert(hook_name.to_string(), hook_fn.into());
                }
            }
        }
        
        // Also discover fixture functions
        let locals = PyDict::new(py);
        locals.set_item("module", module)?;
        
        py.run(r#"
import inspect

fixtures = {}
for name, obj in inspect.getmembers(module):
    # Check if it's a fixture
    if hasattr(obj, '_pytestfixturefunction'):
        fixtures[name] = {
            'func': obj,
            'scope': getattr(obj._pytestfixturefunction, 'scope', 'function'),
            'autouse': getattr(obj._pytestfixturefunction, 'autouse', False),
            'params': getattr(obj._pytestfixturefunction, 'params', None),
        }
    elif name.startswith('fixture_') or name.endswith('_fixture'):
        # Heuristic for fixtures without decorator
        if callable(obj) and not name.startswith('_'):
            fixtures[name] = {
                'func': obj,
                'scope': 'function',
                'autouse': False,
                'params': None,
            }
"#, None, Some(locals))?;
        
        // Store fixture definitions as a special hook
        if let Ok(fixtures) = locals.get_item("fixtures") {
            hooks.insert("_fixtures".to_string(), fixtures.into());
        }
        
        Ok(hooks)
    }
    
    /// Get a hook implementation
    pub fn get_hook(&self, name: &str) -> Option<PyObject> {
        self.hooks.get(name).cloned()
    }
    
    /// Call a Python hook
    pub fn call_hook(&self, py: Python, name: &str, args: &HookArgs) -> HookResult<HookReturn> {
        if let Some(hook_fn) = self.hooks.get(name) {
            // Convert HookArgs to Python kwargs
            let kwargs = PyDict::new(py);
            
            // TODO: Properly convert HookArgs to Python arguments
            // For now, just call with no args
            
            match hook_fn.call0(py) {
                Ok(result) => {
                    // Convert Python result to HookReturn
                    if result.is_none(py) {
                        Ok(HookReturn::None)
                    } else if let Ok(b) = result.extract::<bool>(py) {
                        Ok(HookReturn::Bool(b))
                    } else if let Ok(s) = result.extract::<String>(py) {
                        Ok(HookReturn::String(s))
                    } else {
                        Ok(HookReturn::None)
                    }
                }
                Err(e) => Err(HookError::ExecutionFailed(format!("Python hook failed: {}", e))),
            }
        } else {
            Ok(HookReturn::None)
        }
    }
}

impl std::fmt::Debug for ConftestPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConftestPlugin")
            .field("metadata", &self.metadata)
            .field("hooks", &self.hooks.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Plugin for ConftestPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn initialize(&mut self) -> PluginResult<()> {
        Python::with_gil(|py| {
            // Call pytest_configure if it exists
            if let Err(e) = self.call_hook(py, "pytest_configure", &HookArgs::new()) {
                eprintln!("pytest_configure failed: {}", e);
            }
            Ok(())
        })
    }
    
    fn shutdown(&mut self) -> PluginResult<()> {
        Python::with_gil(|py| {
            // Call pytest_unconfigure if it exists
            if let Err(e) = self.call_hook(py, "pytest_unconfigure", &HookArgs::new()) {
                eprintln!("pytest_unconfigure failed: {}", e);
            }
            Ok(())
        })
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Conftest loader for discovering and loading conftest.py files
pub struct ConftestLoader {
    /// Loaded conftest modules by path
    loaded: HashMap<std::path::PathBuf, ConftestPlugin>,
}

impl ConftestLoader {
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
        }
    }
    
    /// Load all conftest.py files in the path hierarchy
    pub fn load_hierarchy(&mut self, start_path: &Path) -> PluginResult<Vec<&ConftestPlugin>> {
        let mut plugins = Vec::new();
        let mut current = start_path.to_path_buf();
        
        // Walk up the directory tree
        loop {
            let conftest_path = current.join("conftest.py");
            if conftest_path.exists() && !self.loaded.contains_key(&conftest_path) {
                Python::with_gil(|py| {
                    match ConftestPlugin::load(py, &conftest_path) {
                        Ok(plugin) => {
                            self.loaded.insert(conftest_path.clone(), plugin);
                        }
                        Err(e) => {
                            eprintln!("Failed to load {}: {}", conftest_path.display(), e);
                        }
                    }
                });
            }
            
            if self.loaded.contains_key(&conftest_path) {
                plugins.push(&self.loaded[&conftest_path]);
            }
            
            // Move up one directory
            if !current.pop() {
                break;
            }
        }
        
        // Return in order from root to leaf
        plugins.reverse();
        Ok(plugins)
    }
    
    /// Get fixtures from all loaded conftest files
    pub fn collect_fixtures(&self, py: Python) -> HashMap<String, PyObject> {
        let mut all_fixtures = HashMap::new();
        
        for plugin in self.loaded.values() {
            if let Some(fixtures_obj) = plugin.get_hook("_fixtures") {
                if let Ok(fixtures_dict) = fixtures_obj.downcast::<PyDict>(py) {
                    for (name, fixture_info) in fixtures_dict {
                        if let Ok(name_str) = name.extract::<String>() {
                            all_fixtures.insert(name_str, fixture_info.into());
                        }
                    }
                }
            }
        }
        
        all_fixtures
    }
}

/// Create a conftest hook wrapper
pub struct ConftestHook {
    plugin: ConftestPlugin,
    hook_name: String,
}

impl ConftestHook {
    pub fn new(plugin: ConftestPlugin, hook_name: String) -> Self {
        Self { plugin, hook_name }
    }
}

impl Hook for ConftestHook {
    fn name(&self) -> &str {
        &self.hook_name
    }
    
    fn execute(&self, args: HookArgs) -> HookResult<HookReturn> {
        Python::with_gil(|py| {
            self.plugin.call_hook(py, &self.hook_name, &args)
        })
    }
}

impl std::fmt::Debug for ConftestHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConftestHook")
            .field("hook_name", &self.hook_name)
            .finish()
    }
}