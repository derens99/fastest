pub mod builtin;
pub mod execution;

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub use builtin::{
    generate_builtin_fixture_code, get_builtin_fixture_metadata, is_builtin_fixture,
};
pub use execution::{generate_test_code_with_fixtures, FixtureExecutor};

/// Represents a test fixture instance
#[derive(Debug, Clone)]
pub struct Fixture {
    pub name: String,
    pub scope: FixtureScope,
    pub autouse: bool,
    pub params: Vec<serde_json::Value>,
    pub func_path: PathBuf,        // Path to the module containing the fixture
    pub dependencies: Vec<String>, // Other fixtures this depends on
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FixtureScope {
    Function, // New instance for each test
    Class,    // Shared within test class
    Module,   // Shared within module
    Session,  // Shared across entire session
}

impl From<&str> for FixtureScope {
    fn from(s: &str) -> Self {
        match s {
            "class" => FixtureScope::Class,
            "module" => FixtureScope::Module,
            "session" => FixtureScope::Session,
            _ => FixtureScope::Function,
        }
    }
}

/// Key for fixture instance caching
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FixtureKey {
    name: String,
    scope: FixtureScope,
    scope_id: String, // test_id for function, class name for class, module path for module, etc.
}

/// Manages fixture instances and dependencies
pub struct FixtureManager {
    fixtures: HashMap<String, Fixture>,
    instances: Arc<Mutex<HashMap<FixtureKey, serde_json::Value>>>,
    fixture_functions: HashMap<String, String>, // name -> Python code to execute
}

impl FixtureManager {
    pub fn new() -> Self {
        let mut manager = Self {
            fixtures: HashMap::new(),
            instances: Arc::new(Mutex::new(HashMap::new())),
            fixture_functions: HashMap::new(),
        };

        // Register built-in fixtures
        manager.register_builtin_fixtures();
        manager
    }

    /// Register a fixture definition
    pub fn register_fixture(&mut self, fixture: Fixture) {
        self.fixtures.insert(fixture.name.clone(), fixture);
    }

    /// Register fixture function code (for execution)
    pub fn register_fixture_function(&mut self, name: String, code: String) {
        self.fixture_functions.insert(name, code);
    }

    /// Get fixture value for a test
    pub fn get_fixture_value(
        &self,
        name: &str,
        test_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        let fixture = self
            .fixtures
            .get(name)
            .ok_or_else(|| anyhow!("Fixture '{}' not found", name))?;

        let key = FixtureKey {
            name: name.to_string(),
            scope: fixture.scope.clone(),
            scope_id: self.get_scope_id(test_id, &fixture.scope),
        };

        // Check if we already have an instance
        let instances = self.instances.lock().unwrap();
        if let Some(value) = instances.get(&key) {
            return Ok(Some(value.clone()));
        }
        drop(instances);

        // Create new fixture instance
        // This would involve calling Python code to execute the fixture function
        // For now, return None (to be implemented with Python integration)
        Ok(None)
    }

    /// Setup fixtures for a test
    pub fn setup_fixtures(
        &self,
        test_id: &str,
        required_fixtures: &[String],
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut fixture_values = HashMap::new();

        // Sort fixtures by dependency order
        let sorted_fixtures = self.sort_fixtures_by_dependency(required_fixtures)?;

        for fixture_name in sorted_fixtures {
            if let Some(value) = self.get_fixture_value(&fixture_name, test_id)? {
                fixture_values.insert(fixture_name, value);
            }
        }

        Ok(fixture_values)
    }

    /// Teardown fixtures after test
    pub fn teardown_fixtures(&self, test_id: &str, scope: FixtureScope) -> Result<()> {
        let mut instances = self.instances.lock().unwrap();

        // Remove fixtures matching the scope
        instances.retain(|key, _| {
            if key.scope == scope {
                let should_remove = match scope {
                    FixtureScope::Function => key.scope_id == test_id,
                    FixtureScope::Class => key.scope_id == self.get_class_from_test_id(test_id),
                    FixtureScope::Module => key.scope_id == self.get_module_from_test_id(test_id),
                    FixtureScope::Session => false, // Never auto-remove session fixtures
                };
                !should_remove
            } else {
                true
            }
        });

        Ok(())
    }

    /// Get autouse fixtures for a test
    pub fn get_autouse_fixtures(&self, test_id: &str) -> Vec<String> {
        self.fixtures
            .values()
            .filter(|f| f.autouse)
            .filter(|f| self.is_fixture_applicable(f, test_id))
            .map(|f| f.name.clone())
            .collect()
    }

    // Helper methods

    fn get_scope_id(&self, test_id: &str, scope: &FixtureScope) -> String {
        match scope {
            FixtureScope::Function => test_id.to_string(),
            FixtureScope::Class => self.get_class_from_test_id(test_id),
            FixtureScope::Module => self.get_module_from_test_id(test_id),
            FixtureScope::Session => "session".to_string(),
        }
    }

    fn get_class_from_test_id(&self, test_id: &str) -> String {
        // Extract class name from test_id (e.g., "module::Class::method" -> "Class")
        let parts: Vec<&str> = test_id.split("::").collect();
        if parts.len() >= 3 {
            parts[parts.len() - 2].to_string()
        } else {
            "".to_string()
        }
    }

    fn get_module_from_test_id(&self, test_id: &str) -> String {
        // Extract module from test_id (e.g., "module::Class::method" -> "module")
        test_id.split("::").next().unwrap_or("").to_string()
    }

    fn is_fixture_applicable(&self, fixture: &Fixture, test_id: &str) -> bool {
        // Check if fixture is applicable based on scope and location
        match fixture.scope {
            FixtureScope::Session => true,
            FixtureScope::Module => {
                let fixture_module = fixture.func_path.to_string_lossy();
                test_id.starts_with(&*fixture_module)
            }
            _ => true, // Function and class scoped fixtures are handled elsewhere
        }
    }

    fn sort_fixtures_by_dependency(&self, fixtures: &[String]) -> Result<Vec<String>> {
        // Simple topological sort for fixture dependencies
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for fixture in fixtures {
            self.visit_fixture(fixture, &mut sorted, &mut visited)?;
        }

        Ok(sorted)
    }

    fn visit_fixture(
        &self,
        name: &str,
        sorted: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if visited.contains(name) {
            return Ok(());
        }

        if let Some(fixture) = self.fixtures.get(name) {
            // Visit dependencies first
            for dep in &fixture.dependencies {
                self.visit_fixture(dep, sorted, visited)?;
            }
        }

        visited.insert(name.to_string());
        sorted.push(name.to_string());
        Ok(())
    }

    fn register_builtin_fixtures(&mut self) {
        // Register tmp_path
        self.fixtures.insert(
            "tmp_path".to_string(),
            Fixture {
                name: "tmp_path".to_string(),
                scope: FixtureScope::Function,
                autouse: false,
                params: vec![],
                func_path: PathBuf::from("__builtin__"),
                dependencies: vec![],
            },
        );

        // Register capsys
        self.fixtures.insert(
            "capsys".to_string(),
            Fixture {
                name: "capsys".to_string(),
                scope: FixtureScope::Function,
                autouse: false,
                params: vec![],
                func_path: PathBuf::from("__builtin__"),
                dependencies: vec![],
            },
        );

        // Register monkeypatch
        self.fixtures.insert(
            "monkeypatch".to_string(),
            Fixture {
                name: "monkeypatch".to_string(),
                scope: FixtureScope::Function,
                autouse: false,
                params: vec![],
                func_path: PathBuf::from("__builtin__"),
                dependencies: vec![],
            },
        );
    }
}

/// Extract fixture dependencies from test function parameters
pub fn extract_fixture_deps(
    test_function: &crate::parser::TestFunction,
    content: &str,
) -> Vec<String> {
    // Find the function definition line
    let lines: Vec<&str> = content.lines().collect();
    if test_function.line_number > 0 && test_function.line_number <= lines.len() {
        let func_line = lines[test_function.line_number - 1];

        // Extract parameters from function signature
        if let Some(start) = func_line.find('(') {
            if let Some(end) = func_line.find(')') {
                let params_str = &func_line[start + 1..end];
                let params: Vec<String> = params_str
                    .split(',')
                    .map(|p| p.trim())
                    .filter(|p| !p.is_empty() && *p != "self")
                    .map(|p| {
                        // Handle type annotations (e.g., "param: Type" -> "param")
                        p.split(':').next().unwrap_or(p).trim().to_string()
                    })
                    .collect();
                return params;
            }
        }
    }

    Vec::new()
}
