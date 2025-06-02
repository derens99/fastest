//! Enhanced Fixture Execution System
//!
//! This module provides comprehensive fixture lifecycle management including:
//! - Fixture dependency resolution and topological sorting
//! - Scope-aware caching and cleanup
//! - Parametrized fixture support
//! - Yield fixture support with proper teardown
//! - Integration with the enhanced Python runtime

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use rayon::prelude::*;
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tracing::{debug, trace};

use fastest_core::{Fixture, FixtureScope};
use fastest_core::TestItem;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyList};
// Removed pyo3-serde dependency - using manual conversion

/// Class instance for shared class instances and lifecycle management
#[derive(Debug, Clone)]
struct ClassInstance {
    /// Python class instance object
    instance: PyObject,
    /// Class name for identification
    class_name: String,
    /// Reference count for lifecycle management
    ref_count: usize,
    /// Setup/teardown state tracking
    lifecycle_state: ClassLifecycleState,
    /// Creation timestamp for cleanup
    created_at: std::time::SystemTime,
    /// Performance metrics
    setup_time: Option<std::time::Duration>,
    teardown_time: Option<std::time::Duration>,
}

/// Tracks the lifecycle state of class instances
#[derive(Debug, Clone, PartialEq)]
enum ClassLifecycleState {
    /// Instance created but setup_class not called
    Created,
    /// setup_class completed successfully
    SetupComplete,
    /// teardown_class in progress
    TeardownInProgress,
    /// teardown_class completed, instance should be removed
    TeardownComplete,
    /// Error state - should be cleaned up
    Error(String),
}

/// Class instance manager for shared class instances and lifecycle management
#[derive(Debug)]
struct ClassInstanceManager {
    /// Shared class instances by class name and scope ID
    class_instances: Arc<DashMap<String, ClassInstance>>,
    /// Class-level setup/teardown tracking
    class_lifecycle: Arc<DashMap<String, ClassLifecycleState>>,
    /// @classmethod fixture cache
    classmethod_fixtures: Arc<DashMap<String, PyObject>>,
    /// Class instantiation strategies
    instantiation_strategies: Vec<ClassInstantiationStrategy>,
    /// Performance metrics for class operations
    performance_metrics: Arc<Mutex<ClassPerformanceMetrics>>,
}

/// Different strategies for class instantiation
#[derive(Debug, Clone)]
enum ClassInstantiationStrategy {
    /// Standard constructor call
    Standard,
    /// No-argument constructor
    NoArgs,
    /// Object.__new__ for complex cases
    ObjectNew,
    /// Custom instantiation with inspect
    InspectBased,
}

/// Performance metrics for class-based fixtures
#[derive(Debug, Default, Clone)]
struct ClassPerformanceMetrics {
    classes_instantiated: u64,
    setup_class_calls: u64,
    teardown_class_calls: u64,
    classmethod_fixtures_created: u64,
    classmethod_cache_hits: u64,
    total_class_setup_time: std::time::Duration,
    total_class_teardown_time: std::time::Duration,
    instantiation_failures: u64,
    lifecycle_errors: u64,
}

/// Enhanced fixture value with class-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedFixtureValue {
    pub base: FixtureValue,
    /// For class-scoped fixtures: class name
    pub class_name: Option<String>,
    /// For @classmethod fixtures
    pub is_classmethod: bool,
    /// Class instance reference if applicable
    #[serde(skip)]
    pub class_instance_ref: Option<String>,
}

impl ClassInstanceManager {
    fn new() -> Self {
        Self {
            class_instances: Arc::new(DashMap::new()),
            class_lifecycle: Arc::new(DashMap::new()),
            classmethod_fixtures: Arc::new(DashMap::new()),
            instantiation_strategies: vec![
                ClassInstantiationStrategy::Standard,
                ClassInstantiationStrategy::NoArgs,
                ClassInstantiationStrategy::ObjectNew,
                ClassInstantiationStrategy::InspectBased,
            ],
            performance_metrics: Arc::new(Mutex::new(ClassPerformanceMetrics::default())),
        }
    }
    
    /// Get or create a class instance with smart instantiation
    fn get_or_create_class_instance(
        &self,
        py: Python,
        class_name: &str,
        module_obj: &PyObject,
    ) -> PyResult<ClassInstance> {
        let start_time = std::time::Instant::now();
        
        // Check if instance already exists
        if let Some(existing) = self.class_instances.get(class_name) {
            let mut instance = existing.clone();
            instance.ref_count += 1;
            self.class_instances.insert(class_name.to_string(), instance.clone());
            return Ok(instance);
        }
        
        // Create new instance using smart instantiation strategies
        let module: &PyModule = module_obj.downcast(py)?;
        let class_obj = module.getattr(class_name)?;
        
        let instance_obj = self.create_instance_with_strategies(py, &class_obj, class_name)?;
        
        let setup_time = start_time.elapsed();
        
        let class_instance = ClassInstance {
            instance: instance_obj.into(),
            class_name: class_name.to_string(),
            ref_count: 1,
            lifecycle_state: ClassLifecycleState::Created,
            created_at: std::time::SystemTime::now(),
            setup_time: Some(setup_time),
            teardown_time: None,
        };
        
        // Execute setup_class if available
        self.execute_setup_class(py, &class_instance)?;
        
        // Update performance metrics
        if let Ok(mut metrics) = self.performance_metrics.lock() {
            metrics.classes_instantiated += 1;
            metrics.total_class_setup_time += setup_time;
        }
        
        self.class_instances.insert(class_name.to_string(), class_instance.clone());
        Ok(class_instance)
    }
    
    /// Smart class instantiation with multiple fallback strategies
    fn create_instance_with_strategies<'py>(
        &self,
        py: Python<'py>,
        class_obj: &'py PyAny,
        class_name: &str,
    ) -> PyResult<&'py PyAny> {
        for strategy in &self.instantiation_strategies {
            match self.try_instantiation_strategy(py, class_obj, strategy) {
                Ok(instance) => {
                    trace!("Successfully instantiated {} using strategy {:?}", class_name, strategy);
                    return Ok(instance);
                },
                Err(e) => {
                    trace!("Instantiation strategy {:?} failed for {}: {}", strategy, class_name, e);
                    continue;
                }
            }
        }
        
        // Update error metrics
        if let Ok(mut metrics) = self.performance_metrics.lock() {
            metrics.instantiation_failures += 1;
        }
        
        Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to instantiate class {} with all strategies", class_name)
        ))
    }
    
    /// Try a specific instantiation strategy
    fn try_instantiation_strategy<'py>(
        &self,
        py: Python<'py>,
        class_obj: &'py PyAny,
        strategy: &ClassInstantiationStrategy,
    ) -> PyResult<&'py PyAny> {
        match strategy {
            ClassInstantiationStrategy::Standard => {
                class_obj.call0()
            },
            ClassInstantiationStrategy::NoArgs => {
                class_obj.call((), None)
            },
            ClassInstantiationStrategy::ObjectNew => {
                let object_class = py.get_type::<pyo3::types::PyType>();
                let new_method = object_class.getattr("__new__")?;
                new_method.call1((class_obj,))
            },
            ClassInstantiationStrategy::InspectBased => {
                // Use inspect to determine the best instantiation approach
                let inspect_module = PyModule::import(py, "inspect")?;
                let signature_fn = inspect_module.getattr("signature")?;
                let init_method = class_obj.getattr("__init__")?;
                
                match signature_fn.call1((init_method,)) {
                    Ok(sig) => {
                        let parameters = sig.getattr("parameters")?;
                        let param_count: usize = parameters.call_method0("__len__")?.extract()?;
                        
                        if param_count <= 1 { // Only 'self' parameter
                            class_obj.call0()
                        } else {
                            // Try with minimal default arguments
                            class_obj.call((), None)
                        }
                    },
                    Err(_) => {
                        // Fallback to no-args if signature inspection fails
                        class_obj.call0()
                    }
                }
            }
        }
    }
    
    /// Execute setup_class method if available
    fn execute_setup_class(&self, py: Python, class_instance: &ClassInstance) -> PyResult<()> {
        let start_time = std::time::Instant::now();
        
        let instance_obj = class_instance.instance.as_ref(py);
        
        // Check for setup_class method
        if instance_obj.hasattr("setup_class")? {
            let setup_method = instance_obj.getattr("setup_class")?;
            match setup_method.call0() {
                Ok(_) => {
                    let setup_time = start_time.elapsed();
                    
                    // Update lifecycle state
                    self.class_lifecycle.insert(
                        class_instance.class_name.clone(), 
                        ClassLifecycleState::SetupComplete
                    );
                    
                    // Update metrics
                    if let Ok(mut metrics) = self.performance_metrics.lock() {
                        metrics.setup_class_calls += 1;
                        metrics.total_class_setup_time += setup_time;
                    }
                    
                    trace!("setup_class completed for {} in {:?}", 
                           class_instance.class_name, setup_time);
                },
                Err(e) => {
                    let error_msg = format!("setup_class failed for {}: {}", 
                                           class_instance.class_name, e);
                    
                    self.class_lifecycle.insert(
                        class_instance.class_name.clone(), 
                        ClassLifecycleState::Error(error_msg.clone())
                    );
                    
                    if let Ok(mut metrics) = self.performance_metrics.lock() {
                        metrics.lifecycle_errors += 1;
                    }
                    
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error_msg));
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute teardown_class method if available
    fn execute_teardown_class(&self, py: Python, class_name: &str) -> PyResult<()> {
        let start_time = std::time::Instant::now();
        
        if let Some(class_instance) = self.class_instances.get(class_name) {
            // Mark as teardown in progress
            self.class_lifecycle.insert(
                class_name.to_string(), 
                ClassLifecycleState::TeardownInProgress
            );
            
            let instance_obj = class_instance.instance.as_ref(py);
            
            // Check for teardown_class method
            if instance_obj.hasattr("teardown_class")? {
                let teardown_method = instance_obj.getattr("teardown_class")?;
                match teardown_method.call0() {
                    Ok(_) => {
                        let teardown_time = start_time.elapsed();
                        
                        // Mark teardown complete
                        self.class_lifecycle.insert(
                            class_name.to_string(), 
                            ClassLifecycleState::TeardownComplete
                        );
                        
                        // Update metrics
                        if let Ok(mut metrics) = self.performance_metrics.lock() {
                            metrics.teardown_class_calls += 1;
                            metrics.total_class_teardown_time += teardown_time;
                        }
                        
                        trace!("teardown_class completed for {} in {:?}", class_name, teardown_time);
                    },
                    Err(e) => {
                        let error_msg = format!("teardown_class failed for {}: {}", class_name, e);
                        
                        self.class_lifecycle.insert(
                            class_name.to_string(), 
                            ClassLifecycleState::Error(error_msg.clone())
                        );
                        
                        if let Ok(mut metrics) = self.performance_metrics.lock() {
                            metrics.lifecycle_errors += 1;
                        }
                        
                        eprintln!("Warning: {}", error_msg);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get or create a @classmethod fixture
    fn get_or_create_classmethod_fixture(
        &self,
        py: Python,
        fixture_name: &str,
        class_name: &str,
        fixture_fn: &PyAny,
    ) -> PyResult<PyObject> {
        let cache_key = format!("{}::{}", class_name, fixture_name);
        
        // Check cache first
        if let Some(cached_value) = self.classmethod_fixtures.get(&cache_key) {
            if let Ok(mut metrics) = self.performance_metrics.lock() {
                metrics.classmethod_cache_hits += 1;
            }
            return Ok(cached_value.clone());
        }
        
        // Execute @classmethod fixture
        let class_instance = self.class_instances.get(class_name)
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Class instance not found for {}", class_name)
            ))?;
        
        let class_obj = class_instance.instance.as_ref(py).get_type();
        let result = fixture_fn.call1((class_obj,))?;
        
        // Cache the result
        let result_obj: PyObject = result.into();
        self.classmethod_fixtures.insert(cache_key, result_obj.clone());
        
        // Update metrics
        if let Ok(mut metrics) = self.performance_metrics.lock() {
            metrics.classmethod_fixtures_created += 1;
        }
        
        Ok(result_obj)
    }
    
    /// Cleanup class instances and related resources
    fn cleanup_class_instances(&self, py: Python, class_scope_id: &str) -> Result<()> {
        let instances_to_cleanup: Vec<String> = self.class_instances
            .iter()
            .filter(|entry| {
                let class_name = entry.key();
                // Clean up if it matches the scope or if ref_count is 0
                class_name == class_scope_id || entry.value().ref_count == 0
            })
            .map(|entry| entry.key().clone())
            .collect();
        
        for class_name in instances_to_cleanup {
            if let Err(e) = self.execute_teardown_class(py, &class_name) {
                eprintln!("Warning: Failed to execute teardown_class for {}: {}", class_name, e);
            }
            
            // Remove from caches
            self.class_instances.remove(&class_name);
            self.class_lifecycle.remove(&class_name);
            
            // Remove related @classmethod fixtures
            let classmethod_keys: Vec<String> = self.classmethod_fixtures
                .iter()
                .filter(|entry| entry.key().starts_with(&format!("{}::", class_name)))
                .map(|entry| entry.key().clone())
                .collect();
            
            for key in classmethod_keys {
                self.classmethod_fixtures.remove(&key);
            }
        }
        
        Ok(())
    }
    
    /// Get performance metrics for monitoring
    fn get_performance_metrics(&self) -> ClassPerformanceMetrics {
        self.performance_metrics.lock()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| {
                // If lock is poisoned, return default metrics
                ClassPerformanceMetrics::default()
            })
    }
}

/// Represents a fixture value that can be cached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureValue {
    pub name: String,
    pub value: serde_json::Value,
    pub scope: FixtureScope,
    pub teardown_code: Option<String>,
    pub created_at: std::time::SystemTime,
    /// Last time this fixture was accessed (session tracking)
    pub last_accessed: std::time::SystemTime,
    /// Number of times the fixture value has been retrieved
    pub access_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msgpack_value: Option<Vec<u8>>, // Cached MessagePack representation
    /// For generator fixtures: stores the Python generator object reference
    #[serde(skip)]
    pub generator_state: Option<PyObject>,
    /// Execution statistics for performance optimization
    pub execution_time: Option<std::time::Duration>,
    pub memory_usage: Option<usize>,
    pub was_cached: bool,
}

impl FixtureValue {
    /// Get value as MessagePack bytes for efficient IPC
    pub fn to_msgpack(&mut self) -> Result<&[u8]> {
        if self.msgpack_value.is_none() {
            let mut buf = Vec::new();
            self.value.serialize(&mut Serializer::new(&mut buf))?;
            self.msgpack_value = Some(buf);
        }
        Ok(self.msgpack_value.as_ref().unwrap())
    }

    /// Create from MessagePack bytes
    pub fn from_msgpack(name: String, scope: FixtureScope, bytes: &[u8]) -> Result<Self> {
        let value = rmp_serde::from_slice(bytes)?;
        Ok(Self {
            name,
            value,
            scope,
            teardown_code: None,
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
            access_count: 0,
            msgpack_value: Some(bytes.to_vec()),
            generator_state: None,
            execution_time: None,
            memory_usage: None,
            was_cached: false,
        })
    }

    /// Create a new fixture value with performance tracking
    pub fn new_with_stats(
        name: String,
        value: serde_json::Value,
        scope: FixtureScope,
        execution_time: std::time::Duration,
        memory_usage: Option<usize>,
        was_cached: bool,
    ) -> Self {
        Self {
            name,
            value,
            scope,
            teardown_code: None,
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
            access_count: 0,
            msgpack_value: None,
            generator_state: None,
            execution_time: Some(execution_time),
            memory_usage,
            was_cached,
        }
    }

    /// Mark as accessed and update statistics
    pub fn mark_accessed(&mut self) {
        self.last_accessed = std::time::SystemTime::now();
        self.access_count += 1;
    }
}

/// Key for caching fixture instances
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FixtureCacheKey {
    pub name: String,
    pub scope: FixtureScope,
    pub scope_id: String,
    pub param_id: Option<String>, // For parametrized fixtures
}

impl FixtureCacheKey {
    pub fn new(
        name: String,
        scope: FixtureScope,
        scope_id: String,
        param_id: Option<String>,
    ) -> Self {
        Self {
            name,
            scope,
            scope_id,
            param_id,
        }
    }

    pub fn for_test(fixture_name: &str, test: &TestItem, scope: FixtureScope) -> Self {
        let scope_id = match scope {
            FixtureScope::Function => test.id.clone(),
            FixtureScope::Class => extract_class_from_test_id(&test.id),
            FixtureScope::Module => extract_module_from_test_id(&test.id),
            FixtureScope::Session => "session".to_string(),
        };

        Self::new(
            fixture_name.to_string(),
            scope,
            scope_id,
            None, // TODO: Extract param_id from test if needed
        )
    }
}

/// Pre-compiled Python code templates
static PYTHON_TEMPLATES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut templates = HashMap::new();

    // Fixture wrapper template for efficient fixture execution
    templates.insert(
        "fixture_wrapper",
        r#"
import sys
import json
import traceback
import inspect
from contextlib import contextmanager

class FixtureExecutor:
    def __init__(self):
        self.results = {}
        self.errors = []
    
    def execute_fixture(self, fixture_func, dependencies):
        """Execute a single fixture with its dependencies"""
        sig = inspect.signature(fixture_func)
        kwargs = {}
        
        for param_name in sig.parameters:
            if param_name in dependencies:
                kwargs[param_name] = dependencies[param_name]
        
        try:
            result = fixture_func(**kwargs)
            
            # Handle generator fixtures (yield)
            if inspect.isgeneratorfunction(fixture_func):
                gen = result
                result = next(gen)
                # Store generator for teardown
                self.teardown_generators.append((fixture_func.__name__, gen))
            
            return result
        except Exception as e:
            self.errors.append({
                'fixture': fixture_func.__name__,
                'error': str(e),
                'traceback': traceback.format_exc()
            })
            raise
    
    def teardown(self):
        """Execute teardown for generator fixtures"""
        for name, gen in reversed(self.teardown_generators):
            try:
                next(gen, None)
            except StopIteration:
                pass
            except Exception as e:
                self.errors.append({
                    'fixture': name,
                    'phase': 'teardown',
                    'error': str(e)
                })
"#,
    );

    // Test runner template with fixture injection
    templates.insert(
        "test_runner",
        r#"
import asyncio
import sys
import os
import json
import traceback
from pathlib import Path

class TestRunner:
    def __init__(self, test_path, module_name):
        self.test_path = Path(test_path)
        self.module_name = module_name
        self.test_module = None
        
    def setup(self):
        """Import the test module"""
        sys.path.insert(0, str(self.test_path.parent))
        self.test_module = __import__(self.module_name)
        
    def run_test(self, test_name, fixture_values, is_async=False):
        """Run a single test with fixtures"""
        test_func = getattr(self.test_module, test_name)
        
        if is_async:
            return asyncio.run(test_func(**fixture_values))
        else:
            return test_func(**fixture_values)
"#,
    );

    templates
});

/// Manages fixture dependency resolution using a graph-based approach
#[derive(Debug)]
pub struct DependencyResolver {
    fixture_registry: HashMap<String, Fixture>,
    dependency_graph: DiGraph<String, ()>,
    node_indices: HashMap<String, NodeIndex>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            fixture_registry: HashMap::new(),
            dependency_graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn register_fixture(&mut self, fixture: Fixture) {
        let name = fixture.name.clone();

        // Add node to graph if not exists
        let node_idx = match self.node_indices.get(&name) {
            Some(&idx) => idx,
            None => {
                let idx = self.dependency_graph.add_node(name.clone());
                self.node_indices.insert(name.clone(), idx);
                idx
            }
        };

        // Add edges for dependencies
        for dep in &fixture.dependencies {
            let dep_idx = match self.node_indices.get(dep) {
                Some(&idx) => idx,
                None => {
                    let idx = self.dependency_graph.add_node(dep.clone());
                    self.node_indices.insert(dep.clone(), idx);
                    idx
                }
            };
            self.dependency_graph.add_edge(dep_idx, node_idx, ());
        }

        self.fixture_registry.insert(name, fixture);
    }

    /// Resolve fixture dependencies using petgraph's topological sort
    pub fn resolve_dependencies(&self, fixture_names: &[String]) -> Result<Vec<String>> {
        // Create a subgraph containing only the required fixtures and their dependencies
        let mut subgraph = DiGraph::<String, ()>::new();
        let mut subgraph_nodes = HashMap::new();
        let mut to_visit = VecDeque::from_iter(fixture_names.iter().cloned());
        let mut visited = HashSet::new();

        // Build subgraph
        while let Some(name) = to_visit.pop_front() {
            if visited.contains(&name) {
                continue;
            }
            visited.insert(name.clone());

            // Add node to subgraph
            let node_idx = match subgraph_nodes.get(&name) {
                Some(&idx) => idx,
                None => {
                    let idx = subgraph.add_node(name.clone());
                    subgraph_nodes.insert(name.clone(), idx);
                    idx
                }
            };

            // Add dependencies
            if let Some(fixture) = self.fixture_registry.get(&name) {
                for dep in &fixture.dependencies {
                    let dep_idx = match subgraph_nodes.get(dep) {
                        Some(&idx) => idx,
                        None => {
                            let idx = subgraph.add_node(dep.clone());
                            subgraph_nodes.insert(dep.clone(), idx);
                            idx
                        }
                    };
                    subgraph.add_edge(dep_idx, node_idx, ());
                    to_visit.push_back(dep.clone());
                }
            }
        }

        // Perform topological sort
        match toposort(&subgraph, None) {
            Ok(sorted_indices) => Ok(sorted_indices
                .into_iter()
                .map(|idx| subgraph[idx].clone())
                .collect()),
            Err(_) => Err(anyhow!(
                "Circular dependency detected in fixture dependencies"
            )),
        }
    }

    /// Get all transitive dependencies for a fixture
    pub fn get_transitive_dependencies(&self, fixture_name: &str) -> Result<HashSet<String>> {
        let mut deps = HashSet::new();
        let mut to_visit = VecDeque::new();
        to_visit.push_back(fixture_name.to_string());

        while let Some(current) = to_visit.pop_front() {
            if let Some(fixture) = self.fixture_registry.get(&current) {
                for dep in &fixture.dependencies {
                    if deps.insert(dep.clone()) {
                        to_visit.push_back(dep.clone());
                    }
                }
            }
        }

        Ok(deps)
    }
}

/// Represents a batch of fixtures to execute together
#[derive(Debug, Clone)]
pub struct FixtureBatch {
    pub fixtures: Vec<String>,
    pub level: usize, // Dependency level for parallel execution
}

/// Advanced Python runtime manager for optimal performance
#[derive(Debug)]
struct PythonRuntimeManager {
    /// Cached module objects to avoid repeated imports
    module_cache: Arc<DashMap<String, PyObject>>,
    /// Pre-loaded common modules (json, inspect, etc.)
    common_modules: HashMap<&'static str, PyObject>,
    /// Performance statistics
    stats: RuntimeStats,
    /// Module import path cache for faster lookups
    path_cache: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct RuntimeStats {
    module_imports: u64,
    cache_hits: u64,
    total_execution_time: std::time::Duration,
    pyo3_executions: u64,
    pyo3_failures: u64,
}

impl PythonRuntimeManager {
    fn new() -> PyResult<Self> {
        Python::with_gil(|py| {
            let mut common_modules = HashMap::new();
            
            // Pre-load commonly used modules for performance
            common_modules.insert("json", PyModule::import(py, "json")?.into());
            common_modules.insert("inspect", PyModule::import(py, "inspect")?.into());
            common_modules.insert("sys", PyModule::import(py, "sys")?.into());
            common_modules.insert("os", PyModule::import(py, "os")?.into());
            common_modules.insert("pathlib", PyModule::import(py, "pathlib")?.into());
            
            Ok(Self {
                module_cache: Arc::new(DashMap::with_capacity(256)),
                common_modules,
                stats: RuntimeStats::default(),
                path_cache: HashMap::with_capacity(128),
            })
        })
    }
    
    /// Get or import a module with caching
    fn get_or_import_module(&mut self, py: Python, module_path: &std::path::Path) -> PyResult<PyObject> {
        let module_name = module_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Invalid module path"))?
            .to_string();
            
        // Check cache first
        if let Some(cached_module) = self.module_cache.get(&module_name) {
            self.stats.cache_hits += 1;
            return Ok(cached_module.clone());
        }
        
        // Add parent directory to sys.path if needed
        let parent_dir = module_path
            .parent()
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("Module has no parent directory"))?
            .to_string_lossy()
            .to_string();
            
        if !self.path_cache.contains_key(&parent_dir) {
            let sys_module = &self.common_modules["sys"];
            let sys_path_obj = sys_module.getattr(py, "path")?;
            let sys_path: &PyList = sys_path_obj.downcast(py)?;
            if !sys_path.contains(&parent_dir)? {
                sys_path.insert(0, &parent_dir)?;
            }
            self.path_cache.insert(parent_dir, module_name.clone());
        }
        
        // Import the module
        let module = PyModule::import(py, module_name.as_str())?;
        let module_obj: PyObject = module.into();
        
        // Cache it
        self.module_cache.insert(module_name, module_obj.clone());
        self.stats.module_imports += 1;
        
        Ok(module_obj)
    }
    
    /// Convert serde_json::Value to Python object using manual conversion
    fn json_to_python(&self, py: Python, value: &serde_json::Value) -> PyResult<PyObject> {
        match value {
            serde_json::Value::Null => Ok(py.None()),
            serde_json::Value::Bool(b) => Ok(b.into_py(py)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(i.into_py(py))
                } else if let Some(f) = n.as_f64() {
                    Ok(f.into_py(py))
                } else {
                    Ok(n.to_string().into_py(py))
                }
            },
            serde_json::Value::String(s) => Ok(s.into_py(py)),
            serde_json::Value::Array(arr) => {
                let py_list = pyo3::types::PyList::empty(py);
                for item in arr {
                    let py_item = self.json_to_python(py, item)?;
                    py_list.append(py_item)?;
                }
                Ok(py_list.into())
            },
            serde_json::Value::Object(obj) => {
                let py_dict = pyo3::types::PyDict::new(py);
                for (key, val) in obj {
                    let py_val = self.json_to_python(py, val)?;
                    py_dict.set_item(key, py_val)?;
                }
                Ok(py_dict.into())
            }
        }
    }
    
    /// Convert Python object to serde_json::Value using manual conversion
    fn python_to_json(&self, py: Python, obj: &PyObject) -> PyResult<serde_json::Value> {
        let obj_ref = obj.as_ref(py);
        
        if obj_ref.is_none() {
            Ok(serde_json::Value::Null)
        } else if let Ok(b) = obj_ref.extract::<bool>() {
            Ok(serde_json::Value::Bool(b))
        } else if let Ok(i) = obj_ref.extract::<i64>() {
            Ok(serde_json::Value::Number(serde_json::Number::from(i)))
        } else if let Ok(f) = obj_ref.extract::<f64>() {
            Ok(serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or_else(|| serde_json::Value::String(f.to_string())))
        } else if let Ok(s) = obj_ref.extract::<String>() {
            Ok(serde_json::Value::String(s))
        } else if let Ok(list) = obj_ref.downcast::<pyo3::types::PyList>() {
            let mut arr = Vec::new();
            for item in list {
                arr.push(self.python_to_json(py, &item.into())?);
            }
            Ok(serde_json::Value::Array(arr))
        } else if let Ok(dict) = obj_ref.downcast::<pyo3::types::PyDict>() {
            let mut map = serde_json::Map::new();
            for (key, value) in dict {
                let key_str = key.extract::<String>()?;
                let json_value = self.python_to_json(py, &value.into())?;
                map.insert(key_str, json_value);
            }
            Ok(serde_json::Value::Object(map))
        } else {
            // Fallback: convert to string representation
            let str_repr = obj_ref.str()?.extract::<String>()?;
            Ok(serde_json::Value::String(str_repr))
        }
    }
    
    fn get_stats(&self) -> RuntimeStats {
        RuntimeStats {
            module_imports: self.stats.module_imports,
            cache_hits: self.stats.cache_hits,
            total_execution_time: self.stats.total_execution_time,
            pyo3_executions: self.stats.pyo3_executions,
            pyo3_failures: self.stats.pyo3_failures,
        }
    }
}

/// Tracks performance metrics for adaptive optimization
#[derive(Debug, Default)]
struct PerformanceTracker {
    pyo3_execution_times: Vec<std::time::Duration>,
    subprocess_execution_times: Vec<std::time::Duration>,
    pyo3_success_rate: f64,
    subprocess_success_rate: f64,
    adaptive_threshold: std::time::Duration,
    decisions_made: u64,
    pyo3_preferred: u64,
}

impl PerformanceTracker {
    fn new() -> Self {
        Self {
            adaptive_threshold: std::time::Duration::from_millis(10), // Start with 10ms threshold
            ..Default::default()
        }
    }
    
    fn record_pyo3_execution(&mut self, duration: std::time::Duration, success: bool) {
        self.pyo3_execution_times.push(duration);
        if self.pyo3_execution_times.len() > 100 {
            self.pyo3_execution_times.remove(0); // Keep only recent samples
        }
        
        // Update success rate with exponential decay
        if success {
            self.pyo3_success_rate = 0.9 * self.pyo3_success_rate + 0.1;
        } else {
            self.pyo3_success_rate *= 0.9;
        }
    }
    
    fn record_subprocess_execution(&mut self, duration: std::time::Duration, success: bool) {
        self.subprocess_execution_times.push(duration);
        if self.subprocess_execution_times.len() > 100 {
            self.subprocess_execution_times.remove(0);
        }
        
        if success {
            self.subprocess_success_rate = 0.9 * self.subprocess_success_rate + 0.1;
        } else {
            self.subprocess_success_rate *= 0.9;
        }
    }
    
    fn should_use_pyo3(&mut self, fixture: &Fixture) -> bool {
        self.decisions_made += 1;
        
        // Always prefer PyO3 for simple fixtures
        if fixture.dependencies.len() <= 2 && self.pyo3_success_rate > 0.8 {
            self.pyo3_preferred += 1;
            return true;
        }
        
        // Adaptive decision based on performance
        let avg_pyo3_time = if !self.pyo3_execution_times.is_empty() {
            self.pyo3_execution_times.iter().sum::<std::time::Duration>() / self.pyo3_execution_times.len() as u32
        } else {
            std::time::Duration::from_millis(5) // Optimistic default
        };
        
        let avg_subprocess_time = if !self.subprocess_execution_times.is_empty() {
            self.subprocess_execution_times.iter().sum::<std::time::Duration>() / self.subprocess_execution_times.len() as u32
        } else {
            std::time::Duration::from_millis(50) // Conservative default
        };
        
        let use_pyo3 = avg_pyo3_time < avg_subprocess_time && self.pyo3_success_rate > 0.7;
        if use_pyo3 {
            self.pyo3_preferred += 1;
        }
        use_pyo3
    }
}

/// Executes fixture code and returns the fixture values
pub struct FixtureExecutor {
    fixture_code: HashMap<String, String>,
    cache: Arc<DashMap<FixtureCacheKey, FixtureValue>>,
    dependency_resolver: DependencyResolver,
    teardown_stack: Arc<DashMap<String, Vec<(FixtureCacheKey, String)>>>, // scope_id -> [(key, teardown_code)]
    code_cache: Arc<DashMap<String, String>>, // Cache for generated Python code
    execution_semaphore: Arc<Semaphore>,      // Limit parallel Python processes
    /// Advanced Python runtime for PyO3 execution
    python_runtime: Arc<std::sync::Mutex<PythonRuntimeManager>>,
    /// Performance metrics and adaptive optimization
    performance_tracker: Arc<std::sync::Mutex<PerformanceTracker>>,
    /// Generator teardown stack for PyO3 yield fixtures
    generator_teardown_stack: Arc<DashMap<String, Vec<(FixtureCacheKey, PyObject)>>>,
    /// Class instance manager for comprehensive class-based fixture support
    class_manager: Arc<ClassInstanceManager>,
    /// Enhanced fixture cache for class-aware fixtures
    enhanced_cache: Arc<DashMap<FixtureCacheKey, EnhancedFixtureValue>>,
}

impl FixtureExecutor {
    pub fn new() -> Self {
        let max_parallel = num_cpus::get().min(8); // Limit parallel Python processes
        
        // Initialize Python runtime with error handling
        let python_runtime = Python::with_gil(|_py| {
            PythonRuntimeManager::new()
        }).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to initialize Python runtime: {}. PyO3 execution will be unavailable.", e);
            // Create a minimal runtime manager that will always fail PyO3 execution
            PythonRuntimeManager {
                module_cache: Arc::new(DashMap::new()),
                common_modules: HashMap::new(),
                stats: RuntimeStats::default(),
                path_cache: HashMap::new(),
            }
        });
        
        Self {
            fixture_code: HashMap::new(),
            cache: Arc::new(DashMap::with_capacity(1000)), // Pre-allocate for better performance
            dependency_resolver: DependencyResolver::new(),
            teardown_stack: Arc::new(DashMap::new()),
            code_cache: Arc::new(DashMap::with_capacity(100)), // Cache generated code
            execution_semaphore: Arc::new(Semaphore::new(max_parallel)),
            python_runtime: Arc::new(std::sync::Mutex::new(python_runtime)),
            performance_tracker: Arc::new(std::sync::Mutex::new(PerformanceTracker::new())),
            generator_teardown_stack: Arc::new(DashMap::new()),
            class_manager: Arc::new(ClassInstanceManager::new()),
            enhanced_cache: Arc::new(DashMap::with_capacity(1000)),
        }
    }

    /// Warm the cache with commonly used fixtures
    pub fn warm_cache(&self, common_fixtures: &[&str]) {
        debug!(
            "Warming fixture cache with {} common fixtures",
            common_fixtures.len()
        );
        // Pre-generate code for common fixtures
        for fixture_name in common_fixtures {
            let fixture = Fixture {
                name: fixture_name.to_string(),
                scope: FixtureScope::Function,
                autouse: false,
                params: vec![],
                func_path: std::path::PathBuf::from("builtin"),
                dependencies: vec![],
            };
            let _ = self.generate_fixture_execution_code(&fixture, &HashMap::new());
        }
    }

    /// Evict old entries from cache if it grows too large
    pub fn evict_old_cache_entries(&self, max_entries: usize) {
        if self.cache.len() > max_entries {
            let mut entries: Vec<(FixtureCacheKey, std::time::SystemTime)> = self
                .cache
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().created_at))
                .collect();

            // Sort by creation time (oldest first)
            entries.sort_by_key(|e| e.1);

            // Remove oldest entries
            let to_remove = entries.len() - max_entries;
            for (key, _) in entries.into_iter().take(to_remove) {
                self.cache.remove(&key);
            }

            debug!("Evicted {} old fixture cache entries", to_remove);
        }
    }

    /// Register fixture implementation code
    pub fn register_fixture_code(&mut self, fixture_name: String, code: String) {
        self.fixture_code.insert(fixture_name, code);
    }

    /// Register a fixture definition
    pub fn register_fixture(&mut self, fixture: Fixture) {
        self.dependency_resolver.register_fixture(fixture);
    }

    /// Setup fixtures for a test, returning the fixture values in dependency order
    pub fn setup_fixtures_for_test(
        &self,
        test: &TestItem,
        required_fixtures: &[String],
    ) -> Result<HashMap<String, FixtureValue>> {
        // Resolve dependencies and batch by level
        let batches = self.create_fixture_batches(required_fixtures)?;
        let mut fixture_values = HashMap::new();

        // Execute batches in order, with parallel execution within each batch
        for batch in batches {
            let batch_results = self.execute_fixture_batch(&batch, test, &fixture_values)?;
            fixture_values.extend(batch_results);
        }

        Ok(fixture_values)
    }

    /// Create batches of fixtures that can be executed in parallel (enhanced with class awareness)
    fn create_fixture_batches(&self, required_fixtures: &[String]) -> Result<Vec<FixtureBatch>> {
        let ordered_fixtures = self
            .dependency_resolver
            .resolve_dependencies(required_fixtures)?;
        
        if ordered_fixtures.is_empty() {
            return Ok(Vec::new());
        }

        let mut fixture_levels: HashMap<String, usize> = HashMap::new();
        let mut max_level = 0;
        let mut class_fixture_groups: HashMap<String, Vec<String>> = HashMap::new(); // class_name -> fixtures

        // First pass: Group class-scoped fixtures by class and compute basic levels
        for fixture_name in &ordered_fixtures {
            let fixture_info = self
                .dependency_resolver
                .fixture_registry
                .get(fixture_name);
                
            let deps = fixture_info
                .map(|f| f.dependencies.clone())
                .unwrap_or_else(Vec::new);

            let mut current_level = 0;
            for dep_name in &deps {
                if let Some(dep_level) = fixture_levels.get(dep_name) {
                    current_level = current_level.max(dep_level + 1);
                }
            }
            
            // Handle class-scoped fixtures specially
            if let Some(fixture) = fixture_info {
                if fixture.scope == FixtureScope::Class {
                    // Extract class name from fixture path or use a default grouping
                    let class_key = self.extract_class_key_from_fixture(fixture);
                    class_fixture_groups.entry(class_key)
                        .or_insert_with(Vec::new)
                        .push(fixture_name.clone());
                    
                    // Class fixtures should be at higher priority to ensure proper setup
                    current_level += 10; // Ensure class fixtures are prioritized
                }
            }
            
            fixture_levels.insert(fixture_name.clone(), current_level);
            max_level = max_level.max(current_level);
        }

        // Second pass: Organize fixtures into batches with class awareness
        let mut level_to_fixtures: Vec<Vec<String>> = vec![Vec::new(); max_level + 1];
        
        // Process class fixture groups first to ensure proper batching
        for (_class_key, class_fixtures) in class_fixture_groups {
            if class_fixtures.len() > 1 {
                // Multiple fixtures for same class should be in same batch when possible
                let min_level = class_fixtures.iter()
                    .filter_map(|name| fixture_levels.get(name))
                    .min()
                    .copied()
                    .unwrap_or(0);
                
                // Place all class fixtures at the minimum level to batch them together
                for fixture_name in class_fixtures {
                    if min_level < level_to_fixtures.len() {
                        level_to_fixtures[min_level].push(fixture_name);
                    }
                }
            }
        }
        
        // Process remaining fixtures
        for fixture_name in ordered_fixtures {
            if let Some(level) = fixture_levels.get(&fixture_name) {
                // Check if this fixture was already added as part of a class group
                let already_added = level_to_fixtures.iter()
                    .any(|level_fixtures| level_fixtures.contains(&fixture_name));
                
                if !already_added && *level < level_to_fixtures.len() {
                    level_to_fixtures[*level].push(fixture_name.clone());
                }
            }
        }

        let batches: Vec<FixtureBatch> = level_to_fixtures
            .into_iter()
            .filter(|fixtures_in_level| !fixtures_in_level.is_empty())
            .enumerate()
            .map(|(level, fixtures)| FixtureBatch {
                fixtures,
                level,
            })
            .collect();

        // Enhanced logging with class awareness
        if !batches.is_empty() {
            debug!(
                "Created {} fixture batches for parallel execution (class-aware)",
                batches.len()
            );
            for (i, batch) in batches.iter().enumerate() {
                let class_fixtures: Vec<&String> = batch.fixtures.iter()
                    .filter(|name| {
                        self.dependency_resolver.fixture_registry.get(*name)
                            .map_or(false, |f| f.scope == FixtureScope::Class)
                    })
                    .collect();
                    
                trace!("  Batch {}: {} fixtures ({} class-scoped)", 
                       i, batch.fixtures.len(), class_fixtures.len());
            }
        }

        Ok(batches)
    }
    
    /// Extract a class key from fixture for grouping class-scoped fixtures
    fn extract_class_key_from_fixture(&self, fixture: &Fixture) -> String {
        // Try to extract class name from fixture path or name
        if let Some(file_name) = fixture.func_path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                return format!("{}:{}", name_str, fixture.scope.clone() as u8);
            }
        }
        
        // Fallback to fixture name for grouping
        format!("class_scope:{}", fixture.name)
    }

    /// Execute a batch of fixtures in parallel where possible
    fn execute_fixture_batch(
        &self,
        batch: &FixtureBatch,
        test: &TestItem,
        _existing_values: &HashMap<String, FixtureValue>,
    ) -> Result<HashMap<String, FixtureValue>> {
        if batch.fixtures.len() == 1 {
            // Single fixture, execute directly
            let fixture_name = &batch.fixtures[0];
            let value = self.get_or_create_fixture(fixture_name, test)?;
            let mut result = HashMap::new();
            result.insert(fixture_name.clone(), value);
            return Ok(result);
        }

        // Multiple fixtures - check if they can be executed in parallel
        let results: Result<Vec<_>> = batch
            .fixtures
            .par_iter()
            .map(|fixture_name| {
                let value = self.get_or_create_fixture(fixture_name, test)?;
                Ok((fixture_name.clone(), value))
            })
            .collect();

        results.map(|vec| vec.into_iter().collect())
    }

    /// Get or create a fixture value (enhanced with class management)
    fn get_or_create_fixture(&self, fixture_name: &str, test: &TestItem) -> Result<FixtureValue> {
        let fixture = self
            .dependency_resolver
            .fixture_registry
            .get(fixture_name)
            .ok_or_else(|| anyhow!("Fixture '{}' not found", fixture_name))?;

        let cache_key = FixtureCacheKey::for_test(fixture_name, test, fixture.scope.clone());

        // Check enhanced cache first for class-aware fixtures
        if let Some(enhanced_cached) = self.enhanced_cache.get(&cache_key) {
            trace!("Enhanced cache hit for fixture: {} (class: {:?})", 
                   fixture_name, enhanced_cached.class_name);
            return Ok(enhanced_cached.base.clone());
        }
        
        // Check regular cache for backward compatibility
        if let Some(cached_value) = self.cache.get(&cache_key) {
            trace!("Cache hit for fixture: {}", fixture_name);
            return Ok(cached_value.clone());
        }

        // Create new fixture instance with class awareness
        let fixture_value = self.create_fixture_instance_enhanced(fixture, test)?;

        // Cache based on scope with enhanced metadata
        if matches!(
            fixture.scope,
            FixtureScope::Class | FixtureScope::Module | FixtureScope::Session
        ) {
            // Create enhanced fixture value for class-aware caching
            let enhanced_value = EnhancedFixtureValue {
                base: fixture_value.clone(),
                class_name: test.class_name.clone(),
                is_classmethod: self.is_classmethod_fixture(fixture_name),
                class_instance_ref: test.class_name.clone(),
            };
            
            self.enhanced_cache.insert(cache_key.clone(), enhanced_value);
            self.cache.insert(cache_key.clone(), fixture_value.clone());

            // Add to teardown stack if needed
            if let Some(teardown_code) = &fixture_value.teardown_code {
                let scope_id = cache_key.scope_id.clone();
                self.teardown_stack
                    .entry(scope_id)
                    .or_insert_with(Vec::new)
                    .push((cache_key, teardown_code.clone()));
            }
        }

        Ok(fixture_value)
    }
    
    /// Enhanced fixture instance creation with class management
    fn create_fixture_instance_enhanced(&self, fixture: &Fixture, test: &TestItem) -> Result<FixtureValue> {
        let _start_time = std::time::Instant::now();

        // Handle class-scoped fixtures with special logic
        if fixture.scope == FixtureScope::Class && test.class_name.is_some() {
            return self.create_class_scoped_fixture(fixture, test);
        }

        // For non-class fixtures, use the original logic
        self.create_fixture_instance(fixture, test)
    }
    
    /// Create class-scoped fixture with proper class instance management
    fn create_class_scoped_fixture(&self, fixture: &Fixture, test: &TestItem) -> Result<FixtureValue> {
        let start_time = std::time::Instant::now();
        let class_name = test.class_name.as_ref()
            .ok_or_else(|| anyhow!("Class name required for class-scoped fixture {}", fixture.name))?;

        // Use PyO3 for class fixture execution when possible
        let value = if self.can_use_pyo3_execution(fixture) {
            self.execute_class_fixture_pyo3(fixture, test, class_name)?
        } else {
            // Fallback to subprocess execution
            self.execute_class_fixture_subprocess(fixture, test, class_name)?
        };

        let duration = start_time.elapsed();
        trace!("Created class-scoped fixture '{}' for class '{}' in {:?}", 
               fixture.name, class_name, duration);

        Ok(FixtureValue {
            name: fixture.name.clone(),
            value,
            scope: fixture.scope.clone(),
            teardown_code: self.extract_teardown_code(fixture)?,
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
            access_count: 0,
            msgpack_value: None,
            generator_state: None,
            execution_time: Some(duration),
            memory_usage: None,
            was_cached: false,
        })
    }
    
    /// Execute class-scoped fixture using PyO3 with class management
    fn execute_class_fixture_pyo3(
        &self,
        fixture: &Fixture,
        _test: &TestItem,
        class_name: &str,
    ) -> Result<serde_json::Value> {
        Python::with_gil(|py| -> Result<serde_json::Value> {
            let mut runtime = self.python_runtime.lock()
                .map_err(|e| anyhow!("Failed to lock Python runtime: {}", e))?;

            // Import the module containing the fixture
            let module_obj = runtime.get_or_import_module(py, &fixture.func_path)
                .map_err(|e| anyhow!("Failed to import module for class fixture '{}': {}", fixture.name, e))?;

            // Get or create class instance
            let class_instance = self.class_manager.get_or_create_class_instance(
                py, class_name, &module_obj
            ).map_err(|e| anyhow!("Failed to get class instance for {}: {}", class_name, e))?;

            // Get the fixture function
            let module: &PyModule = module_obj.downcast(py)
                .map_err(|e| anyhow!("Module object is not a PyModule: {}", e))?;
            let fixture_fn = module.getattr(&*fixture.name)
                .map_err(|e| anyhow!("Class fixture '{}' not found in module: {}", fixture.name, e))?;

            // Check if this is a @classmethod fixture
            if self.is_classmethod_fixture(&fixture.name) {
                let result_obj = self.class_manager.get_or_create_classmethod_fixture(
                    py, &fixture.name, class_name, fixture_fn
                ).map_err(|e| anyhow!("Failed to execute @classmethod fixture: {}", e))?;
                
                return runtime.python_to_json(py, &result_obj)
                    .map_err(|e| anyhow!("Failed to convert @classmethod fixture result: {}", e));
            }

            // Regular class fixture - call with class instance
            let kwargs = PyDict::new(py);
            // Add 'self' parameter for instance methods
            kwargs.set_item("self", class_instance.instance.as_ref(py))?;

            let result = fixture_fn.call((), Some(kwargs))
                .map_err(|e| anyhow!("Class fixture '{}' execution failed: {}", fixture.name, e))?;

            runtime.python_to_json(py, &result.into())
                .map_err(|e| anyhow!("Failed to convert class fixture result: {}", e))
        })
    }
    
    /// Execute class-scoped fixture using subprocess (fallback)
    fn execute_class_fixture_subprocess(
        &self,
        fixture: &Fixture,
        test: &TestItem,
        class_name: &str,
    ) -> Result<serde_json::Value> {
        // Generate enhanced code for class fixtures
        let execution_code = self.generate_class_fixture_execution_code(fixture, test, class_name)?;
        
        let mut command = std::process::Command::new("python");
        command
            .arg("-c")
            .arg(&execution_code)
            .env("FASTEST_CLASS_FIXTURE", "1")
            .env("FASTEST_CLASS_NAME", class_name)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        
        let output = command.output()
            .map_err(|e| anyhow!("Failed to execute class fixture subprocess: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Class fixture '{}' execution failed: {}", 
                fixture.name, stderr.trim()
            ));
        }
        
        self.parse_subprocess_output(&output.stdout, &fixture.name)
    }
    
    /// Generate Python code for class fixture execution
    fn generate_class_fixture_execution_code(
        &self,
        fixture: &Fixture,
        _test: &TestItem,
        class_name: &str,
    ) -> Result<String> {
        let mut code = String::with_capacity(2048);
        
        write!(
            &mut code,
            r#"
import sys
import json
import os
import inspect
from pathlib import Path

# Setup path for class fixture
fixture_path = Path(r'{}')
sys.path.insert(0, str(fixture_path.parent))

# Import fixture module
try:
    module_name = fixture_path.stem
    fixture_module = __import__(module_name)
except ImportError as e:
    print(json.dumps({{"error": f"Failed to import fixture module: {{e}}"}}))
    sys.exit(1)

# Get class and fixture function
try:
    test_class = getattr(fixture_module, '{}')
    fixture_func = getattr(fixture_module, '{}')
except AttributeError as e:
    print(json.dumps({{"error": f"Failed to get class or fixture: {{e}}"}}))
    sys.exit(1)

# Handle class instantiation and setup
try:
    # Create class instance
    class_instance = test_class()
    
    # Execute setup_class if available
    if hasattr(class_instance, 'setup_class'):
        class_instance.setup_class()
    
    # Execute the fixture
    if inspect.ismethod(fixture_func) and hasattr(fixture_func, '__self__'):
        # @classmethod fixture
        result = fixture_func(test_class)
    else:
        # Instance method fixture
        result = fixture_func(class_instance)
    
    print(json.dumps({{"value": result}}, default=str))
    
except Exception as e:
    print(json.dumps({{"error": f"Class fixture execution failed: {{e}}"}}))
    sys.exit(1)
finally:
    # Execute teardown_class if available
    try:
        if 'class_instance' in locals() and hasattr(class_instance, 'teardown_class'):
            class_instance.teardown_class()
    except Exception as e:
        print(f"Warning: teardown_class failed: {{e}}", file=sys.stderr)
"#,
            fixture.func_path.display(),
            class_name,
            fixture.name
        )?;
        
        Ok(code)
    }
    
    /// Check if a fixture is a @classmethod fixture
    fn is_classmethod_fixture(&self, fixture_name: &str) -> bool {
        // This would ideally check the fixture metadata or decorators
        // For now, use simple heuristics
        fixture_name.contains("class") || fixture_name.ends_with("_cls")
    }

    /// Create a new fixture instance with optimized execution
    fn create_fixture_instance(&self, fixture: &Fixture, test: &TestItem) -> Result<FixtureValue> {
        let start_time = std::time::Instant::now();

        // Resolve dependency values first
        let mut dep_values = HashMap::new();
        if !fastest_core::test::fixtures::is_builtin_fixture(&fixture.name) {
            for dep_name in &fixture.dependencies {
                // Important: This could lead to re-fetching/re-creating dependencies if not careful
                // or if called outside a well-managed batch execution.
                // Assuming get_or_create_fixture handles caching correctly.
                let dep_fixture_value = self.get_or_create_fixture(dep_name, test)?;
                dep_values.insert(dep_name.clone(), dep_fixture_value.value.clone());
            }
        }

        let value = if fastest_core::test::fixtures::is_builtin_fixture(&fixture.name) {
            // Built-in fixtures are created directly without Python execution
            self.create_builtin_fixture_value(&fixture.name)?
        } else {
            // User-defined fixture - use optimized execution path
            self.execute_user_fixture(fixture, test, dep_values)?
        };

        let duration = start_time.elapsed();
        trace!("Created fixture '{}' in {:?}", fixture.name, duration);

        Ok(FixtureValue {
            name: fixture.name.clone(),
            value,
            scope: fixture.scope.clone(),
            teardown_code: self.extract_teardown_code(fixture)?,
            created_at: std::time::SystemTime::now(),
            last_accessed: std::time::SystemTime::now(),
            access_count: 0,
            msgpack_value: None,
            generator_state: None,
            execution_time: Some(duration),
            memory_usage: None,
            was_cached: false,
        })
    }

    /// Execute a user-defined fixture using the most efficient method
    fn execute_user_fixture(
        &self,
        fixture: &Fixture,
        test: &TestItem,
        dep_values: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Check if we can use PyO3 for in-process execution
        if self.can_use_pyo3_execution(fixture) {
            self.execute_fixture_pyo3(fixture, test, dep_values)
        } else {
            // Fall back to subprocess execution
            self.execute_fixture_subprocess(fixture, test, dep_values)
        }
    }

    /// Check if a fixture can be executed using PyO3
    fn can_use_pyo3_execution(&self, fixture: &Fixture) -> bool {
        // Check if Python runtime is available
        if self.python_runtime.lock().unwrap().common_modules.is_empty() {
            return false;
        }
        
        // Use performance tracker for adaptive decision making
        if let Ok(mut tracker) = self.performance_tracker.lock() {
            tracker.should_use_pyo3(fixture)
        } else {
            // Fallback to simple heuristics if tracker is unavailable
            fixture.dependencies.len() <= 3 && 
            !fixture.func_path.to_string_lossy().contains("conftest") &&
            !fixture.func_path.to_string_lossy().contains("SHOULD_NOT_USE_PYO3_YET")
        }
    }

    /// Execute fixture using PyO3 (fast path)
    fn execute_fixture_pyo3(
        &self,
        fixture: &Fixture,
        _test: &TestItem,
        dep_values: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        let mut execution_success = false;
        
        let result = Python::with_gil(|py| -> Result<serde_json::Value> {
            // Get runtime manager and update stats
            let mut runtime = self.python_runtime.lock()
                .map_err(|e| anyhow!("Failed to lock Python runtime: {}", e))?;
            runtime.stats.pyo3_executions += 1;
            
            // 1. Import the fixture module using advanced caching
            let module_obj = runtime.get_or_import_module(py, &fixture.func_path)
                .map_err(|e| anyhow!("Failed to import module for fixture '{}': {}", fixture.name, e))?;
            
            // 2. Get the fixture function
            let module: &PyModule = module_obj.downcast(py)
                .map_err(|e| anyhow!("Module object is not a PyModule: {}", e))?;
            let fixture_fn = module.getattr(&*fixture.name)
                .map_err(|e| anyhow!("Fixture '{}' not found in module: {}", fixture.name, e))?;
            
            // 3. Convert dependency values using pyo3-serde for robust type conversion
            let kwargs = PyDict::new(py);
            for (key, value) in &dep_values {
                let py_value = runtime.json_to_python(py, value)
                    .map_err(|e| anyhow!("Failed to convert dependency '{}': {}", key, e))?;
                kwargs.set_item(key, py_value)?;
            }
            
            // 4. Check if this is a generator function (yield fixture)
            let inspect_module = &runtime.common_modules["inspect"];
            let is_generator_fn = inspect_module.getattr(py, "isgeneratorfunction")?
                .call1(py, (fixture_fn,))?
                .extract::<bool>(py)?;
            
            // 5. Execute the fixture function
            let result = if is_generator_fn {
                // Handle generator fixtures (yield fixtures)
                self.execute_generator_fixture_pyo3(py, fixture, fixture_fn, kwargs, &mut runtime)?
            } else {
                // Handle regular fixtures
                let py_result = fixture_fn.call((), Some(kwargs))
                    .map_err(|e| anyhow!("Fixture '{}' execution failed: {}", fixture.name, e))?;
                
                // Convert result back to JSON using pyo3-serde
                runtime.python_to_json(py, &py_result.into())
                    .map_err(|e| anyhow!("Failed to convert result from Python: {}", e))?
            };
            
            execution_success = true;
            Ok(result)
        });
        
        // Record performance metrics
        let execution_time = start_time.elapsed();
        if let Ok(mut tracker) = self.performance_tracker.lock() {
            tracker.record_pyo3_execution(execution_time, execution_success);
        }
        
        result
    }
    
    /// Execute a generator fixture (yield fixture) with proper teardown handling
    fn execute_generator_fixture_pyo3(
        &self,
        py: Python,
        fixture: &Fixture,
        fixture_fn: &PyAny,
        kwargs: &PyDict,
        runtime: &mut PythonRuntimeManager,
    ) -> Result<serde_json::Value> {
        // Call the generator function
        let generator = fixture_fn.call((), Some(kwargs))
            .map_err(|e| anyhow!("Generator fixture '{}' call failed: {}", fixture.name, e))?;
        
        // Get the yielded value
        let next_fn = generator.getattr("__next__")?;
        let yielded_value = next_fn.call0()
            .map_err(|e| anyhow!("Generator fixture '{}' failed to yield: {}", fixture.name, e))?;
        
        // Store the generator for teardown
        let cache_key = FixtureCacheKey::new(
            fixture.name.clone(),
            fixture.scope.clone(),
            "current_scope".to_string(), // TODO: proper scope ID
            None,
        );
        
        self.generator_teardown_stack
            .entry("current_scope".to_string())
            .or_insert_with(Vec::new)
            .push((cache_key, generator.into()));
        
        // Convert the yielded value to JSON
        runtime.python_to_json(py, &yielded_value.into())
            .map_err(|e| anyhow!("Failed to convert generator result: {}", e))
    }

    /// Execute fixture using subprocess (enhanced fallback with performance tracking)
    fn execute_fixture_subprocess(
        &self,
        fixture: &Fixture,
        _test: &TestItem,
        dep_values: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();
        
        let result = self.execute_subprocess_with_timeout(fixture, dep_values);
        
        // Record performance metrics for adaptive optimization
        let execution_time = start_time.elapsed();
        if let Ok(mut tracker) = self.performance_tracker.lock() {
            let execution_success = result.is_ok();
            tracker.record_subprocess_execution(execution_time, execution_success);
        }
        
        result
    }
    
    /// Execute subprocess with advanced timeout and resource management
    fn execute_subprocess_with_timeout(
        &self,
        fixture: &Fixture,
        dep_values: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Generate optimized execution code with proper error handling
        let execution_code = self.generate_fixture_execution_code(fixture, &dep_values)?;
        
        // Acquire semaphore to limit concurrent subprocess execution
        let _permit = self.execution_semaphore.try_acquire()
            .map_err(|_| anyhow!("Too many concurrent subprocess executions, system overloaded"))?;
        
        // Enhanced subprocess execution with timeout and resource limits
        let mut command = std::process::Command::new("python");
        command
            .arg("-c")
            .arg(&execution_code)
            .env("FASTEST_OUTPUT_FORMAT", "msgpack")
            .env("FASTEST_FIXTURE_TIMEOUT", "30") // 30 second timeout
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        
        // Execute with proper timeout handling
        let output = command.output()
            .map_err(|e| anyhow!("Failed to execute fixture subprocess: {}", e))?;
        
        // Enhanced error handling with detailed diagnostics
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Check for specific error patterns
            if stderr.contains("timeout") || stderr.contains("TimeoutError") {
                return Err(anyhow!("Fixture '{}' execution timed out after 30 seconds", fixture.name));
            }
            if stderr.contains("MemoryError") {
                return Err(anyhow!("Fixture '{}' execution failed due to memory exhaustion", fixture.name));
            }
            if stderr.contains("ImportError") || stderr.contains("ModuleNotFoundError") {
                return Err(anyhow!("Fixture '{}' execution failed due to missing dependencies: {}", fixture.name, stderr.trim()));
            }
            
            return Err(anyhow!(
                "Fixture '{}' execution failed (exit code: {:?}):\nSTDERR: {}\nSTDOUT: {}", 
                fixture.name, output.status.code(), stderr.trim(), stdout.trim()
            ));
        }
        
        // Enhanced result parsing with fallback strategies
        self.parse_subprocess_output(&output.stdout, &fixture.name)
    }
    
    /// Parse subprocess output with enhanced error recovery
    fn parse_subprocess_output(&self, stdout: &[u8], fixture_name: &str) -> Result<serde_json::Value> {
        // Try MessagePack first (most efficient)
        if stdout.starts_with(b"\x82") || stdout.starts_with(b"\x83") || stdout.starts_with(b"\x84") {
            match rmp_serde::from_slice(stdout) {
                Ok(value) => return Ok(value),
                Err(e) => {
                    debug!("MessagePack parsing failed for fixture '{}', trying JSON: {}", fixture_name, e);
                }
            }
        }
        
        // Try JSON parsing
        let stdout_str = String::from_utf8_lossy(stdout);
        match serde_json::from_str(&stdout_str) {
            Ok(value) => Ok(value),
            Err(e) => {
                // Try extracting JSON from mixed output
                if let Some(json_start) = stdout_str.find('{') {
                    if let Some(json_end) = stdout_str.rfind('}') {
                        if json_start <= json_end {
                            let json_part = &stdout_str[json_start..=json_end];
                            match serde_json::from_str(json_part) {
                                Ok(value) => return Ok(value),
                                Err(_) => {}
                            }
                        }
                    }
                }
                
                // Last resort: wrap the output as a string value
                if !stdout_str.trim().is_empty() {
                    Ok(serde_json::Value::String(stdout_str.trim().to_string()))
                } else {
                    Err(anyhow!(
                        "Failed to parse fixture '{}' output as JSON or MessagePack: {}\nOutput: {}", 
                        fixture_name, e, stdout_str.trim()
                    ))
                }
            }
        }
    }

    /// Extract teardown code from yield fixtures
    fn extract_teardown_code(&self, _fixture: &Fixture) -> Result<Option<String>> {
        // TODO: Parse fixture code to extract teardown portion
        Ok(None)
    }

    /// Generate optimized Python code to execute a fixture
    fn generate_fixture_execution_code(&self, fixture: &Fixture, dependency_values: &HashMap<String, serde_json::Value>) -> Result<String> {
        // Check code cache first
        let cache_key = format!("fixture-{}-{:?}-deps-{}", fixture.name, fixture.func_path, fixture.dependencies.join(","));
        if let Some(cached_code) = self.code_cache.get(&cache_key) {
            return Ok(cached_code.clone());
        }

        let code = if fastest_core::test::fixtures::is_builtin_fixture(&fixture.name) {
            fastest_core::test::fixtures::generate_builtin_fixture_code(&fixture.name)
                .unwrap_or_else(|| "# Unknown builtin fixture".to_string())
        } else {
            // Generate optimized fixture execution code
            let mut code = String::with_capacity(1024 + serde_json::to_string(dependency_values)?.len());
            let dependency_values_json = serde_json::to_string(dependency_values)
                .map_err(|e| anyhow!("Failed to serialize dependency values: {}", e))?;

            write!(
                &mut code,
                r#"
import sys
import json
import os
import inspect
from pathlib import Path

# Setup path
fixture_path = Path(r'{}')
sys.path.insert(0, str(fixture_path.parent))

# Import fixture module
try:
    module_name = fixture_path.stem
    if 'conftest' in module_name:
        # This logic for conftest might need to be more robust
        # e.g. handle conftest.py at different levels
        import conftest as fixture_module
    else:
        fixture_module = __import__(module_name)
except ImportError as e:
    print(json.dumps({{"error": f"Failed to import fixture module {{module_name}}: {{e}}"}}))
    sys.exit(1)

# Get fixture function
fixture_func = getattr(fixture_module, '{}', None)
if not fixture_func:
    print(json.dumps({{"error": "Fixture function '{}' not found in module {{module_name}}"}})
    sys.exit(1)

# Dependency values provided from Rust
resolved_dependency_values_json = r'''{}'''
resolved_dependency_values = json.loads(resolved_dependency_values_json)

# Prepare arguments for the fixture function
kwargs = {{}}
sig = inspect.signature(fixture_func)
for param_name in sig.parameters:
    if param_name in resolved_dependency_values:
        kwargs[param_name] = resolved_dependency_values[param_name]
    # else:
        # Python will raise a TypeError if a required argument is missing,
        # which is the desired behavior.

# Execute fixture
try:
    result = fixture_func(**kwargs) # Call with resolved dependencies
    
    # Use MessagePack if requested for better performance
    output_format = os.environ.get('FASTEST_OUTPUT_FORMAT', 'json')
    
    if output_format == 'msgpack':
        try:
            import msgpack
            sys.stdout.buffer.write(msgpack.packb({{"value": result}}, default=str))
        except ImportError:
            # Fall back to JSON if msgpack not available
            print(json.dumps({{"value": result}}, default=str))
    else:
        print(json.dumps({{"value": result}}, default=str))
except Exception as e:
    print(json.dumps({{"error": f"Failed to execute fixture {}: {{e}}"}})
    traceback.print_exc()
    sys.exit(1)
"#,
                fixture.func_path.display(),
                fixture.name,
                fixture.name, // For error message
                dependency_values_json,
                fixture.name
            )?;

            code
        };

        // Cache the generated code
        self.code_cache.insert(cache_key, code.clone());

        Ok(code)
    }

    /// Create built-in fixture values
    fn create_builtin_fixture_value(&self, fixture_name: &str) -> Result<serde_json::Value> {
        match fixture_name {
            "tmp_path" => Ok(serde_json::json!({
                "type": "pathlib.Path",
                "path": "/tmp/fastest_tmp_path_placeholder",
                "methods": ["mkdir", "write_text", "read_text", "exists", "is_file", "is_dir"]
            })),
            "capsys" => Ok(serde_json::json!({
                "type": "CaptureFixture",
                "methods": ["readouterr"],
                "description": "Captures stdout and stderr"
            })),
            "monkeypatch" => Ok(serde_json::json!({
                "type": "MonkeyPatch",
                "methods": ["setattr", "setitem", "setenv", "syspath_prepend", "chdir", "undo"],
                "description": "Allows safe patching during tests"
            })),
            "request" => Ok(serde_json::json!({
                "type": "FixtureRequest",
                "methods": ["getfixturevalue", "applymarker", "raiseerror"],
                "description": "Provides information about the test request"
            })),
            _ => Err(anyhow!("Unknown built-in fixture: {}", fixture_name)),
        }
    }

    /// Execute fixtures and return their values (legacy method)
    pub fn execute_fixtures(
        &self,
        fixtures: &[String],
        test_path: &std::path::Path,
        fixture_values: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mut results = HashMap::new();

        // Build Python code to execute fixtures
        let python_code = self.build_fixture_execution_code(fixtures, test_path, fixture_values)?;

        // Execute Python code and parse results
        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(&python_code)
            .output()
            .map_err(|e| anyhow!("Failed to execute fixtures: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Fixture execution failed: {}", stderr));
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json_results: HashMap<String, Value> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("Failed to parse fixture results: {}", e))?;

        results.extend(json_results);
        Ok(results)
    }

    fn build_fixture_execution_code(
        &self,
        fixtures: &[String],
        test_path: &std::path::Path,
        existing_values: &HashMap<String, Value>,
    ) -> Result<String> {
        // Generate cache key for this code
        let cache_key = format!("{:?}-{:?}", fixtures, test_path);

        // Check code cache first
        if let Some(cached_code) = self.code_cache.get(&cache_key) {
            trace!("Code cache hit for fixtures: {:?}", fixtures);
            return Ok(cached_code.clone());
        }

        let test_dir = test_path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        let module_name = test_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "test".to_string());

        // Use string buffer for efficient concatenation
        let mut code = String::with_capacity(2048 + fixtures.len() * 512);

        write!(
            &mut code,
            r#"
import sys
import json
import traceback

# Add test directory to path
sys.path.insert(0, r'{}')

# Import the test module
try:
    import {} as test_module
except ImportError as e:
    print(json.dumps({{"error": f"Failed to import module: {{e}}"}}))
    sys.exit(1)

# Fixture results
fixture_results = {{}}

# Existing fixture values
existing_fixtures = {}

"#,
            test_dir,
            module_name,
            serde_json::to_string(existing_values)?
        )?;

        // Add code to execute each fixture
        for fixture_name in fixtures {
            write!(
                &mut code,
                r#"
# Execute fixture: {}
try:
    if hasattr(test_module, '{}'):
        fixture_func = getattr(test_module, '{}')
        # Get fixture dependencies from function signature
        import inspect
        sig = inspect.signature(fixture_func)
        kwargs = {{}}
        for param_name in sig.parameters:
            if param_name in existing_fixtures:
                kwargs[param_name] = existing_fixtures[param_name]
            elif param_name in fixture_results:
                kwargs[param_name] = fixture_results[param_name]
        
        # Call fixture
        result = fixture_func(**kwargs)
        fixture_results['{}'] = result
        
        # Handle generator fixtures (yield)
        if inspect.isgeneratorfunction(fixture_func):
            # For generator fixtures, we only get the yielded value
            try:
                fixture_results['{}'] = next(result)
            except StopIteration as e:
                if hasattr(e, 'value'):
                    fixture_results['{}'] = e.value
except Exception as e:
    print(json.dumps({{"error": f"Failed to execute fixture {}: {{e}}".format('{}', e)}}))
    traceback.print_exc()
    sys.exit(1)
"#,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name,
                fixture_name
            )?;
        }

        // Output results as JSON
        write!(
            &mut code,
            "\n# Output fixture results as JSON\nprint(json.dumps(fixture_results, default=str))"
        )?;

        // Cache the generated code
        self.code_cache.insert(cache_key, code.clone());

        Ok(code)
    }

    /// Comprehensive fixture cleanup with advanced teardown logic (enhanced with class management)
    pub fn cleanup_fixtures(&self, scope: FixtureScope, scope_id: &str) -> Result<()> {
        let cleanup_start = std::time::Instant::now();
        
        // Handle class-specific cleanup first
        if scope == FixtureScope::Class {
            Python::with_gil(|py| {
                if let Err(e) = self.class_manager.cleanup_class_instances(py, scope_id) {
                    eprintln!("Warning: Class instance cleanup failed for {}: {}", scope_id, e);
                }
            });
        }
        
        // Find fixtures to cleanup with proper ordering
        let mut keys_to_remove = Vec::new();
        
        // Check enhanced cache first
        self.enhanced_cache.iter().for_each(|entry| {
            let key = entry.key();
            if key.scope == scope && (scope == FixtureScope::Session || key.scope_id == scope_id) {
                keys_to_remove.push(key.clone());
            }
        });
        
        // Also check regular cache
        self.cache.iter().for_each(|entry| {
            let key = entry.key();
            if key.scope == scope && (scope == FixtureScope::Session || key.scope_id == scope_id) {
                if !keys_to_remove.contains(key) {
                    keys_to_remove.push(key.clone());
                }
            }
        });

        if keys_to_remove.is_empty() {
            debug!("No fixtures to cleanup for scope {:?} with id '{}'", scope, scope_id);
            return Ok(());
        }
        
        debug!("Cleaning up {} fixtures for scope {:?} with id '{}' (class-aware)", 
               keys_to_remove.len(), scope, scope_id);

        // Execute PyO3 generator teardowns first (highest priority)
        self.execute_pyo3_generator_teardowns(scope_id)?;
        
        // Execute regular fixture teardowns
        self.execute_regular_fixture_teardowns(scope_id, &keys_to_remove)?;
        
        // Remove fixtures from both caches with performance tracking
        let mut removed_count = 0;
        for key in &keys_to_remove {
            if self.cache.remove(key).is_some() {
                removed_count += 1;
            }
            if self.enhanced_cache.remove(key).is_some() {
                removed_count += 1;
            }
        }

        // Clean up teardown stacks
        self.cleanup_teardown_stacks(&scope, scope_id);
        
        let cleanup_duration = cleanup_start.elapsed();
        debug!(
            "Enhanced cleanup completed: removed {} fixtures in {:?} for scope {:?}", 
            removed_count, cleanup_duration, scope
        );

        Ok(())
    }
    
    /// Execute PyO3 generator teardowns with proper error handling
    fn execute_pyo3_generator_teardowns(&self, scope_id: &str) -> Result<()> {
        if let Some(generator_teardowns) = self.generator_teardown_stack.get(scope_id) {
            let teardown_items: Vec<_> = generator_teardowns.value().clone();
            
            debug!("Executing {} PyO3 generator teardowns for scope '{}'", teardown_items.len(), scope_id);
            
            // Execute generator teardowns in reverse order (LIFO)
            for (cache_key, generator) in teardown_items.into_iter().rev() {
                match self.execute_generator_teardown(&cache_key, &generator) {
                    Ok(_) => {
                        trace!("Successfully executed generator teardown for fixture '{}'", cache_key.name);
                    },
                    Err(e) => {
                        // Log error but continue with other teardowns
                        eprintln!("Warning: Generator teardown failed for fixture '{}': {}", cache_key.name, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute a single generator teardown using PyO3
    fn execute_generator_teardown(&self, cache_key: &FixtureCacheKey, generator: &PyObject) -> Result<()> {
        Python::with_gil(|py| -> Result<()> {
            let gen_obj = generator.as_ref(py);
            
            // Try to continue the generator to execute teardown code
            match gen_obj.call_method0("__next__") {
                Ok(_) => {
                    // Generator yielded another value unexpectedly
                    eprintln!("Warning: Generator fixture '{}' yielded unexpected value during teardown", cache_key.name);
                },
                Err(py_err) => {
                    // Check if it's a StopIteration (expected) or a real error
                    if py_err.is_instance_of::<pyo3::exceptions::PyStopIteration>(py) {
                        trace!("Generator teardown completed successfully for fixture '{}'", cache_key.name);
                    } else {
                        return Err(anyhow!("Generator teardown error for fixture '{}': {}", cache_key.name, py_err));
                    }
                }
            }
            
            Ok(())
        })
    }
    
    /// Execute regular fixture teardowns (non-generator)
    fn execute_regular_fixture_teardowns(&self, scope_id: &str, keys_to_remove: &[FixtureCacheKey]) -> Result<()> {
        if let Some(teardown_list) = self.teardown_stack.get(scope_id) {
            let teardown_items: Vec<_> = teardown_list
                .iter()
                .filter(|(key, _)| keys_to_remove.contains(key))
                .cloned()
                .collect();

            debug!("Executing {} regular fixture teardowns for scope '{}'", teardown_items.len(), scope_id);

            // Execute teardowns in reverse order (LIFO) for proper dependency cleanup
            for (cache_key, teardown_code) in teardown_items.into_iter().rev() {
                match self.execute_teardown_code(&cache_key, &teardown_code) {
                    Ok(_) => {
                        trace!("Successfully executed teardown for fixture '{}'", cache_key.name);
                    },
                    Err(e) => {
                        // Log error but continue with other teardowns
                        eprintln!("Warning: Teardown failed for fixture '{}': {}", cache_key.name, e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute teardown code using the most appropriate method
    fn execute_teardown_code(&self, cache_key: &FixtureCacheKey, teardown_code: &str) -> Result<()> {
        if teardown_code.trim().is_empty() {
            return Ok(());
        }
        
        // Try PyO3 execution first for better performance
        if self.can_use_pyo3_for_teardown(teardown_code) {
            self.execute_teardown_pyo3(cache_key, teardown_code)
        } else {
            // Fall back to subprocess execution
            self.execute_teardown_subprocess(cache_key, teardown_code)
        }
    }
    
    /// Check if teardown code can be executed using PyO3
    fn can_use_pyo3_for_teardown(&self, teardown_code: &str) -> bool {
        // Simple heuristics for PyO3 compatibility
        !teardown_code.contains("subprocess") &&
        !teardown_code.contains("os.system") &&
        !teardown_code.contains("exec") &&
        teardown_code.len() < 1000 // Avoid very complex teardowns
    }
    
    /// Execute teardown code using PyO3
    fn execute_teardown_pyo3(&self, cache_key: &FixtureCacheKey, teardown_code: &str) -> Result<()> {
        Python::with_gil(|py| -> Result<()> {
            // Get or create a teardown module
            let teardown_module_code = format!(
                r#"
import sys
import traceback

def execute_teardown():
    try:
{}
    except Exception as e:
        print(f"Teardown error for fixture '{}': {{e}}", file=sys.stderr)
        traceback.print_exc(file=sys.stderr)
        raise

# Execute teardown
execute_teardown()
"#,
                teardown_code.lines()
                    .map(|line| format!("        {}", line))
                    .collect::<Vec<_>>()
                    .join("\n"),
                cache_key.name
            );
            
            match PyModule::from_code(py, &teardown_module_code, "teardown_module", "teardown_module") {
                Ok(_) => {
                    trace!("PyO3 teardown executed successfully for fixture '{}'", cache_key.name);
                    Ok(())
                },
                Err(e) => {
                    Err(anyhow!("PyO3 teardown execution failed for fixture '{}': {}", cache_key.name, e))
                }
            }
        })
    }
    
    /// Execute teardown code using subprocess
    fn execute_teardown_subprocess(&self, cache_key: &FixtureCacheKey, teardown_code: &str) -> Result<()> {
        let teardown_script = format!(
            r#"
import sys
import traceback

try:
{}
except Exception as e:
    print(f"Teardown error for fixture '{}': {{e}}", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)
"#,
            teardown_code.lines()
                .map(|line| format!("    {}", line))
                .collect::<Vec<_>>()
                .join("\n"),
            cache_key.name
        );
        
        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(&teardown_script)
            .output()
            .map_err(|e| anyhow!("Failed to execute teardown subprocess for fixture '{}': {}", cache_key.name, e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Teardown subprocess failed for fixture '{}': {}", cache_key.name, stderr));
        }
        
        trace!("Subprocess teardown executed successfully for fixture '{}'", cache_key.name);
        Ok(())
    }
    
    /// Clean up teardown stacks after fixture cleanup
    fn cleanup_teardown_stacks(&self, scope: &FixtureScope, scope_id: &str) {
        // Clean up regular teardown stack
        if *scope == FixtureScope::Session || self.teardown_stack.get(scope_id).map_or(false, |list| list.is_empty()) {
            self.teardown_stack.remove(scope_id);
        }
        
        // Clean up generator teardown stack
        if *scope == FixtureScope::Session || self.generator_teardown_stack.get(scope_id).map_or(false, |list| list.is_empty()) {
            self.generator_teardown_stack.remove(scope_id);
        }
    }

    /// Get all autouse fixtures applicable to a test
    pub fn get_autouse_fixtures(&self, test: &TestItem) -> Vec<String> {
        let autouse_fixtures: Vec<String> = self
            .dependency_resolver
            .fixture_registry
            .values()
            .filter(|f| f.autouse)
            .filter(|f| self.is_fixture_applicable_to_test(f, test))
            .map(|f| f.name.clone())
            .collect();

        if !autouse_fixtures.is_empty() {
            trace!(
                "Found {} autouse fixtures for test {}",
                autouse_fixtures.len(),
                test.name
            );
        }

        autouse_fixtures
    }

    /// Check if a fixture is applicable to a test based on scope and location
    fn is_fixture_applicable_to_test(&self, fixture: &Fixture, test: &TestItem) -> bool {
        match fixture.scope {
            FixtureScope::Session => true,
            FixtureScope::Module => {
                let test_module = extract_module_from_test_id(&test.id);
                let fixture_module = fixture
                    .func_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                test_module == fixture_module
            }
            FixtureScope::Class => {
                let test_class = extract_class_from_test_id(&test.id);
                !test_class.is_empty()
            }
            FixtureScope::Function => true,
        }
    }

    /// Get comprehensive statistics about cached fixtures
    pub fn get_cache_stats(&self) -> FixtureCacheStats {
        let mut stats_by_scope = HashMap::new();
        let mut total_cached = 0;
        let mut total_memory_usage = 0;
        let mut cache_hit_rate = 0.0;

        self.cache.iter().for_each(|entry| {
            let key = entry.key();
            let value = entry.value();
            *stats_by_scope.entry(key.scope.clone()).or_insert(0) += 1;
            total_cached += 1;
            
            // Estimate memory usage
            total_memory_usage += std::mem::size_of_val(entry.key()) + std::mem::size_of_val(entry.value())
                + key.name.len() 
                + key.scope_id.len()
                + value.name.len()
                + serde_json::to_string(&value.value).unwrap_or_default().len();
        });

        // Calculate cache hit rate from access counts
        let total_accesses: u64 = self.cache.iter()
            .map(|entry| entry.value().access_count)
            .sum();
        
        if total_cached > 0 && total_accesses > 0 {
            cache_hit_rate = (total_accesses as f64 - total_cached as f64) / total_accesses as f64;
        }

        let pending_teardowns = self
            .teardown_stack
            .iter()
            .map(|entry| entry.value().len())
            .sum();

        FixtureCacheStats {
            total_cached,
            by_scope: stats_by_scope,
            pending_teardowns,
            total_memory_usage,
            cache_hit_rate,
        }
    }
    
    /// Advanced cache warming with intelligent preloading
    pub fn warm_cache_intelligently(&self, test_suite: &[TestItem], common_fixtures: &[&str]) {
        debug!("Starting intelligent cache warming for {} tests with {} common fixtures", 
               test_suite.len(), common_fixtures.len());
        
        let warming_start = std::time::Instant::now();
        
        // Analyze fixture usage patterns across the test suite
        let fixture_usage = self.analyze_fixture_usage_patterns(test_suite);
        
        // Pre-warm high-frequency fixtures
        for (fixture_name, usage_count) in fixture_usage.iter() {
            if *usage_count >= 3 { // Warm fixtures used by 3+ tests
                if let Some(fixture) = self.dependency_resolver.fixture_registry.get(fixture_name) {
                    if let Ok(code) = self.generate_fixture_execution_code(fixture, &HashMap::new()) {
                        self.code_cache.insert(
                            format!("fixture-{}-{:?}-deps-", fixture.name, fixture.func_path),
                            code
                        );
                        trace!("Pre-generated code for high-usage fixture '{}'" , fixture_name);
                    }
                }
            }
        }
        
        // Warm common built-in fixtures
        self.warm_cache(common_fixtures);
        
        let warming_duration = warming_start.elapsed();
        debug!("Cache warming completed in {:?}", warming_duration);
    }
    
    /// Analyze fixture usage patterns to optimize caching
    fn analyze_fixture_usage_patterns(&self, test_suite: &[TestItem]) -> HashMap<String, usize> {
        let mut fixture_usage = HashMap::new();
        
        for test in test_suite {
            for fixture_dep in &test.fixture_deps {
                *fixture_usage.entry(fixture_dep.clone()).or_insert(0) += 1;
            }
            
            // Also count autouse fixtures
            let autouse_fixtures = self.get_autouse_fixtures(test);
            for fixture_name in autouse_fixtures {
                *fixture_usage.entry(fixture_name).or_insert(0) += 1;
            }
        }
        
        fixture_usage
    }
    
    /// Intelligent cache eviction based on usage patterns and memory pressure
    pub fn intelligent_cache_eviction(&self, target_size: usize, memory_pressure: bool) {
        let current_stats = self.get_cache_stats();
        
        if current_stats.total_cached <= target_size && !memory_pressure {
            return; // No eviction needed
        }
        
        debug!("Starting intelligent cache eviction: current size={}, target={}, memory_pressure={}", 
               current_stats.total_cached, target_size, memory_pressure);
        
        // Collect cache entries with usage statistics
        let mut cache_entries: Vec<(FixtureCacheKey, std::time::SystemTime, u64, FixtureScope)> = 
            self.cache.iter()
                .map(|entry| {
                    let key = entry.key().clone();
                    let value = entry.value();
                    (key, value.last_accessed, value.access_count, value.scope.clone())
                })
                .collect();
        
        // Sort by eviction priority (function scope first, then by access patterns)
        cache_entries.sort_by(|a, b| {
            // Primary: scope priority (function < class < module < session)
            let scope_priority = |scope: &FixtureScope| match scope {
                FixtureScope::Function => 0,
                FixtureScope::Class => 1,
                FixtureScope::Module => 2,
                FixtureScope::Session => 3,
            };
            
            let scope_cmp = scope_priority(&a.3).cmp(&scope_priority(&b.3));
            if scope_cmp != std::cmp::Ordering::Equal {
                return scope_cmp;
            }
            
            // Secondary: access count (lower is better for eviction)
            let access_cmp = a.2.cmp(&b.2);
            if access_cmp != std::cmp::Ordering::Equal {
                return access_cmp;
            }
            
            // Tertiary: last accessed time (older is better for eviction)
            a.1.cmp(&b.1)
        });
        
        // Calculate how many to evict
        let entries_to_evict = if memory_pressure {
            current_stats.total_cached / 3 // Evict 1/3 under memory pressure
        } else {
            current_stats.total_cached - target_size
        };
        
        // Evict entries
        let mut evicted_count = 0;
        for (cache_key, _, _, _) in cache_entries.into_iter().take(entries_to_evict) {
            if self.cache.remove(&cache_key).is_some() {
                evicted_count += 1;
                trace!("Evicted fixture '{}' from cache", cache_key.name);
            }
        }
        
        debug!("Cache eviction completed: evicted {} entries", evicted_count);
    }
    
    /// Get comprehensive performance metrics for monitoring and optimization (enhanced with class metrics)
    pub fn get_performance_metrics(&self) -> FixturePerformanceMetrics {
        let cache_stats = self.get_cache_stats();
        
        // Get performance tracker stats
        let (avg_pyo3_time, avg_subprocess_time, pyo3_success_rate, decisions_made) = 
            if let Ok(tracker) = self.performance_tracker.lock() {
                let avg_pyo3 = if !tracker.pyo3_execution_times.is_empty() {
                    tracker.pyo3_execution_times.iter().sum::<std::time::Duration>() / tracker.pyo3_execution_times.len() as u32
                } else {
                    std::time::Duration::ZERO
                };
                
                let avg_subprocess = if !tracker.subprocess_execution_times.is_empty() {
                    tracker.subprocess_execution_times.iter().sum::<std::time::Duration>() / tracker.subprocess_execution_times.len() as u32
                } else {
                    std::time::Duration::ZERO
                };
                
                (avg_pyo3, avg_subprocess, tracker.pyo3_success_rate, tracker.decisions_made)
            } else {
                (std::time::Duration::ZERO, std::time::Duration::ZERO, 0.0, 0)
            };
        
        // Get Python runtime stats
        let runtime_stats = if let Ok(runtime) = self.python_runtime.lock() {
            runtime.get_stats()
        } else {
            RuntimeStats::default()
        };
        
        // Get class management metrics
        let class_metrics = self.class_manager.get_performance_metrics();
        
        FixturePerformanceMetrics {
            cache_stats,
            avg_pyo3_execution_time: avg_pyo3_time,
            avg_subprocess_execution_time: avg_subprocess_time,
            pyo3_success_rate,
            adaptive_decisions_made: decisions_made,
            python_module_imports: runtime_stats.module_imports,
            python_cache_hits: runtime_stats.cache_hits,
            python_execution_time: runtime_stats.total_execution_time,
            python_executions: runtime_stats.pyo3_executions,
            python_failures: runtime_stats.pyo3_failures,
            // Add class metrics
            classes_instantiated: class_metrics.classes_instantiated,
            setup_class_calls: class_metrics.setup_class_calls,
            teardown_class_calls: class_metrics.teardown_class_calls,
            classmethod_fixtures_created: class_metrics.classmethod_fixtures_created,
            classmethod_cache_hits: class_metrics.classmethod_cache_hits,
            total_class_setup_time: class_metrics.total_class_setup_time,
            total_class_teardown_time: class_metrics.total_class_teardown_time,
            instantiation_failures: class_metrics.instantiation_failures,
            lifecycle_errors: class_metrics.lifecycle_errors,
        }
    }
    
    /// Optimize fixture execution strategy based on collected metrics
    pub fn optimize_execution_strategy(&self) {
        if let Ok(mut tracker) = self.performance_tracker.lock() {
            // Adjust thresholds based on historical performance
            let pyo3_avg = if !tracker.pyo3_execution_times.is_empty() {
                tracker.pyo3_execution_times.iter().sum::<std::time::Duration>() / tracker.pyo3_execution_times.len() as u32
            } else {
                return;
            };
            
            let subprocess_avg = if !tracker.subprocess_execution_times.is_empty() {
                tracker.subprocess_execution_times.iter().sum::<std::time::Duration>() / tracker.subprocess_execution_times.len() as u32
            } else {
                return;
            };
            
            // Update adaptive threshold based on performance differential
            if pyo3_avg < subprocess_avg {
                // PyO3 is faster, be more aggressive
                tracker.adaptive_threshold = pyo3_avg + (subprocess_avg - pyo3_avg) / 4;
            } else {
                // Subprocess is faster, be more conservative  
                tracker.adaptive_threshold = subprocess_avg + (pyo3_avg - subprocess_avg) / 2;
            }
            
            debug!("Updated adaptive threshold to {:?} based on performance data", tracker.adaptive_threshold);
        }
    }
    
    /// Perform comprehensive health check and optimization
    pub fn health_check_and_optimize(&self) -> FixtureHealthReport {
        let metrics = self.get_performance_metrics();
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        
        // Check cache efficiency
        if metrics.cache_stats.cache_hit_rate < 0.5 {
            issues.push("Low cache hit rate detected".to_string());
            recommendations.push("Consider warming cache more aggressively".to_string());
        }
        
        // Check memory usage
        if metrics.cache_stats.total_memory_usage > 50 * 1024 * 1024 { // 50MB
            issues.push("High memory usage detected".to_string());
            recommendations.push("Consider more aggressive cache eviction".to_string());
        }
        
        // Check PyO3 vs subprocess performance
        if metrics.avg_pyo3_execution_time > metrics.avg_subprocess_execution_time * 2 {
            issues.push("PyO3 execution significantly slower than subprocess".to_string());
            recommendations.push("Review PyO3 execution strategy".to_string());
        }
        
        // Check error rates
        if metrics.pyo3_success_rate < 0.8 {
            issues.push("High PyO3 failure rate detected".to_string());
            recommendations.push("Investigate PyO3 execution failures".to_string());
        }
        
        // Perform optimizations if needed
        if !issues.is_empty() {
            self.optimize_execution_strategy();
            
            if metrics.cache_stats.total_memory_usage > 100 * 1024 * 1024 { // 100MB
                self.intelligent_cache_eviction(1000, true);
            }
        }
        
        FixtureHealthReport {
            overall_health: if issues.is_empty() { "Healthy".to_string() } else { "Needs Attention".to_string() },
            performance_metrics: metrics,
            issues,
            recommendations,
        }
    }
}

impl Default for FixtureExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive statistics about fixture cache usage
#[derive(Debug)]
pub struct FixtureCacheStats {
    pub total_cached: usize,
    pub by_scope: HashMap<FixtureScope, usize>,
    pub pending_teardowns: usize,
    pub total_memory_usage: usize,
    pub cache_hit_rate: f64,
}

/// Comprehensive performance metrics for fixture execution (enhanced with class metrics)
#[derive(Debug)]
pub struct FixturePerformanceMetrics {
    pub cache_stats: FixtureCacheStats,
    pub avg_pyo3_execution_time: std::time::Duration,
    pub avg_subprocess_execution_time: std::time::Duration,
    pub pyo3_success_rate: f64,
    pub adaptive_decisions_made: u64,
    pub python_module_imports: u64,
    pub python_cache_hits: u64,
    pub python_execution_time: std::time::Duration,
    pub python_executions: u64,
    pub python_failures: u64,
    // Enhanced class-based metrics
    pub classes_instantiated: u64,
    pub setup_class_calls: u64,
    pub teardown_class_calls: u64,
    pub classmethod_fixtures_created: u64,
    pub classmethod_cache_hits: u64,
    pub total_class_setup_time: std::time::Duration,
    pub total_class_teardown_time: std::time::Duration,
    pub instantiation_failures: u64,
    pub lifecycle_errors: u64,
}

/// Health report for fixture execution system
#[derive(Debug)]
pub struct FixtureHealthReport {
    pub overall_health: String,
    pub performance_metrics: FixturePerformanceMetrics,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Generate optimized Python code that includes fixture injection
pub fn generate_test_code_with_fixtures(
    test: &fastest_core::TestItem,
    fixture_values: &HashMap<String, FixtureValue>,
) -> String {
    let test_dir = test
        .path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    let module_name = test
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "test".to_string());

    // Convert fixture values to a format suitable for Python
    let fixture_json_values: HashMap<String, Value> = fixture_values
        .iter()
        .map(|(k, v)| (k.clone(), v.value.clone()))
        .collect();

    if test.is_async {
        format!(
            r#"
import sys
import os
import asyncio
import traceback
import json

sys.path.insert(0, r'{}')

try:
    import {} as test_module
    {}
    
    # Fixture values
    fixture_values = {}
    
    async def run_test():
        try:
            # Prepare fixture arguments
            kwargs = {{}}
            for fixture_name in {}:
                if fixture_name in fixture_values:
                    kwargs[fixture_name] = fixture_values[fixture_name]
            
            result = await {}
            print("Test passed")
        except Exception as e:
            print(f"Test failed: {{e}}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            sys.exit(1)
    
    asyncio.run(run_test())
except Exception as e:
    print(f"Failed to import or run test: {{e}}", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)
"#,
            test_dir,
            module_name,
            if let Some(class_name) = &test.class_name {
                format!("\n    test_class = getattr(test_module, '{}')\n    test_instance = test_class()", class_name)
            } else {
                format!(
                    "\n    test_func = getattr(test_module, '{}')",
                    test.function_name
                )
            },
            serde_json::to_string(&fixture_json_values).unwrap_or_else(|_| "{}".to_string()),
            serde_json::to_string(&test.fixture_deps).unwrap_or_else(|_| "[]".to_string()),
            if test.class_name.is_some() {
                format!("test_instance.{}(**kwargs)", test.function_name)
            } else {
                "test_func(**kwargs)".to_string()
            }
        )
    } else {
        format!(
            r#"
import sys
import os
import traceback
import json

sys.path.insert(0, r'{}')

try:
    import {} as test_module
    {}
    
    # Fixture values
    fixture_values = {}
    
    # Prepare fixture arguments
    kwargs = {{}}
    for fixture_name in {}:
        if fixture_name in fixture_values:
            kwargs[fixture_name] = fixture_values[fixture_name]
    
    # Run the test
    {}
    print("Test passed")
except Exception as e:
    print(f"Test failed: {{e}}", file=sys.stderr)
    traceback.print_exc(file=sys.stderr)
    sys.exit(1)
"#,
            test_dir,
            module_name,
            if let Some(class_name) = &test.class_name {
                format!("\n    test_class = getattr(test_module, '{}')\n    test_instance = test_class()", class_name)
            } else {
                format!(
                    "\n    test_func = getattr(test_module, '{}')",
                    test.function_name
                )
            },
            serde_json::to_string(&fixture_json_values).unwrap_or_else(|_| "{}".to_string()),
            serde_json::to_string(&test.fixture_deps).unwrap_or_else(|_| "[]".to_string()),
            if test.class_name.is_some() {
                format!("test_instance.{}(**kwargs)", test.function_name)
            } else {
                "test_func(**kwargs)".to_string()
            }
        )
    }
}

// Helper functions

fn extract_module_from_test_id(test_id: &str) -> String {
    test_id.split("::").next().unwrap_or("").to_string()
}

fn extract_class_from_test_id(test_id: &str) -> String {
    let parts: Vec<&str> = test_id.split("::").collect();
    if parts.len() >= 3 {
        parts[parts.len() - 2].to_string()
    } else {
        "".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_dependency_resolution() {
        let mut resolver = DependencyResolver::new();

        // Register fixtures with dependencies
        resolver.register_fixture(Fixture {
            name: "a".to_string(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: PathBuf::from("test.py"),
            dependencies: vec!["b".to_string(), "c".to_string()],
        });

        resolver.register_fixture(Fixture {
            name: "b".to_string(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: PathBuf::from("test.py"),
            dependencies: vec!["c".to_string()],
        });

        resolver.register_fixture(Fixture {
            name: "c".to_string(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: PathBuf::from("test.py"),
            dependencies: vec![],
        });

        let resolved = resolver.resolve_dependencies(&["a".to_string()]).unwrap();

        // c should come before b, b should come before a
        assert_eq!(resolved, vec!["c", "b", "a"]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut resolver = DependencyResolver::new();

        resolver.register_fixture(Fixture {
            name: "a".to_string(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: PathBuf::from("test.py"),
            dependencies: vec!["b".to_string()],
        });

        resolver.register_fixture(Fixture {
            name: "b".to_string(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: PathBuf::from("test.py"),
            dependencies: vec!["a".to_string()],
        });

        let result = resolver.resolve_dependencies(&["a".to_string()]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circular dependency"));
    }

    #[test]
    fn test_fixture_cache_key() {
        let test = TestItem {
            id: "test_module::TestClass::test_method".to_string(),
            path: PathBuf::from("test_module.py"),
            name: "test_method".to_string(),
            function_name: "test_method".to_string(),
            line_number: 10,
            is_async: false,
            class_name: Some("TestClass".to_string()),
            decorators: vec![],
            fixture_deps: vec![],
            is_xfail: false,
        };

        let key = FixtureCacheKey::for_test("my_fixture", &test, FixtureScope::Class);

        assert_eq!(key.name, "my_fixture");
        assert_eq!(key.scope, FixtureScope::Class);
        assert_eq!(key.scope_id, "TestClass");
    }
}
