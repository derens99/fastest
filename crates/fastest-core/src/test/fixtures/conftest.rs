//! Conftest.py fixture discovery and loading
//!
//! This module handles finding and parsing conftest.py files throughout
//! the project hierarchy to discover fixture definitions.

use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser, Query, QueryCursor};
use walkdir::WalkDir;

use super::advanced::{FixtureDefinition, FixtureScope};

/// Conftest file representation
#[derive(Debug, Clone)]
pub struct ConftestFile {
    pub path: PathBuf,
    pub fixtures: Vec<FixtureDefinition>,
    pub imports: Vec<String>,
    pub hooks: Vec<String>,
}

/// Conftest discovery and parsing
pub struct ConftestDiscovery {
    /// Cache of parsed conftest files
    conftest_cache: HashMap<PathBuf, ConftestFile>,
    /// Tree-sitter parser for Python
    parser: Parser,
    /// Query for finding fixtures
    fixture_query: Query,
}

impl ConftestDiscovery {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::language())?;

        // Query for finding fixture decorators and functions
        let fixture_query = Query::new(
            &tree_sitter_python::language(),
            r#"
            (decorated_definition
              decorator: (decorator
                (call
                  function: [
                    (identifier) @decorator_name
                    (attribute
                      object: (identifier) @module
                      attribute: (identifier) @decorator_name)
                  ]
                  (#match? @decorator_name "fixture")
                  arguments: (argument_list)? @args
                )
              )
              definition: (function_definition
                name: (identifier) @function_name
                parameters: (parameters) @params
                body: (block) @body
              )
            ) @fixture_def
            "#,
        )?;

        Ok(Self {
            conftest_cache: HashMap::new(),
            parser,
            fixture_query,
        })
    }

    /// Discover all conftest.py files in the project
    pub fn discover_conftest_files(&mut self, root_path: &Path) -> Result<Vec<PathBuf>> {
        let mut conftest_files = Vec::new();

        for entry in WalkDir::new(root_path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden directories and __pycache__
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "__pycache__" && name != "node_modules"
            })
        {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if path.file_name() == Some(std::ffi::OsStr::new("conftest.py")) {
                    conftest_files.push(path.to_path_buf());
                }
            }
        }

        // Sort by path depth (root conftest first)
        conftest_files.sort_by_key(|p| p.components().count());

        Ok(conftest_files)
    }

    /// Parse a conftest.py file to extract fixtures
    pub fn parse_conftest(&mut self, path: &Path) -> Result<ConftestFile> {
        // Check cache first
        if let Some(cached) = self.conftest_cache.get(path) {
            return Ok(cached.clone());
        }

        let content = std::fs::read_to_string(path)?;
        let tree = self
            .parser
            .parse(&content, None)
            .ok_or_else(|| anyhow!("Failed to parse conftest.py: {}", path.display()))?;

        let mut fixtures = Vec::new();
        let mut imports = Vec::new();
        let mut hooks = Vec::new();

        let root_node = tree.root_node();
        let mut cursor = QueryCursor::new();

        // Find all fixtures
        for match_ in cursor.matches(&self.fixture_query, root_node, content.as_bytes()) {
            if let Some(fixture) = self.parse_fixture_from_match(&match_, &content, path) {
                fixtures.push(fixture);
            }
        }

        // Find imports
        self.extract_imports(&root_node, &content, &mut imports);

        // Find pytest hooks
        self.extract_hooks(&root_node, &content, &mut hooks);

        let conftest = ConftestFile {
            path: path.to_path_buf(),
            fixtures,
            imports,
            hooks,
        };

        // Cache the result
        self.conftest_cache
            .insert(path.to_path_buf(), conftest.clone());

        Ok(conftest)
    }

    /// Parse a fixture from a query match
    fn parse_fixture_from_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        content: &str,
        file_path: &Path,
    ) -> Option<FixtureDefinition> {
        let mut fixture_name = None;
        let mut decorator_args = None;
        let mut params_node = None;
        let mut body_node = None;
        let mut line_number = 0;

        for capture in match_.captures {
            match self.fixture_query.capture_names()[capture.index as usize] {
                "function_name" => {
                    fixture_name =
                        Some(capture.node.utf8_text(content.as_bytes()).ok()?.to_string());
                    line_number = capture.node.start_position().row + 1;
                }
                "args" => {
                    decorator_args = Some(capture.node);
                }
                "params" => {
                    params_node = Some(capture.node);
                }
                "body" => {
                    body_node = Some(capture.node);
                }
                _ => {}
            }
        }

        let name = fixture_name?;

        // Parse decorator arguments
        let (scope, autouse, params, ids) = if let Some(args_node) = decorator_args {
            self.parse_fixture_decorator_args(args_node, content)
        } else {
            (FixtureScope::Function, false, Vec::new(), Vec::new())
        };

        // Extract dependencies from function parameters
        let dependencies = if let Some(params) = params_node {
            self.extract_fixture_dependencies(params, content)
        } else {
            Vec::new()
        };

        // Check if it's a yield fixture
        let is_yield_fixture = if let Some(body) = body_node {
            self.contains_yield(body, content)
        } else {
            false
        };

        // Check if async
        let is_async = params_node
            .and_then(|n| n.prev_sibling())
            .map(|n| n.kind() == "async")
            .unwrap_or(false);

        Some(FixtureDefinition {
            name,
            scope,
            autouse,
            params,
            ids,
            dependencies,
            module_path: file_path.to_path_buf(),
            line_number,
            is_yield_fixture,
            is_async,
        })
    }

    /// Parse fixture decorator arguments
    fn parse_fixture_decorator_args(
        &self,
        args_node: Node,
        content: &str,
    ) -> (FixtureScope, bool, Vec<serde_json::Value>, Vec<String>) {
        let mut scope = FixtureScope::Function;
        let mut autouse = false;
        let mut params = Vec::new();
        let mut ids = Vec::new();

        // Parse keyword arguments
        for child in args_node.children(&mut args_node.walk()) {
            if child.kind() == "keyword_argument" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let arg_name = name_node.utf8_text(content.as_bytes()).unwrap_or("");

                    if let Some(value_node) = child.child_by_field_name("value") {
                        match arg_name {
                            "scope" => {
                                if let Ok(scope_str) = value_node.utf8_text(content.as_bytes()) {
                                    let scope_str =
                                        scope_str.trim_matches(|c| c == '"' || c == '\'');
                                    scope = FixtureScope::from(scope_str);
                                }
                            }
                            "autouse" => {
                                let value_text =
                                    value_node.utf8_text(content.as_bytes()).unwrap_or("");
                                autouse = value_text == "True";
                            }
                            "params" => {
                                // Parse params list
                                if value_node.kind() == "list" {
                                    for item in value_node.children(&mut value_node.walk()) {
                                        if item.kind() != ","
                                            && item.kind() != "["
                                            && item.kind() != "]"
                                        {
                                            if let Ok(text) = item.utf8_text(content.as_bytes()) {
                                                // Try to parse as JSON value
                                                if let Ok(value) = serde_json::from_str(text) {
                                                    params.push(value);
                                                } else {
                                                    params.push(serde_json::Value::String(
                                                        text.to_string(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            "ids" => {
                                // Parse ids list
                                if value_node.kind() == "list" {
                                    for item in value_node.children(&mut value_node.walk()) {
                                        if item.kind() == "string" {
                                            if let Ok(text) = item.utf8_text(content.as_bytes()) {
                                                ids.push(
                                                    text.trim_matches(|c| c == '"' || c == '\'')
                                                        .to_string(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        (scope, autouse, params, ids)
    }

    /// Extract fixture dependencies from function parameters
    fn extract_fixture_dependencies(&self, params_node: Node, content: &str) -> Vec<String> {
        let mut dependencies = Vec::new();

        for param in params_node.children(&mut params_node.walk()) {
            match param.kind() {
                "identifier" => {
                    if let Ok(name) = param.utf8_text(content.as_bytes()) {
                        if name != "self" && name != "cls" {
                            dependencies.push(name.to_string());
                        }
                    }
                }
                "typed_parameter" => {
                    if let Some(name_node) = param.child_by_field_name("name") {
                        if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                            if name != "self" && name != "cls" {
                                dependencies.push(name.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Remove 'request' as it's a special pytest fixture
        dependencies.retain(|d| d != "request");

        dependencies
    }

    /// Check if function body contains yield
    fn contains_yield(&self, body_node: Node, _content: &str) -> bool {
        // Simple approach: walk all descendants checking for yield
        let mut has_yield = false;

        // Use a queue to avoid the lifetime issue
        let mut nodes_to_check = vec![body_node];

        while let Some(node) = nodes_to_check.pop() {
            if node.kind() == "yield" || node.kind() == "yield_statement" {
                has_yield = true;
                break;
            }

            // Add children to check
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    nodes_to_check.push(child);
                }
            }
        }

        has_yield
    }

    /// Extract imports from the module
    fn extract_imports(&self, root: &Node, content: &str, imports: &mut Vec<String>) {
        for child in root.children(&mut root.walk()) {
            match child.kind() {
                "import_statement" => {
                    if let Ok(text) = child.utf8_text(content.as_bytes()) {
                        imports.push(text.to_string());
                    }
                }
                "import_from_statement" => {
                    if let Ok(text) = child.utf8_text(content.as_bytes()) {
                        imports.push(text.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract pytest hooks from the module
    fn extract_hooks(&self, root: &Node, content: &str, hooks: &mut Vec<String>) {
        for child in root.children(&mut root.walk()) {
            if child.kind() == "function_definition" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(content.as_bytes()) {
                        // Check if it's a pytest hook
                        if name.starts_with("pytest_") {
                            hooks.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    /// Get all fixtures visible from a given test file
    pub fn get_visible_fixtures(&self, test_file: &Path) -> Vec<&FixtureDefinition> {
        let mut visible_fixtures = Vec::new();
        let mut seen_names = HashSet::new();

        // Walk up the directory tree looking for conftest files
        let mut current_dir = test_file.parent();

        while let Some(dir) = current_dir {
            let conftest_path = dir.join("conftest.py");

            if let Some(conftest) = self.conftest_cache.get(&conftest_path) {
                // Add fixtures from this conftest (closer conftests override)
                for fixture in &conftest.fixtures {
                    if !seen_names.contains(&fixture.name) {
                        visible_fixtures.push(fixture);
                        seen_names.insert(fixture.name.clone());
                    }
                }
            }

            // Move up one directory
            current_dir = dir.parent();
        }

        // Reverse to have root fixtures first (for dependency resolution)
        visible_fixtures.reverse();

        visible_fixtures
    }
}

impl Default for ConftestDiscovery {
    fn default() -> Self {
        Self::new().expect("Failed to initialize conftest discovery")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_conftest_discovery() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Create directory structure with conftest files
        let test_dir = root.join("tests");
        fs::create_dir(&test_dir)?;

        let sub_dir = test_dir.join("unit");
        fs::create_dir(&sub_dir)?;

        // Create conftest files
        fs::write(root.join("conftest.py"), "# Root conftest")?;
        fs::write(test_dir.join("conftest.py"), "# Tests conftest")?;
        fs::write(sub_dir.join("conftest.py"), "# Unit conftest")?;

        let mut discovery = ConftestDiscovery::new()?;
        let conftest_files = discovery.discover_conftest_files(root)?;

        assert_eq!(conftest_files.len(), 3);
        // Should be sorted by depth
        assert!(conftest_files[0].ends_with("conftest.py"));
        assert!(conftest_files[1].to_string_lossy().contains("tests"));
        assert!(conftest_files[2].to_string_lossy().contains("unit"));

        Ok(())
    }

    #[test]
    fn test_fixture_parsing() -> Result<()> {
        let conftest_content = r#"
import pytest

@pytest.fixture
def simple_fixture():
    return 42

@pytest.fixture(scope="module", autouse=True)
def module_fixture():
    print("Setup")
    yield
    print("Teardown")

@pytest.fixture(params=[1, 2, 3], ids=["one", "two", "three"])
def parametrized_fixture(request):
    return request.param

@pytest.fixture
async def async_fixture():
    return "async"

@pytest.fixture
def dependent_fixture(simple_fixture, module_fixture):
    return simple_fixture + 1
"#;

        let temp_dir = TempDir::new()?;
        let conftest_path = temp_dir.path().join("conftest.py");
        fs::write(&conftest_path, conftest_content)?;

        let mut discovery = ConftestDiscovery::new()?;
        let conftest = discovery.parse_conftest(&conftest_path)?;

        assert_eq!(conftest.fixtures.len(), 5);

        // Check simple fixture
        let simple = &conftest.fixtures[0];
        assert_eq!(simple.name, "simple_fixture");
        assert_eq!(simple.scope, FixtureScope::Function);
        assert!(!simple.autouse);
        assert!(!simple.is_yield_fixture);

        // Check module fixture
        let module = &conftest.fixtures[1];
        assert_eq!(module.name, "module_fixture");
        assert_eq!(module.scope, FixtureScope::Module);
        assert!(module.autouse);
        assert!(module.is_yield_fixture);

        // Check parametrized fixture
        let param = &conftest.fixtures[2];
        assert_eq!(param.name, "parametrized_fixture");
        assert_eq!(param.params.len(), 3);
        assert_eq!(param.ids.len(), 3);

        // Check async fixture
        let async_fix = &conftest.fixtures[3];
        assert_eq!(async_fix.name, "async_fixture");
        assert!(async_fix.is_async);

        // Check dependent fixture
        let dependent = &conftest.fixtures[4];
        assert_eq!(dependent.name, "dependent_fixture");
        assert_eq!(dependent.dependencies.len(), 2);
        assert!(dependent
            .dependencies
            .contains(&"simple_fixture".to_string()));
        assert!(dependent
            .dependencies
            .contains(&"module_fixture".to_string()));

        Ok(())
    }
}
