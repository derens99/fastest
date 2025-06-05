//! pytest-mock compatibility plugin
//!
//! Provides the `mocker` fixture and mock functionality.

use std::sync::Arc;
use parking_lot::RwLock;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::api::{Plugin, PluginMetadata, PluginBuilder, PluginResult};
use crate::impl_plugin;

/// pytest-mock compatibility plugin
pub struct MockPlugin {
    metadata: PluginMetadata,
    mock_module: Arc<RwLock<Option<PyObject>>>,
}

impl MockPlugin {
    pub fn new() -> Self {
        let metadata = PluginBuilder::new("fastest.pytest_mock")
            .version("0.1.0")
            .description("pytest-mock compatibility - provides mocker fixture")
            .requires("fastest.fixtures")
            .priority(50)
            .build();
        
        Self {
            metadata,
            mock_module: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize the mock module
    fn init_mock_module(&self, py: Python) -> PyResult<PyObject> {
        // Try to import unittest.mock or mock
        let mock_module = if let Ok(module) = PyModule::import(py, "unittest.mock") {
            module
        } else if let Ok(module) = PyModule::import(py, "mock") {
            module
        } else {
            // Create a minimal mock module
            PyModule::new(py, "fastest_mock")?
        };
        
        Ok(mock_module.into())
    }
}

impl std::fmt::Debug for MockPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockPlugin")
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl Plugin for MockPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn initialize(&mut self) -> PluginResult<()> {
        Python::with_gil(|py| {
            let mock_module = self.init_mock_module(py)
                .map_err(|e| crate::api::PluginError::InitializationFailed(
                    format!("Failed to init mock module: {}", e)
                ))?;
            
            *self.mock_module.write() = Some(mock_module);
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

/// Mocker fixture implementation
pub fn create_mocker_fixture(py: Python) -> PyResult<PyObject> {
    let code = r#"
class MockerFixture:
    """pytest-mock compatible mocker fixture"""
    
    def __init__(self):
        import sys
        
        # Import mock module
        if sys.version_info >= (3, 3):
            from unittest import mock
        else:
            import mock
        
        self._mock_module = mock
        self._patches = []
        self._mocks = []
    
    def patch(self, target, *args, **kwargs):
        """Create a mock patch"""
        p = self._mock_module.patch(target, *args, **kwargs)
        mock_obj = p.start()
        self._patches.append(p)
        self._mocks.append(mock_obj)
        return mock_obj
    
    def patch_object(self, obj, attribute, *args, **kwargs):
        """Patch an object's attribute"""
        p = self._mock_module.patch.object(obj, attribute, *args, **kwargs)
        mock_obj = p.start()
        self._patches.append(p)
        self._mocks.append(mock_obj)
        return mock_obj
    
    def patch_multiple(self, target, *args, **kwargs):
        """Patch multiple attributes"""
        p = self._mock_module.patch.multiple(target, *args, **kwargs)
        result = p.start()
        self._patches.append(p)
        return result
    
    def patch_dict(self, in_dict, values=(), clear=False, **kwargs):
        """Patch a dictionary"""
        p = self._mock_module.patch.dict(in_dict, values, clear, **kwargs)
        p.start()
        self._patches.append(p)
        return p
    
    def spy(self, obj, name):
        """Create a spy - a mock that wraps the real object"""
        method = getattr(obj, name)
        autospec = inspect.ismethod(method) or inspect.isfunction(method)
        
        spy_obj = self.patch.object(
            obj, name,
            side_effect=method,
            autospec=autospec,
        )
        spy_obj.spy_return = None
        spy_obj.spy_exception = None
        
        def wrapper(*args, **kwargs):
            try:
                r = method(*args, **kwargs)
                spy_obj.spy_return = r
                return r
            except Exception as e:
                spy_obj.spy_exception = e
                raise
        
        spy_obj.side_effect = wrapper
        return spy_obj
    
    def stub(self, name=None):
        """Create a stub (mock with no behavior)"""
        return self._mock_module.MagicMock(name=name)
    
    def mock(self, *args, **kwargs):
        """Create a mock object"""
        return self._mock_module.Mock(*args, **kwargs)
    
    def magic_mock(self, *args, **kwargs):
        """Create a MagicMock object"""
        return self._mock_module.MagicMock(*args, **kwargs)
    
    def property_mock(self, *args, **kwargs):
        """Create a PropertyMock"""
        return self._mock_module.PropertyMock(*args, **kwargs)
    
    def async_mock(self, *args, **kwargs):
        """Create an AsyncMock (Python 3.8+)"""
        if hasattr(self._mock_module, 'AsyncMock'):
            return self._mock_module.AsyncMock(*args, **kwargs)
        else:
            # Fallback for older Python versions
            return self._mock_module.MagicMock(*args, **kwargs)
    
    def stop_all(self):
        """Stop all patches"""
        for p in reversed(self._patches):
            p.stop()
        self._patches.clear()
        self._mocks.clear()
    
    def reset_all(self):
        """Reset all mocks"""
        for m in self._mocks:
            if hasattr(m, 'reset_mock'):
                m.reset_mock()
    
    def __enter__(self):
        return self
    
    def __exit__(self, *args):
        self.stop_all()

# Create the fixture instance
import inspect
mocker = MockerFixture()
"#;
    
    let locals = PyDict::new(py);
    py.run(code, None, Some(locals))?;
    
    Ok(locals.get_item("mocker").unwrap().into())
}

/// Hook to register the mocker fixture
pub struct MockerFixtureHook;

impl crate::hooks::Hook for MockerFixtureHook {
    fn name(&self) -> &str {
        "pytest_fixture_setup"
    }
    
    fn execute(&self, mut args: crate::hooks::HookArgs) -> crate::hooks::HookResult<crate::hooks::HookReturn> {
        if let Some(fixture_name) = args.get::<String>("fixture_name") {
            if fixture_name == "mocker" {
                Python::with_gil(|py| {
                    match create_mocker_fixture(py) {
                        Ok(mocker) => {
                            // Return the mocker fixture
                            Ok(crate::hooks::HookReturn::Value(Box::new(mocker)))
                        }
                        Err(e) => {
                            eprintln!("Failed to create mocker fixture: {}", e);
                            Ok(crate::hooks::HookReturn::None)
                        }
                    }
                })
            } else {
                Ok(crate::hooks::HookReturn::None)
            }
        } else {
            Ok(crate::hooks::HookReturn::None)
        }
    }
}

impl std::fmt::Debug for MockerFixtureHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockerFixtureHook").finish()
    }
}