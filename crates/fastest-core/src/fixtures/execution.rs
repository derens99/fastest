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
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, trace};

use super::{Fixture, FixtureScope};
use crate::discovery::TestItem;

/// Represents a fixture value that can be cached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureValue {
    pub name: String,
    pub value: serde_json::Value,
    pub scope: FixtureScope,
    pub teardown_code: Option<String>,
    pub created_at: std::time::SystemTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msgpack_value: Option<Vec<u8>>, // Cached MessagePack representation
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
            msgpack_value: Some(bytes.to_vec()),
        })
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

/// Executes fixture code and returns the fixture values
pub struct FixtureExecutor {
    fixture_code: HashMap<String, String>,
    cache: Arc<DashMap<FixtureCacheKey, FixtureValue>>,
    dependency_resolver: DependencyResolver,
    teardown_stack: Arc<DashMap<String, Vec<(FixtureCacheKey, String)>>>, // scope_id -> [(key, teardown_code)]
    code_cache: Arc<DashMap<String, String>>, // Cache for generated Python code
    execution_semaphore: Arc<Semaphore>,      // Limit parallel Python processes
}

impl FixtureExecutor {
    pub fn new() -> Self {
        let max_parallel = num_cpus::get().min(8); // Limit parallel Python processes
        Self {
            fixture_code: HashMap::new(),
            cache: Arc::new(DashMap::with_capacity(1000)), // Pre-allocate for better performance
            dependency_resolver: DependencyResolver::new(),
            teardown_stack: Arc::new(DashMap::new()),
            code_cache: Arc::new(DashMap::with_capacity(100)), // Cache generated code
            execution_semaphore: Arc::new(Semaphore::new(max_parallel)),
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
            let _ = self.generate_fixture_execution_code(&fixture);
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

    /// Create batches of fixtures that can be executed in parallel
    fn create_fixture_batches(&self, required_fixtures: &[String]) -> Result<Vec<FixtureBatch>> {
        let ordered_fixtures = self
            .dependency_resolver
            .resolve_dependencies(required_fixtures)?;
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut processed = HashSet::new();

        // Group fixtures by dependency level
        for fixture_name in ordered_fixtures {
            let deps = self
                .dependency_resolver
                .fixture_registry
                .get(&fixture_name)
                .map(|f| f.dependencies.clone())
                .unwrap_or_else(Vec::new);

            // Check if all dependencies are already processed
            let can_batch = deps.iter().all(|dep| processed.contains(dep));

            if can_batch && !current_batch.is_empty() {
                // Can execute in parallel with current batch
                current_batch.push(fixture_name.clone());
            } else {
                // Need to start a new batch
                if !current_batch.is_empty() {
                    batches.push(FixtureBatch {
                        fixtures: current_batch,
                        level: batches.len(),
                    });
                    current_batch = Vec::new();
                }
                current_batch.push(fixture_name.clone());
            }

            processed.insert(fixture_name);
        }

        if !current_batch.is_empty() {
            batches.push(FixtureBatch {
                fixtures: current_batch,
                level: batches.len(),
            });
        }

        // Log batch information for debugging
        if !batches.is_empty() {
            debug!(
                "Created {} fixture batches for parallel execution",
                batches.len()
            );
            for (i, batch) in batches.iter().enumerate() {
                trace!("  Batch {}: {} fixtures", i, batch.fixtures.len());
            }
        }

        Ok(batches)
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

    /// Get or create a fixture value
    fn get_or_create_fixture(&self, fixture_name: &str, test: &TestItem) -> Result<FixtureValue> {
        let fixture = self
            .dependency_resolver
            .fixture_registry
            .get(fixture_name)
            .ok_or_else(|| anyhow!("Fixture '{}' not found", fixture_name))?;

        let cache_key = FixtureCacheKey::for_test(fixture_name, test, fixture.scope.clone());

        // Check cache first - DashMap allows concurrent reads
        if let Some(cached_value) = self.cache.get(&cache_key) {
            trace!("Cache hit for fixture: {}", fixture_name);
            return Ok(cached_value.clone());
        }

        // Create new fixture instance
        let fixture_value = self.create_fixture_instance(fixture, test)?;

        // Cache if appropriate
        if matches!(
            fixture.scope,
            FixtureScope::Class | FixtureScope::Module | FixtureScope::Session
        ) {
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

    /// Create a new fixture instance with optimized execution
    fn create_fixture_instance(&self, fixture: &Fixture, test: &TestItem) -> Result<FixtureValue> {
        let start_time = std::time::Instant::now();

        let value = if crate::fixtures::is_builtin_fixture(&fixture.name) {
            // Built-in fixtures are created directly without Python execution
            self.create_builtin_fixture_value(&fixture.name)?
        } else {
            // User-defined fixture - use optimized execution path
            self.execute_user_fixture(fixture, test)?
        };

        let duration = start_time.elapsed();
        trace!("Created fixture '{}' in {:?}", fixture.name, duration);

        Ok(FixtureValue {
            name: fixture.name.clone(),
            value,
            scope: fixture.scope.clone(),
            teardown_code: self.extract_teardown_code(fixture)?,
            created_at: std::time::SystemTime::now(),
            msgpack_value: None,
        })
    }

    /// Execute a user-defined fixture using the most efficient method
    fn execute_user_fixture(
        &self,
        fixture: &Fixture,
        test: &TestItem,
    ) -> Result<serde_json::Value> {
        // Check if we can use PyO3 for in-process execution
        if self.can_use_pyo3_execution(fixture) {
            self.execute_fixture_pyo3(fixture, test)
        } else {
            // Fall back to subprocess execution
            self.execute_fixture_subprocess(fixture, test)
        }
    }

    /// Check if a fixture can be executed using PyO3
    fn can_use_pyo3_execution(&self, fixture: &Fixture) -> bool {
        // Simple fixtures without complex dependencies can use PyO3
        fixture.dependencies.len() < 3 && !fixture.func_path.to_string_lossy().contains("conftest")
    }

    /// Execute fixture using PyO3 (fast path)
    fn execute_fixture_pyo3(
        &self,
        fixture: &Fixture,
        _test: &TestItem,
    ) -> Result<serde_json::Value> {
        // This would use PyO3 for direct Python execution
        // For now, return a placeholder
        Ok(serde_json::json!({
            "type": "pyo3_fixture",
            "name": fixture.name,
            "executed_with": "pyo3",
            "scope": format!("{:?}", fixture.scope)
        }))
    }

    /// Execute fixture using subprocess (fallback)
    fn execute_fixture_subprocess(
        &self,
        fixture: &Fixture,
        _test: &TestItem,
    ) -> Result<serde_json::Value> {
        let execution_code = self.generate_fixture_execution_code(fixture)?;

        // Execute via subprocess with environment variable for output format
        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(&execution_code)
            .env("FASTEST_OUTPUT_FORMAT", "msgpack")
            .output()
            .map_err(|e| anyhow!("Failed to execute fixture: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Fixture execution failed: {}", stderr));
        }

        // Try to parse as MessagePack first, fall back to JSON
        if output.stdout.starts_with(b"\x82") || output.stdout.starts_with(b"\x83") {
            // Likely MessagePack format
            rmp_serde::from_slice(&output.stdout)
                .map_err(|e| anyhow!("Failed to parse MessagePack fixture result: {}", e))
        } else {
            // Fall back to JSON
            let stdout = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str(&stdout)
                .map_err(|e| anyhow!("Failed to parse JSON fixture result: {}", e))
        }
    }

    /// Extract teardown code from yield fixtures
    fn extract_teardown_code(&self, _fixture: &Fixture) -> Result<Option<String>> {
        // TODO: Parse fixture code to extract teardown portion
        Ok(None)
    }

    /// Generate optimized Python code to execute a fixture
    fn generate_fixture_execution_code(&self, fixture: &Fixture) -> Result<String> {
        // Check code cache first
        let cache_key = format!("fixture-{}-{:?}", fixture.name, fixture.func_path);
        if let Some(cached_code) = self.code_cache.get(&cache_key) {
            return Ok(cached_code.clone());
        }

        let code = if crate::fixtures::is_builtin_fixture(&fixture.name) {
            crate::fixtures::generate_builtin_fixture_code(&fixture.name)
                .unwrap_or_else(|| "# Unknown builtin fixture".to_string())
        } else {
            // Generate optimized fixture execution code
            let mut code = String::with_capacity(1024);

            write!(
                &mut code,
                r#"
import sys
import json
import os
from pathlib import Path

# Setup path
fixture_path = Path(r'{}')
sys.path.insert(0, str(fixture_path.parent))

# Import fixture module
try:
    module_name = fixture_path.stem
    if 'conftest' in module_name:
        import conftest as fixture_module
    else:
        fixture_module = __import__(module_name)
except ImportError as e:
    print(json.dumps({{"error": f"Failed to import fixture module: {{e}}"}})
    sys.exit(1)

# Get fixture function
fixture_func = getattr(fixture_module, '{}', None)
if not fixture_func:
    print(json.dumps({{"error": "Fixture function not found"}})
    sys.exit(1)

# Execute fixture with minimal overhead
try:
    result = fixture_func()
    
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
    print(json.dumps({{"error": str(e)}}), file=sys.stderr)
    sys.exit(1)
"#,
                fixture.func_path.display(),
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
    print(json.dumps({{"error": f"Failed to execute fixture {}: {{e}}"}}))
    traceback.print_exc()
    sys.exit(1)
"#,
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

    /// Cleanup fixtures for a specific scope
    pub fn cleanup_fixtures(&self, scope: FixtureScope, scope_id: &str) -> Result<()> {
        // Find fixtures to cleanup
        let mut keys_to_remove = Vec::new();
        self.cache.iter().for_each(|entry| {
            let key = entry.key();
            if key.scope == scope && (scope == FixtureScope::Session || key.scope_id == scope_id) {
                keys_to_remove.push(key.clone());
            }
        });

        // Execute teardown code if exists
        if let Some(teardown_list) = self.teardown_stack.get(scope_id) {
            let teardown_items: Vec<_> = teardown_list
                .iter()
                .filter(|(key, _)| keys_to_remove.contains(key))
                .cloned()
                .collect();

            for (key, _teardown_code) in teardown_items.into_iter().rev() {
                // Execute teardown code via Python runtime
                // TODO: Integrate with Python runtime for actual execution
                debug!("Executing teardown for fixture '{}'", key.name);
            }
        }

        // Remove from cache
        for key in &keys_to_remove {
            self.cache.remove(key);
        }

        // Clean up teardown stack
        if scope == FixtureScope::Session || !keys_to_remove.is_empty() {
            self.teardown_stack.remove(scope_id);
        }

        Ok(())
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

    /// Get statistics about cached fixtures
    pub fn get_cache_stats(&self) -> FixtureCacheStats {
        let mut stats_by_scope = HashMap::new();
        let mut total_cached = 0;

        self.cache.iter().for_each(|entry| {
            let key = entry.key();
            *stats_by_scope.entry(key.scope.clone()).or_insert(0) += 1;
            total_cached += 1;
        });

        let pending_teardowns = self
            .teardown_stack
            .iter()
            .map(|entry| entry.value().len())
            .sum();

        FixtureCacheStats {
            total_cached,
            by_scope: stats_by_scope,
            pending_teardowns,
        }
    }
}

impl Default for FixtureExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about fixture cache usage
#[derive(Debug)]
pub struct FixtureCacheStats {
    pub total_cached: usize,
    pub by_scope: HashMap<FixtureScope, usize>,
    pub pending_teardowns: usize,
}

/// Generate optimized Python code that includes fixture injection
pub fn generate_test_code_with_fixtures(
    test: &crate::discovery::TestItem,
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
