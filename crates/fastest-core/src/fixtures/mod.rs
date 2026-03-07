//! Fixture management, dependency resolution, and scope-aware caching.
//!
//! This module implements the core fixture system for the Fastest test runner,
//! including:
//! - Fixture type definitions and metadata
//! - Topological dependency resolution for fixture ordering
//! - Built-in fixture recognition
//! - conftest.py discovery and fixture extraction
//! - Scope-aware fixture caching

pub mod builtin;
pub mod conftest;
pub mod scope;

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use topological_sort::TopologicalSort;

pub use builtin::{generate_builtin_code, BUILTIN_FIXTURES};
pub use conftest::discover_conftest_fixtures;
pub use scope::FixtureCache;

/// The scope of a fixture, controlling how long its value is cached.
///
/// Ordered from narrowest (per-test) to broadest (per-session).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FixtureScope {
    /// A new instance is created for each test function (default).
    #[default]
    Function,
    /// Shared across all methods in a test class.
    Class,
    /// Shared across all tests in a module (file).
    Module,
    /// Shared across all tests in a package (directory).
    Package,
    /// Shared across the entire test session.
    Session,
}

impl std::fmt::Display for FixtureScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixtureScope::Function => write!(f, "function"),
            FixtureScope::Class => write!(f, "class"),
            FixtureScope::Module => write!(f, "module"),
            FixtureScope::Package => write!(f, "package"),
            FixtureScope::Session => write!(f, "session"),
        }
    }
}

/// A fixture definition extracted from a Python source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    /// Name of the fixture (the decorated function name).
    pub name: String,
    /// Scope controlling cache lifetime.
    pub scope: FixtureScope,
    /// Whether the fixture is automatically used by all tests in scope.
    pub autouse: bool,
    /// Parametrize values for the fixture, if any.
    pub params: Vec<serde_json::Value>,
    /// Path to the Python file containing the fixture definition.
    pub func_path: PathBuf,
    /// Names of other fixtures this fixture depends on (its parameters).
    pub dependencies: Vec<String>,
    /// Whether the fixture uses `yield` (indicating teardown logic).
    pub is_yield: bool,
}

/// Check whether a fixture name refers to a built-in pytest fixture.
pub fn is_builtin(name: &str) -> bool {
    BUILTIN_FIXTURES.contains(&name)
}

/// Resolve the order in which fixtures should be set up, respecting dependencies.
///
/// Uses topological sorting to produce a linear ordering where each fixture
/// appears after all of its dependencies. Returns an error if a circular
/// dependency is detected.
///
/// Built-in fixtures are excluded from the resolved order since they are
/// provided by the framework rather than user code.
pub fn resolve_fixture_order(
    required: &[String],
    available: &HashMap<String, Fixture>,
) -> Result<Vec<String>> {
    let mut ts = TopologicalSort::<String>::new();
    let mut visited = std::collections::HashSet::new();
    let mut to_visit: Vec<String> = required.to_vec();

    // Walk the dependency graph, inserting edges into the topological sorter
    while let Some(name) = to_visit.pop() {
        if visited.contains(&name) {
            continue;
        }
        visited.insert(name.clone());

        // Built-in fixtures have no user-defined dependencies to resolve
        if is_builtin(&name) {
            ts.insert(name.clone());
            continue;
        }

        if let Some(fixture) = available.get(&name) {
            ts.insert(name.clone());
            for dep in &fixture.dependencies {
                ts.add_dependency(dep.clone(), name.clone());
                to_visit.push(dep.clone());
            }
        } else {
            // Unknown fixture -- still add it so it appears in the ordering
            ts.insert(name.clone());
        }
    }

    let mut order = Vec::with_capacity(ts.len());

    while !ts.is_empty() {
        let batch = ts.pop_all();
        if batch.is_empty() {
            // Remaining items form a cycle
            return Err(Error::Discovery(
                "Circular dependency detected among fixtures".to_string(),
            ));
        }
        order.extend(batch);
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a simple fixture with given dependencies.
    fn make_fixture(name: &str, deps: Vec<&str>) -> Fixture {
        Fixture {
            name: name.to_string(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: PathBuf::from("conftest.py"),
            dependencies: deps.into_iter().map(String::from).collect(),
            is_yield: false,
        }
    }

    #[test]
    fn test_resolve_simple_order() {
        let mut available = HashMap::new();
        available.insert("db".to_string(), make_fixture("db", vec![]));
        available.insert("user".to_string(), make_fixture("user", vec!["db"]));

        let order =
            resolve_fixture_order(&["user".to_string(), "db".to_string()], &available).unwrap();

        let db_pos = order.iter().position(|n| n == "db").unwrap();
        let user_pos = order.iter().position(|n| n == "user").unwrap();
        assert!(
            db_pos < user_pos,
            "db (pos {}) should come before user (pos {})",
            db_pos,
            user_pos
        );
    }

    #[test]
    fn test_resolve_with_chain() {
        let mut available = HashMap::new();
        available.insert("config".to_string(), make_fixture("config", vec![]));
        available.insert("db".to_string(), make_fixture("db", vec!["config"]));
        available.insert("user".to_string(), make_fixture("user", vec!["db"]));
        available.insert("admin".to_string(), make_fixture("admin", vec!["user"]));

        let order = resolve_fixture_order(&["admin".to_string()], &available).unwrap();

        let config_pos = order.iter().position(|n| n == "config").unwrap();
        let db_pos = order.iter().position(|n| n == "db").unwrap();
        let user_pos = order.iter().position(|n| n == "user").unwrap();
        let admin_pos = order.iter().position(|n| n == "admin").unwrap();

        assert!(config_pos < db_pos, "config should come before db");
        assert!(db_pos < user_pos, "db should come before user");
        assert!(user_pos < admin_pos, "user should come before admin");
    }

    #[test]
    fn test_builtin_recognition() {
        assert!(is_builtin("tmp_path"));
        assert!(is_builtin("capsys"));
        assert!(is_builtin("monkeypatch"));
        assert!(is_builtin("request"));
        assert!(is_builtin("pytestconfig"));
        assert!(is_builtin("cache"));
        assert!(is_builtin("tmp_path_factory"));
        assert!(is_builtin("capfd"));
        assert!(!is_builtin("my_custom_fixture"));
        assert!(!is_builtin("db"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut available = HashMap::new();
        available.insert("a".to_string(), make_fixture("a", vec!["b"]));
        available.insert("b".to_string(), make_fixture("b", vec!["c"]));
        available.insert("c".to_string(), make_fixture("c", vec!["a"]));

        let result = resolve_fixture_order(&["a".to_string()], &available);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Circular dependency"),
            "Expected circular dependency error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_fixture_scope_display() {
        assert_eq!(FixtureScope::Function.to_string(), "function");
        assert_eq!(FixtureScope::Class.to_string(), "class");
        assert_eq!(FixtureScope::Module.to_string(), "module");
        assert_eq!(FixtureScope::Package.to_string(), "package");
        assert_eq!(FixtureScope::Session.to_string(), "session");
    }

    #[test]
    fn test_fixture_scope_default() {
        assert_eq!(FixtureScope::default(), FixtureScope::Function);
    }

    #[test]
    fn test_resolve_with_builtins() {
        let mut available = HashMap::new();
        available.insert(
            "my_fixture".to_string(),
            make_fixture("my_fixture", vec!["tmp_path"]),
        );

        let order = resolve_fixture_order(&["my_fixture".to_string()], &available).unwrap();

        let tmp_pos = order.iter().position(|n| n == "tmp_path").unwrap();
        let my_pos = order.iter().position(|n| n == "my_fixture").unwrap();
        assert!(
            tmp_pos < my_pos,
            "tmp_path (pos {}) should come before my_fixture (pos {})",
            tmp_pos,
            my_pos
        );
    }

    #[test]
    fn test_resolve_empty() {
        let available = HashMap::new();
        let order = resolve_fixture_order(&[], &available).unwrap();
        assert!(order.is_empty());
    }
}
