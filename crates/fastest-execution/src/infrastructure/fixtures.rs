//! Fixture execution integration
//! 
//! This module integrates the advanced fixture system from fastest-core
//! with the Python execution layer.

use anyhow::Result;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fastest_core::test::fixtures::{
    AdvancedFixtureManager, FixtureRequest, FixtureScope,
    ConftestDiscovery, generate_builtin_fixture_code, is_builtin_fixture,
};
use fastest_core::TestItem;

/// Fixture executor that bridges Rust fixture management with Python execution
pub struct FixtureExecutor {
    /// Advanced fixture manager from core
    manager: Arc<AdvancedFixtureManager>,
    /// Conftest discovery
    conftest_discovery: Arc<Mutex<ConftestDiscovery>>,
    /// Python fixture implementations cache
    fixture_implementations: Arc<Mutex<HashMap<String, PyObject>>>,
    /// Active fixture instances by scope
    active_instances: Arc<Mutex<HashMap<String, PyObject>>>,
}

impl FixtureExecutor {
    pub fn new(project_root: PathBuf) -> Result<Self> {
        let manager = Arc::new(AdvancedFixtureManager::new());
        let mut conftest_discovery = ConftestDiscovery::new()?;
        
        // Discover and register all conftest fixtures
        let conftest_files = conftest_discovery.discover_conftest_files(&project_root)?;
        for conftest_path in conftest_files {
            let conftest = conftest_discovery.parse_conftest(&conftest_path)?;
            for fixture in conftest.fixtures {
                manager.register_fixture(fixture)?;
            }
        }
        
        Ok(Self {
            manager,
            conftest_discovery: Arc::new(Mutex::new(conftest_discovery)),
            fixture_implementations: Arc::new(Mutex::new(HashMap::new())),
            active_instances: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    /// Initialize Python fixture environment
    pub fn initialize_python_fixtures(&self, py: Python) -> PyResult<PyObject> {
        let fixture_module = PyModule::from_code(
            py,
            &self.get_fixture_runtime_code(),
            "fastest_fixtures",
            "fastest_fixtures",
        )?;
        
        // Register built-in fixtures
        self.register_builtin_fixtures(py, &fixture_module)?;
        
        Ok(fixture_module.into())
    }
    
    /// Setup fixtures for a test
    pub fn setup_test_fixtures(
        &self,
        py: Python,
        test: &TestItem,
        fixture_module: &PyModule,
    ) -> PyResult<PyObject> {
        let request = FixtureRequest::from_test_item(test);
        
        // Get all required fixtures (including autouse)
        let _required_fixtures = self.manager.get_required_fixtures(&request)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        
        // Create fixture values dict
        let fixture_values = PyDict::new(py);
        
        // Setup fixtures in dependency order
        let sorted_fixtures = self.manager.setup_fixtures(&request)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        
        for (name, _value) in sorted_fixtures {
            let py_value = self.execute_fixture(py, &name, &request, fixture_module)?;
            fixture_values.set_item(name, py_value)?;
        }
        
        Ok(fixture_values.into())
    }
    
    /// Execute a single fixture
    fn execute_fixture(
        &self,
        py: Python,
        name: &str,
        request: &FixtureRequest,
        fixture_module: &PyModule,
    ) -> PyResult<PyObject> {
        // Check if it's a built-in fixture
        if is_builtin_fixture(name) {
            return self.execute_builtin_fixture(py, name, request);
        }
        
        // Get fixture definition
        let fixture_def = self.manager.get_fixture_info(name)
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err(
                format!("Fixture '{}' not found", name)
            ))?;
        
        // Check cache based on scope
        let cache_key = format!("{}::{}", name, request.get_scope_id(fixture_def.scope));
        
        let instances = self.active_instances.lock().unwrap();
        if let Some(cached) = instances.get(&cache_key) {
            return Ok(cached.clone());
        }
        drop(instances);
        
        // Execute fixture function
        let fixture_fn = fixture_module.getattr(&*name)?;
        
        // Prepare fixture arguments (dependencies)
        let kwargs = PyDict::new(py);
        for dep in &fixture_def.dependencies {
            let dep_value = self.execute_fixture(py, dep, request, fixture_module)?;
            kwargs.set_item(dep, dep_value)?;
        }
        
        // Handle parametrized fixtures
        let result = if !fixture_def.params.is_empty() && request.param_index.is_some() {
            let param_index = request.param_index.unwrap();
            if param_index < fixture_def.params.len() {
                let param_value = &fixture_def.params[param_index];
                let py_request = Py::new(py, PyFixtureRequest::from_request(request, Some(param_value)))?;
                kwargs.set_item("request", py_request)?;
            }
            fixture_fn.call((), Some(kwargs))?
        } else if fixture_def.dependencies.contains(&"request".to_string()) {
            let py_request = Py::new(py, PyFixtureRequest::from_request(request, None))?;
            kwargs.set_item("request", py_request)?;
            fixture_fn.call((), Some(kwargs))?
        } else {
            fixture_fn.call((), Some(kwargs))?
        };
        
        // Cache based on scope
        let mut instances = self.active_instances.lock().unwrap();
        instances.insert(cache_key, result.clone().into());
        
        Ok(result.into())
    }
    
    /// Execute built-in fixture
    fn execute_builtin_fixture(
        &self,
        py: Python,
        name: &str,
        _request: &FixtureRequest,
    ) -> PyResult<PyObject> {
        match name {
            "tmp_path" => {
                let tmp_path_class = py.eval(
                    "type('TmpPath', (), {'__init__': lambda self: setattr(self, 'path', __import__('tempfile').mkdtemp())})",
                    None,
                    None,
                )?;
                let instance = tmp_path_class.call0()?;
                Ok(instance.getattr("path")?.into())
            }
            "capsys" => {
                let capsys_code = r#"
import sys
from io import StringIO

class Capsys:
    def __init__(self):
        self._stdout = StringIO()
        self._stderr = StringIO()
        self._old_stdout = None
        self._old_stderr = None
    
    def _start(self):
        self._old_stdout = sys.stdout
        self._old_stderr = sys.stderr
        sys.stdout = self._stdout
        sys.stderr = self._stderr
    
    def _stop(self):
        if self._old_stdout:
            sys.stdout = self._old_stdout
        if self._old_stderr:
            sys.stderr = self._old_stderr
    
    def readouterr(self):
        out = self._stdout.getvalue()
        err = self._stderr.getvalue()
        self._stdout.seek(0)
        self._stdout.truncate()
        self._stderr.seek(0)
        self._stderr.truncate()
        
        class Output:
            def __init__(self, out, err):
                self.out = out
                self.err = err
        
        return Output(out, err)

capsys = Capsys()
capsys._start()
capsys
"#;
                Ok(py.eval(capsys_code, None, None)?.into())
            }
            "monkeypatch" => {
                let monkeypatch_code = generate_builtin_fixture_code("monkeypatch")
                    .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Missing monkeypatch code"))?;
                let locals = PyDict::new(py);
                py.run(&monkeypatch_code, None, Some(locals))?;
                let mp_class = locals.get_item("MonkeyPatch").unwrap();
                Ok(mp_class.call0()?.into())
            }
            _ => Err(pyo3::exceptions::PyRuntimeError::new_err(
                format!("Unknown built-in fixture: {}", name)
            ))
        }
    }
    
    /// Teardown fixtures after test
    pub fn teardown_test_fixtures(
        &self,
        _py: Python,
        test: &TestItem,
        scope: FixtureScope,
    ) -> PyResult<()> {
        let request = FixtureRequest::from_test_item(test);
        
        // Clear Python instances for the scope
        let scope_id = request.get_scope_id(scope);
        let mut instances = self.active_instances.lock().unwrap();
        
        let keys_to_remove: Vec<_> = instances.keys()
            .filter(|k| k.ends_with(&scope_id))
            .cloned()
            .collect();
        
        for key in keys_to_remove {
            instances.remove(&key);
        }
        
        // Teardown in the fixture manager
        self.manager.teardown_fixtures(&request, scope)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        
        Ok(())
    }
    
    /// Register built-in fixtures
    fn register_builtin_fixtures(&self, py: Python, module: &PyModule) -> PyResult<()> {
        // Register tmp_path
        let tmp_path_code = generate_builtin_fixture_code("tmp_path").unwrap();
        py.run(&tmp_path_code, None, Some(module.dict()))?;
        
        // Register capsys
        let capsys_code = generate_builtin_fixture_code("capsys").unwrap();
        py.run(&capsys_code, None, Some(module.dict()))?;
        
        // Register monkeypatch
        let monkeypatch_code = generate_builtin_fixture_code("monkeypatch").unwrap();
        py.run(&monkeypatch_code, None, Some(module.dict()))?;
        
        Ok(())
    }
    
    /// Get Python runtime code for fixtures
    fn get_fixture_runtime_code(&self) -> String {
        r#"
import sys
import inspect
from typing import Any, Dict, List, Optional

class FixtureRequest:
    """Pytest fixture request object"""
    def __init__(self, node_id: str, test_name: str, param=None):
        self.node_id = node_id
        self.test_name = test_name
        self.param = param
        self._fixture_defs = {}
    
    def getfixturevalue(self, name: str) -> Any:
        """Get fixture value by name"""
        # This would be implemented to get fixture from the manager
        pass

# Fixture registry
_fixture_registry = {}

def fixture(func=None, *, scope="function", params=None, autouse=False, ids=None):
    """Pytest fixture decorator"""
    def decorator(f):
        _fixture_registry[f.__name__] = {
            'func': f,
            'scope': scope,
            'params': params or [],
            'autouse': autouse,
            'ids': ids or [],
        }
        return f
    
    if func is not None:
        return decorator(func)
    return decorator

# Export pytest compatibility
class pytest:
    fixture = fixture

sys.modules['pytest'] = pytest
"#.to_string()
    }
}

/// Python fixture request object
#[pyclass]
struct PyFixtureRequest {
    node_id: String,
    test_name: String,
    param: Option<serde_json::Value>,
}

impl PyFixtureRequest {
    fn from_request(request: &FixtureRequest, param: Option<&serde_json::Value>) -> Self {
        Self {
            node_id: request.node_id.clone(),
            test_name: request.test_name.clone(),
            param: param.cloned(),
        }
    }
}

#[pymethods]
impl PyFixtureRequest {
    #[new]
    fn new(
        node_id: String,
        test_name: String,
        _param: Option<PyObject>,
    ) -> Self {
        // For the new method, we don't store the PyObject, just None
        // The actual param is set via from_request
        Self {
            node_id,
            test_name,
            param: None,
        }
    }
    
    #[getter]
    fn node_id(&self) -> &str {
        &self.node_id
    }
    
    #[getter]
    fn test_name(&self) -> &str {
        &self.test_name
    }
    
    #[getter]
    fn param(&self, py: Python) -> PyResult<PyObject> {
        match &self.param {
            Some(value) => {
                // Convert JSON value to Python object
                let json_str = value.to_string();
                Ok(py.eval(&format!("__import__('json').loads('{}')", json_str), None, None)?.into())
            }
            None => Ok(py.None()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_fixture_executor_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let executor = FixtureExecutor::new(temp_dir.path().to_path_buf())?;
        
        // Should create successfully
        assert!(Arc::strong_count(&executor.manager) == 1);
        
        Ok(())
    }
}