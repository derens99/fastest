//! Plugin Loader - Dynamic plugin discovery and loading
//!
//! This module handles discovering and loading plugins from various sources:
//! - Python packages with entry points
//! - conftest.py files
//! - Native Rust plugins (.so/.dll/.dylib)
//! - Built-in plugins

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use crate::api::{Plugin, PluginError, PluginInfo, PluginMetadata, PluginResult, PluginType};
use crate::conftest::ConftestPlugin;

/// Plugin loader for dynamic plugin discovery
pub struct PluginLoader {
    /// Cache of loaded plugins
    cache: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    
    /// Python interpreter state
    python_initialized: bool,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            python_initialized: false,
        }
    }
    
    /// Initialize Python if needed
    fn ensure_python(&mut self) -> PyResult<()> {
        if !self.python_initialized {
            pyo3::prepare_freethreaded_python();
            self.python_initialized = true;
        }
        Ok(())
    }
    
    /// Discover plugins from Python entry points
    pub fn discover_entry_points(&mut self) -> PluginResult<Vec<PluginInfo>> {
        self.ensure_python()
            .map_err(|e| PluginError::InitializationFailed(format!("Python init failed: {}", e)))?;
        
        Python::with_gil(|py| {
            let mut discovered = Vec::new();
            
            // Import pkg_resources or importlib.metadata
            let discover_code = r#"
import sys
plugins = []

# Try importlib.metadata first (Python 3.8+)
try:
    from importlib import metadata
    for ep in metadata.entry_points().get('pytest11', []):
        plugins.append({
            'name': ep.name,
            'module': ep.value,
            'dist': str(ep.dist)
        })
except:
    # Fall back to pkg_resources
    try:
        import pkg_resources
        for ep in pkg_resources.iter_entry_points('pytest11'):
            plugins.append({
                'name': ep.name,
                'module': ep.module_name,
                'dist': str(ep.dist)
            })
    except:
        pass

# Also check for fastest-specific plugins
try:
    from importlib import metadata
    for ep in metadata.entry_points().get('fastest.plugins', []):
        plugins.append({
            'name': ep.name,
            'module': ep.value,
            'dist': str(ep.dist),
            'fastest': True
        })
except:
    pass

plugins
"#;
            
            let locals = PyDict::new(py);
            py.run(discover_code, None, Some(locals))
                .map_err(|e| PluginError::Other(anyhow::anyhow!("Discovery failed: {}", e)))?;
            
            let plugins: &PyList = locals.get_item("plugins")
                .and_then(|p| p.downcast::<PyList>().ok())
                .ok_or_else(|| PluginError::Other(anyhow::anyhow!("No plugins found")))?;
            
            for plugin_info in plugins {
                let info_dict: &PyDict = plugin_info.downcast()?;
                
                let name: String = info_dict.get_item("name")
                    .and_then(|n| n.extract().ok())
                    .unwrap_or_default();
                    
                let module: String = info_dict.get_item("module")
                    .and_then(|m| m.extract().ok())
                    .unwrap_or_default();
                    
                let dist: String = info_dict.get_item("dist")
                    .and_then(|d| d.extract().ok())
                    .unwrap_or_default();
                
                let is_fastest = info_dict.get_item("fastest")
                    .and_then(|f| f.extract::<bool>().ok())
                    .unwrap_or(false);
                
                discovered.push(PluginInfo {
                    metadata: PluginMetadata {
                        name: name.clone(),
                        version: "unknown".to_string(),
                        description: format!("Plugin from {}", dist),
                        ..Default::default()
                    },
                    plugin_type: PluginType::Python,
                    source: module,
                });
            }
            
            Ok(discovered)
        })
    }
    
    /// Load a specific plugin
    pub fn load_plugin(&mut self, info: &PluginInfo) -> PluginResult<Box<dyn Plugin>> {
        match info.plugin_type {
            PluginType::Python => self.load_python_plugin(info),
            PluginType::Native => self.load_native_plugin(info),
            PluginType::Conftest => self.load_conftest(&PathBuf::from(&info.source))
                .and_then(|mut plugins| plugins.pop()
                    .ok_or_else(|| PluginError::NotFound("No plugin in conftest".to_string()))),
            _ => Err(PluginError::Invalid("Unsupported plugin type".to_string())),
        }
    }
    
    /// Load a Python plugin
    fn load_python_plugin(&mut self, info: &PluginInfo) -> PluginResult<Box<dyn Plugin>> {
        self.ensure_python()
            .map_err(|e| PluginError::InitializationFailed(format!("Python init failed: {}", e)))?;
        
        Python::with_gil(|py| {
            // Import the module
            let module = PyModule::import(py, &info.source)
                .map_err(|e| PluginError::InitializationFailed(
                    format!("Failed to import {}: {}", info.source, e)
                ))?;
            
            // Create a wrapper plugin
            Ok(Box::new(PythonPluginWrapper::new(
                info.metadata.clone(),
                module.into(),
            )) as Box<dyn Plugin>)
        })
    }
    
    /// Load a native Rust plugin
    fn load_native_plugin(&mut self, info: &PluginInfo) -> PluginResult<Box<dyn Plugin>> {
        // Use dlopen2 to load the plugin
        let lib = unsafe {
            dlopen2::raw::Library::new(&info.source)
                .map_err(|e| PluginError::InitializationFailed(
                    format!("Failed to load library: {}", e)
                ))?
        };
        
        // Look for the plugin entry point
        type PluginEntry = unsafe extern "C" fn() -> *mut dyn Plugin;
        
        let entry: dlopen2::raw::Symbol<PluginEntry> = unsafe {
            lib.get(b"fastest_plugin_entry\0")
                .map_err(|e| PluginError::InitializationFailed(
                    format!("Plugin entry point not found: {}", e)
                ))?
        };
        
        let plugin_ptr = unsafe { entry() };
        let plugin = unsafe { Box::from_raw(plugin_ptr) };
        
        // Keep library loaded
        std::mem::forget(lib);
        
        Ok(plugin)
    }
    
    /// Find all conftest.py files
    pub fn find_conftest_files(&self) -> PluginResult<Vec<PathBuf>> {
        let mut conftest_files = Vec::new();
        let mut current_dir = std::env::current_dir()
            .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to get current dir: {}", e)))?;
        
        // Walk up directory tree looking for conftest.py
        loop {
            let conftest_path = current_dir.join("conftest.py");
            if conftest_path.exists() {
                conftest_files.push(conftest_path);
            }
            
            // Also check in test directories
            for test_dir in &["tests", "test", "testing"] {
                let test_conftest = current_dir.join(test_dir).join("conftest.py");
                if test_conftest.exists() && !conftest_files.contains(&test_conftest) {
                    conftest_files.push(test_conftest);
                }
            }
            
            // Move up one directory
            if !current_dir.pop() {
                break;
            }
        }
        
        // Reverse to get root conftest first
        conftest_files.reverse();
        Ok(conftest_files)
    }
    
    /// Load plugins from conftest.py
    pub fn load_conftest(&mut self, path: &Path) -> PluginResult<Vec<Box<dyn Plugin>>> {
        self.ensure_python()
            .map_err(|e| PluginError::InitializationFailed(format!("Python init failed: {}", e)))?;
        
        Python::with_gil(|py| {
            let conftest_plugin = ConftestPlugin::load(py, path)?;
            Ok(vec![Box::new(conftest_plugin) as Box<dyn Plugin>])
        })
    }
    
    /// Load plugins from a directory
    pub fn load_from_directory(&mut self, dir: &Path) -> PluginResult<Vec<Box<dyn Plugin>>> {
        let mut plugins = Vec::new();
        
        if !dir.exists() || !dir.is_dir() {
            return Ok(plugins);
        }
        
        // Look for Python plugins
        for entry in std::fs::read_dir(dir)
            .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read dir: {}", e)))?
        {
            let entry = entry
                .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) == Some("py") {
                // Python plugin
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    let info = PluginInfo {
                        metadata: PluginMetadata {
                            name: name.to_string(),
                            ..Default::default()
                        },
                        plugin_type: PluginType::Python,
                        source: path.to_string_lossy().to_string(),
                    };
                    
                    match self.load_plugin(&info) {
                        Ok(plugin) => plugins.push(plugin),
                        Err(e) => eprintln!("Failed to load {}: {}", path.display(), e),
                    }
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                // Native plugin
                if matches!(ext, "so" | "dll" | "dylib") {
                    let info = PluginInfo {
                        metadata: PluginMetadata {
                            name: path.file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            ..Default::default()
                        },
                        plugin_type: PluginType::Native,
                        source: path.to_string_lossy().to_string(),
                    };
                    
                    match self.load_plugin(&info) {
                        Ok(plugin) => plugins.push(plugin),
                        Err(e) => eprintln!("Failed to load {}: {}", path.display(), e),
                    }
                }
            }
        }
        
        Ok(plugins)
    }
}

/// Wrapper for Python plugins
struct PythonPluginWrapper {
    metadata: PluginMetadata,
    module: PyObject,
}

impl PythonPluginWrapper {
    fn new(metadata: PluginMetadata, module: PyObject) -> Self {
        Self { metadata, module }
    }
}

impl std::fmt::Debug for PythonPluginWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythonPluginWrapper")
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl Plugin for PythonPluginWrapper {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn initialize(&mut self) -> PluginResult<()> {
        Python::with_gil(|py| {
            let module = self.module.as_ref(py);
            
            // Call pytest_configure if it exists
            if let Ok(configure_fn) = module.getattr("pytest_configure") {
                configure_fn.call0()
                    .map_err(|e| PluginError::InitializationFailed(
                        format!("pytest_configure failed: {}", e)
                    ))?;
            }
            
            Ok(())
        })
    }
    
    fn shutdown(&mut self) -> PluginResult<()> {
        Python::with_gil(|py| {
            let module = self.module.as_ref(py);
            
            // Call pytest_unconfigure if it exists
            if let Ok(unconfigure_fn) = module.getattr("pytest_unconfigure") {
                unconfigure_fn.call0()
                    .map_err(|e| PluginError::InitializationFailed(
                        format!("pytest_unconfigure failed: {}", e)
                    ))?;
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