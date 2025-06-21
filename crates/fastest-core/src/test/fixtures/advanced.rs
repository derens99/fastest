//! High-Performance Fixture System
//!
//! Optimized fixture management with:
//! - Lock-free reads using RwLock
//! - Pre-computed dependency graphs
//! - Efficient caching with minimal allocations
//! - Fast topological sorting
//! - Compact data structures

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::TestItem;

/// Fixture scope with compact representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum FixtureScope {
    Session = 0,  // Highest scope
    Package = 1,
    Module = 2,
    Class = 3,
    Function = 4, // Lowest scope
}

impl From<&str> for FixtureScope {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "session" => FixtureScope::Session,
            "package" => FixtureScope::Package,
            "module" => FixtureScope::Module,
            "class" => FixtureScope::Class,
            _ => FixtureScope::Function,
        }
    }
}

impl FixtureScope {
    #[inline]
    pub fn priority(&self) -> u8 {
        *self as u8
    }
}

/// Compact fixture definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureDefinition {
    pub name: Arc<str>,
    pub scope: FixtureScope,
    pub flags: u8, // Bit 0: autouse, Bit 1: is_yield, Bit 2: is_async
    pub params: SmallVec<[serde_json::Value; 2]>, // Most fixtures have 0-2 params
    pub ids: SmallVec<[String; 2]>,
    pub dependencies: SmallVec<[Arc<str>; 4]>, // Most fixtures have <4 deps
    pub module_path: Arc<PathBuf>,
    pub line_number: u32,
}

impl FixtureDefinition {
    #[inline]
    pub fn is_autouse(&self) -> bool {
        self.flags & 0x01 != 0
    }
    
    #[inline]
    pub fn is_yield_fixture(&self) -> bool {
        self.flags & 0x02 != 0
    }
    
    #[inline]
    pub fn is_async(&self) -> bool {
        self.flags & 0x04 != 0
    }
    
    pub fn set_autouse(&mut self, value: bool) {
        if value {
            self.flags |= 0x01;
        } else {
            self.flags &= !0x01;
        }
    }
    
    pub fn set_yield_fixture(&mut self, value: bool) {
        if value {
            self.flags |= 0x02;
        } else {
            self.flags &= !0x02;
        }
    }
    
    pub fn set_async(&mut self, value: bool) {
        if value {
            self.flags |= 0x04;
        } else {
            self.flags &= !0x04;
        }
    }
}

/// Fixture instance key for efficient caching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FixtureKey {
    pub name: Arc<str>,
    pub scope_id: Arc<str>,
    pub param_index: u16, // Most fixtures have <65k params
}

/// Fixture value types for better performance
#[derive(Debug, Clone)]
pub enum FixtureValue {
    TmpPath(PathBuf),
    Capsys { stdout: String, stderr: String },
    Monkeypatch(Vec<(String, String)>), // (target, original)
    Json(serde_json::Value),
}

/// Fixture instance with metadata
#[derive(Debug, Clone)]
pub struct FixtureInstance {
    pub value: FixtureValue,
    pub teardown: Option<Arc<str>>, // Teardown code
    pub created_at: Instant,
}

/// Fixture request context
#[derive(Debug, Clone)]
pub struct FixtureRequest {
    pub node_id: Arc<str>,
    pub test_name: Arc<str>,
    pub module_path: Arc<PathBuf>,
    pub class_name: Option<Arc<str>>,
    pub param_index: u16,
    pub requested_fixtures: SmallVec<[Arc<str>; 8]>,
    pub indirect_params: FxHashMap<String, serde_json::Value>,
}

impl FixtureRequest {
    pub fn from_test_item(test: &TestItem) -> Self {
        Self {
            node_id: Arc::from(test.id.as_str()),
            test_name: Arc::from(test.function_name.as_str()),
            module_path: Arc::new(test.path.clone()),
            class_name: test.class_name.as_ref().map(|s| Arc::from(s.as_str())),
            param_index: 0,
            requested_fixtures: test.fixture_deps.iter()
                .map(|s| Arc::from(s.as_str()))
                .collect(),
            indirect_params: FxHashMap::default(),
        }
    }
    
    #[inline]
    pub fn get_scope_id(&self, scope: FixtureScope) -> Arc<str> {
        match scope {
            FixtureScope::Function => Arc::clone(&self.node_id),
            FixtureScope::Class => {
                if let Some(class) = &self.class_name {
                    Arc::from(format!("{}::{}", self.module_path.display(), class))
                } else {
                    Arc::from(self.module_path.display().to_string())
                }
            }
            FixtureScope::Module => Arc::from(self.module_path.display().to_string()),
            FixtureScope::Package => {
                // Find package root
                let package_path = self.module_path
                    .parent()
                    .and_then(|p| p.ancestors().find(|a| a.join("__init__.py").exists()))
                    .unwrap_or_else(|| self.module_path.parent().unwrap());
                Arc::from(package_path.display().to_string())
            }
            FixtureScope::Session => Arc::from("session"),
        }
    }
}

/// Pre-computed dependency graph for fixtures
#[derive(Debug, Clone)]
struct FixtureDependencyGraph {
    /// Adjacency list representation
    edges: FxHashMap<Arc<str>, SmallVec<[Arc<str>; 4]>>,
    /// Pre-computed topological order
    topo_order: Vec<Arc<str>>,
    /// Reverse dependencies for efficient lookups
    reverse_deps: FxHashMap<Arc<str>, SmallVec<[Arc<str>; 4]>>,
}

impl FixtureDependencyGraph {
    fn new() -> Self {
        Self {
            edges: FxHashMap::default(),
            topo_order: Vec::new(),
            reverse_deps: FxHashMap::default(),
        }
    }
    
    fn add_fixture(&mut self, name: Arc<str>, deps: &[Arc<str>]) {
        self.edges.insert(Arc::clone(&name), deps.iter().cloned().collect());
        
        // Update reverse dependencies
        for dep in deps {
            self.reverse_deps
                .entry(Arc::clone(dep))
                .or_default()
                .push(Arc::clone(&name));
        }
    }
    
    fn compute_order(&mut self) -> Result<()> {
        let mut in_degree: FxHashMap<Arc<str>, usize> = FxHashMap::default();
        let mut queue = Vec::new();
        
        // Initialize in-degrees
        for (node, deps) in &self.edges {
            in_degree.entry(Arc::clone(node)).or_insert(0);
            for dep in deps {
                *in_degree.entry(Arc::clone(dep)).or_insert(0) += 1;
            }
        }
        
        // Find nodes with no dependencies
        for (node, &degree) in &in_degree {
            if degree == 0 {
                queue.push(Arc::clone(node));
            }
        }
        
        let mut result = Vec::with_capacity(self.edges.len());
        
        while let Some(node) = queue.pop() {
            result.push(Arc::clone(&node));
            
            if let Some(deps) = self.reverse_deps.get(&node) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(Arc::clone(dep));
                        }
                    }
                }
            }
        }
        
        if result.len() != self.edges.len() {
            return Err(anyhow!("Circular dependency detected in fixtures"));
        }
        
        self.topo_order = result;
        Ok(())
    }
    
    fn get_execution_order(&self, required: &[Arc<str>]) -> Vec<Arc<str>> {
        let required_set: FxHashSet<_> = required.iter().cloned().collect();
        self.topo_order.iter()
            .filter(|name| required_set.contains(*name))
            .cloned()
            .collect()
    }
}

/// High-performance fixture manager
pub struct AdvancedFixtureManager {
    /// Fixture definitions
    fixtures: Arc<RwLock<FxHashMap<Arc<str>, FixtureDefinition>>>,
    /// Cached fixture instances
    instances: Arc<RwLock<FxHashMap<FixtureKey, FixtureInstance>>>,
    /// Dependency graph
    dep_graph: Arc<RwLock<FixtureDependencyGraph>>,
    /// Setup order for teardown
    setup_order: Arc<RwLock<Vec<FixtureKey>>>,
    /// Fixture implementation code
    fixture_code: Arc<RwLock<FxHashMap<Arc<str>, Arc<str>>>>,
}

impl AdvancedFixtureManager {
    pub fn new() -> Self {
        Self {
            fixtures: Arc::new(RwLock::new(FxHashMap::default())),
            instances: Arc::new(RwLock::new(FxHashMap::default())),
            dep_graph: Arc::new(RwLock::new(FixtureDependencyGraph::new())),
            setup_order: Arc::new(RwLock::new(Vec::new())),
            fixture_code: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }
    
    /// Register a fixture with validation
    pub fn register_fixture(&self, mut fixture: FixtureDefinition) -> Result<()> {
        // Intern strings for efficiency
        fixture.name = Arc::from(fixture.name.as_ref());
        fixture.dependencies = fixture.dependencies.iter()
            .map(|d| Arc::from(d.as_ref()))
            .collect();
        
        let name = Arc::clone(&fixture.name);
        let deps = fixture.dependencies.clone();
        
        // Update fixtures
        {
            let mut fixtures = self.fixtures.write();
            if let Some(existing) = fixtures.get(&name) {
                if Arc::ptr_eq(&existing.module_path, &fixture.module_path) {
                    return Err(anyhow!("Duplicate fixture '{}' in same module", name));
                }
            }
            fixtures.insert(Arc::clone(&name), fixture);
        }
        
        // Update dependency graph
        {
            let mut graph = self.dep_graph.write();
            graph.add_fixture(name, &deps);
            graph.compute_order()?;
        }
        
        Ok(())
    }
    
    /// Register fixture code
    pub fn register_fixture_code(&self, name: Arc<str>, code: Arc<str>) {
        self.fixture_code.write().insert(name, code);
    }
    
    /// Get required fixtures including autouse
    pub fn get_required_fixtures(&self, request: &FixtureRequest) -> Result<Vec<Arc<str>>> {
        let fixtures = self.fixtures.read();
        let mut required: FxHashSet<Arc<str>> = request.requested_fixtures.iter()
            .cloned()
            .collect();
        
        // Add visible autouse fixtures
        for (name, fixture) in fixtures.iter() {
            if fixture.is_autouse() && self.is_fixture_visible(fixture, request) {
                required.insert(Arc::clone(name));
            }
        }
        
        // Expand dependencies
        let mut all_fixtures = FxHashSet::default();
        let mut stack: Vec<Arc<str>> = required.into_iter().collect();
        
        while let Some(name) = stack.pop() {
            if all_fixtures.insert(Arc::clone(&name)) {
                if let Some(fixture) = fixtures.get(&name) {
                    for dep in &fixture.dependencies {
                        if !all_fixtures.contains(dep) {
                            stack.push(Arc::clone(dep));
                        }
                    }
                }
            }
        }
        
        Ok(all_fixtures.into_iter().collect())
    }
    
    /// Setup fixtures in dependency order
    pub fn setup_fixtures(
        &self,
        request: &FixtureRequest,
    ) -> Result<FxHashMap<String, FixtureValue>> {
        let required = self.get_required_fixtures(request)?;
        let graph = self.dep_graph.read();
        let ordered = graph.get_execution_order(&required);
        drop(graph);
        
        let mut values = FxHashMap::default();
        
        for fixture_name in ordered {
            let value = self.get_or_create_fixture(&fixture_name, request)?;
            values.insert(fixture_name.to_string(), value);
        }
        
        Ok(values)
    }
    
    /// Teardown fixtures for a scope
    pub fn teardown_fixtures(&self, request: &FixtureRequest, scope: FixtureScope) -> Result<()> {
        let scope_id = request.get_scope_id(scope);
        let mut instances = self.instances.write();
        let mut setup_order = self.setup_order.write();
        
        // Find fixtures to teardown in reverse order
        let to_teardown: Vec<_> = setup_order.iter()
            .rev()
            .filter(|key| {
                if let Some(fixture) = self.fixtures.read().get(&key.name) {
                    fixture.scope == scope && key.scope_id == scope_id
                } else {
                    false
                }
            })
            .cloned()
            .collect();
        
        for key in to_teardown {
            instances.remove(&key);
            setup_order.retain(|k| k != &key);
        }
        
        Ok(())
    }
    
    /// Get or create fixture instance
    fn get_or_create_fixture(
        &self,
        name: &Arc<str>,
        request: &FixtureRequest,
    ) -> Result<FixtureValue> {
        let fixture = {
            let fixtures = self.fixtures.read();
            fixtures.get(name)
                .ok_or_else(|| anyhow!("Fixture '{}' not found", name))?
                .clone()
        };
        
        let scope_id = request.get_scope_id(fixture.scope);
        let key = FixtureKey {
            name: Arc::clone(name),
            scope_id,
            param_index: request.param_index,
        };
        
        // Check cache
        {
            let instances = self.instances.read();
            if let Some(instance) = instances.get(&key) {
                return Ok(instance.value.clone());
            }
        }
        
        // Create new instance
        let value = self.create_fixture_value(name, &fixture)?;
        let instance = FixtureInstance {
            value: value.clone(),
            teardown: None,
            created_at: Instant::now(),
        };
        
        // Cache and track
        {
            let mut instances = self.instances.write();
            instances.insert(key.clone(), instance);
        }
        {
            let mut order = self.setup_order.write();
            order.push(key);
        }
        
        Ok(value)
    }
    
    /// Create fixture value based on type
    fn create_fixture_value(
        &self,
        name: &Arc<str>,
        _fixture: &FixtureDefinition,
    ) -> Result<FixtureValue> {
        // Built-in fixtures
        match name.as_ref() {
            "tmp_path" => {
                let path = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
                std::fs::create_dir_all(&path)?;
                Ok(FixtureValue::TmpPath(path))
            }
            "capsys" => {
                Ok(FixtureValue::Capsys {
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
            "monkeypatch" => {
                Ok(FixtureValue::Monkeypatch(Vec::new()))
            }
            _ => {
                // Custom fixture - return placeholder
                Ok(FixtureValue::Json(serde_json::Value::Null))
            }
        }
    }
    
    /// Check if fixture is visible to test
    fn is_fixture_visible(&self, fixture: &FixtureDefinition, request: &FixtureRequest) -> bool {
        // Simple visibility check - can be enhanced
        fixture.module_path == request.module_path ||
        fixture.scope <= FixtureScope::Module
    }
}

impl Default for AdvancedFixtureManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixture_flags() {
        let mut fixture = FixtureDefinition {
            name: Arc::from("test"),
            scope: FixtureScope::Function,
            flags: 0,
            params: SmallVec::new(),
            ids: SmallVec::new(),
            dependencies: SmallVec::new(),
            module_path: Arc::new(PathBuf::from("test.py")),
            line_number: 1,
        };
        
        fixture.set_autouse(true);
        assert!(fixture.is_autouse());
        
        fixture.set_yield_fixture(true);
        assert!(fixture.is_yield_fixture());
        assert!(fixture.is_autouse()); // Should still be true
        
        fixture.set_async(true);
        assert!(fixture.is_async());
        assert_eq!(fixture.flags, 0x07); // All flags set
    }
    
    #[test]
    fn test_dependency_graph() {
        let mut graph = FixtureDependencyGraph::new();
        
        graph.add_fixture(Arc::from("a"), &[]);
        graph.add_fixture(Arc::from("b"), &[Arc::from("a")]);
        graph.add_fixture(Arc::from("c"), &[Arc::from("a"), Arc::from("b")]);
        
        graph.compute_order().unwrap();
        
        let order = graph.get_execution_order(&[
            Arc::from("c"),
            Arc::from("b"),
            Arc::from("a"),
        ]);
        
        // Should be ordered: a, b, c
        assert_eq!(order[0].as_ref(), "a");
        assert_eq!(order[1].as_ref(), "b");
        assert_eq!(order[2].as_ref(), "c");
    }
}