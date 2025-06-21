//! Optimized Tree-sitter Python Parser
//!
//! High-performance parsing with:
//! - Single-pass AST traversal
//! - Pre-compiled queries for common patterns
//! - Minimal allocations with string slices
//! - Efficient data structures

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
// use parking_lot::RwLock; // Not needed
// use smallvec::SmallVec; // Not needed
use std::collections::HashMap;
use std::path::Path;
// use std::sync::Arc; // Not needed
use tree_sitter::{Node, Parser as TSParser, Query};
use unicode_normalization::UnicodeNormalization;

/// Normalize Unicode test names for safe IDs while preserving display names
pub fn normalize_test_name(name: &str) -> (String, String) {
    // Normalize to NFD (canonical decomposition)
    let normalized = name.nfd().collect::<String>();
    
    // Create safe ID by replacing non-ASCII characters
    let safe_id = normalized
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_string()
            } else if c == ' ' || c == '-' || c == '.' {
                "_".to_string()
            } else {
                // Convert non-ASCII to hex representation
                format!("_u{:04x}", c as u32)
            }
        })
        .collect::<String>();
    
    // Return both safe ID and original display name
    (safe_id, name.to_string())
}

/// Pre-compiled tree-sitter queries for better performance
static PYTHON_QUERIES: Lazy<PythonQueries> = Lazy::new(|| {
    let language = tree_sitter_python::language();
    
    PythonQueries {
        test_functions: Query::new(
            &language,
            r#"
            (function_definition
              name: (identifier) @name
              parameters: (parameters)? @params
            ) @func
            "#,
        ).unwrap(),
        
        class_definitions: Query::new(
            &language,
            r#"
            (class_definition
              name: (identifier) @name
              body: (block) @body
            ) @class
            "#,
        ).unwrap(),
        
        decorators: Query::new(
            &language,
            r#"
            (decorated_definition
              (decorator) @decorator
              definition: (_) @def
            )
            "#,
        ).unwrap(),
        
        fixture_decorator: Query::new(
            &language,
            r#"
            (decorator
              (call
                function: [
                  (identifier) @fixture_name
                  (attribute
                    value: (identifier) @module
                    attribute: (identifier) @fixture_name
                  )
                ]
                (#match? @fixture_name "fixture")
              )
            )
            "#,
        ).unwrap(),
    }
});

struct PythonQueries {
    test_functions: Query,
    class_definitions: Query,
    decorators: Query,
    fixture_decorator: Query,
}

/// Optimized test function information
#[derive(Debug, Clone)]
pub struct TestFunction {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub class_name: Option<String>,
    pub decorators: Vec<String>,
    pub parameters: Vec<String>,
}

/// Setup/Teardown method information
#[derive(Debug, Clone)]
pub struct SetupTeardownMethod {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub method_type: SetupTeardownType,
    pub scope: SetupTeardownScope,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetupTeardownType {
    Setup,
    Teardown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetupTeardownScope {
    Module,
    Class,
    Method,
    Function,
}

/// Module metadata
#[derive(Debug, Clone, Default)]
pub struct ModuleMetadata {
    pub setup_module: Option<SetupTeardownMethod>,
    pub teardown_module: Option<SetupTeardownMethod>,
}

/// Class metadata
#[derive(Debug, Clone)]
pub struct ClassMetadata {
    pub name: String,
    pub setup_class: Option<SetupTeardownMethod>,
    pub teardown_class: Option<SetupTeardownMethod>,
    pub setup_method: Option<SetupTeardownMethod>,
    pub teardown_method: Option<SetupTeardownMethod>,
    pub has_setup: bool,
    pub has_teardown: bool,
}

/// Fixture definition
#[derive(Debug, Clone)]
pub struct FixtureDefinition {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub scope: String,
    pub autouse: bool,
    pub params: Vec<String>,
    pub decorators: Vec<String>,
}

/// Single-pass visitor for efficient parsing
struct SinglePassVisitor<'a> {
    content: &'a str,
    fixtures: Vec<FixtureDefinition>,
    tests: Vec<TestFunction>,
    module_metadata: ModuleMetadata,
    class_metadata: HashMap<String, ClassMetadata>,
    current_class: Option<&'a str>,
}

impl<'a> SinglePassVisitor<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            content,
            fixtures: Vec::new(),
            tests: Vec::new(),
            module_metadata: ModuleMetadata::default(),
            class_metadata: HashMap::new(),
            current_class: None,
        }
    }
    
    fn visit_node(&mut self, node: Node<'a>) {
        match node.kind() {
            "module" => self.visit_module(node),
            "class_definition" => self.visit_class(node),
            "function_definition" => self.visit_function(node, None),
            "decorated_definition" => self.visit_decorated(node),
            _ => self.visit_children(node),
        }
    }
    
    fn visit_children(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }
    }
    
    fn visit_module(&mut self, node: Node<'a>) {
        self.visit_children(node);
    }
    
    fn visit_class(&mut self, node: Node<'a>) {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(class_name) = name_node.utf8_text(self.content.as_bytes()) {
                if class_name.starts_with("Test") {
                    let old_class = self.current_class;
                    self.current_class = Some(class_name);
                    
                    // Initialize class metadata
                    let mut metadata = ClassMetadata {
                        name: class_name.to_string(),
                        setup_class: None,
                        teardown_class: None,
                        setup_method: None,
                        teardown_method: None,
                        has_setup: false,
                        has_teardown: false,
                    };
                    
                    // Visit class body
                    if let Some(body) = node.child_by_field_name("body") {
                        self.visit_class_body(body, &mut metadata);
                    }
                    
                    self.class_metadata.insert(class_name.to_string(), metadata);
                    self.current_class = old_class;
                }
            }
        }
    }
    
    fn visit_class_body(&mut self, node: Node<'a>, metadata: &mut ClassMetadata) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" => self.visit_class_method(child, metadata),
                "decorated_definition" => {
                    if let Some(def) = child.child_by_field_name("definition") {
                        if def.kind() == "function_definition" {
                            let decorators = self.extract_decorators(child);
                            self.visit_class_method_with_decorators(def, metadata, decorators);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    fn visit_class_method(&mut self, node: Node<'a>, metadata: &mut ClassMetadata) {
        self.visit_class_method_with_decorators(node, metadata, Vec::new());
    }
    
    fn visit_class_method_with_decorators(
        &mut self,
        node: Node<'a>,
        metadata: &mut ClassMetadata,
        decorators: Vec<String>,
    ) {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(method_name) = name_node.utf8_text(self.content.as_bytes()) {
                let line_number = name_node.start_position().row + 1;
                let is_async = node.child(0).map(|n| n.kind() == "async").unwrap_or(false);
                
                // Check for setup/teardown methods
                match method_name {
                    "setup_class" => {
                        metadata.setup_class = Some(SetupTeardownMethod {
                            name: method_name.to_string(),
                            line_number,
                            is_async,
                            method_type: SetupTeardownType::Setup,
                            scope: SetupTeardownScope::Class,
                        });
                    }
                    "teardown_class" => {
                        metadata.teardown_class = Some(SetupTeardownMethod {
                            name: method_name.to_string(),
                            line_number,
                            is_async,
                            method_type: SetupTeardownType::Teardown,
                            scope: SetupTeardownScope::Class,
                        });
                    }
                    "setup_method" => {
                        metadata.setup_method = Some(SetupTeardownMethod {
                            name: method_name.to_string(),
                            line_number,
                            is_async,
                            method_type: SetupTeardownType::Setup,
                            scope: SetupTeardownScope::Method,
                        });
                    }
                    "teardown_method" => {
                        metadata.teardown_method = Some(SetupTeardownMethod {
                            name: method_name.to_string(),
                            line_number,
                            is_async,
                            method_type: SetupTeardownType::Teardown,
                            scope: SetupTeardownScope::Method,
                        });
                    }
                    "setUp" => metadata.has_setup = true,
                    "tearDown" => metadata.has_teardown = true,
                    _ if method_name.starts_with("test") => {
                        // It's a test method
                        let parameters = self.extract_parameters(node);
                        self.tests.push(TestFunction {
                            name: method_name.to_string(),
                            line_number,
                            is_async,
                            class_name: self.current_class.map(String::from),
                            decorators,
                            parameters,
                        });
                    }
                    _ => {
                        // Check if it's a fixture
                        if decorators.iter().any(|d| d.contains("fixture")) {
                            let fixture = self.create_fixture_from_function(
                                method_name,
                                line_number,
                                is_async,
                                decorators,
                            );
                            self.fixtures.push(fixture);
                        }
                    }
                }
            }
        }
    }
    
    fn visit_function(&mut self, node: Node<'a>, decorators: Option<Vec<String>>) {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let Ok(func_name) = name_node.utf8_text(self.content.as_bytes()) {
                let line_number = name_node.start_position().row + 1;
                let is_async = node.child(0).map(|n| n.kind() == "async").unwrap_or(false);
                let decorators = decorators.unwrap_or_default();
                
                // Check for module-level setup/teardown
                match func_name {
                    "setup_module" => {
                        self.module_metadata.setup_module = Some(SetupTeardownMethod {
                            name: func_name.to_string(),
                            line_number,
                            is_async,
                            method_type: SetupTeardownType::Setup,
                            scope: SetupTeardownScope::Module,
                        });
                    }
                    "teardown_module" => {
                        self.module_metadata.teardown_module = Some(SetupTeardownMethod {
                            name: func_name.to_string(),
                            line_number,
                            is_async,
                            method_type: SetupTeardownType::Teardown,
                            scope: SetupTeardownScope::Module,
                        });
                    }
                    _ if func_name.starts_with("test") => {
                        // It's a test function
                        let parameters = self.extract_parameters(node);
                        self.tests.push(TestFunction {
                            name: func_name.to_string(),
                            line_number,
                            is_async,
                            class_name: None,
                            decorators,
                            parameters,
                        });
                    }
                    _ => {
                        // Check if it's a fixture
                        if decorators.iter().any(|d| d.contains("fixture")) {
                            let fixture = self.create_fixture_from_function(
                                func_name,
                                line_number,
                                is_async,
                                decorators,
                            );
                            self.fixtures.push(fixture);
                        }
                    }
                }
            }
        }
    }
    
    fn visit_decorated(&mut self, node: Node<'a>) {
        let decorators = self.extract_decorators(node);
        if let Some(def) = node.child_by_field_name("definition") {
            if def.kind() == "function_definition" {
                self.visit_function(def, Some(decorators));
            }
        }
    }
    
    fn extract_decorators(&self, node: Node<'a>) -> Vec<String> {
        let mut decorators = Vec::new();
        let mut cursor = node.walk();
        
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                if let Ok(text) = child.utf8_text(self.content.as_bytes()) {
                    decorators.push(text.trim_start_matches('@').to_string());
                }
            }
        }
        
        decorators
    }
    
    fn extract_parameters(&self, node: Node<'a>) -> Vec<String> {
        let mut params = Vec::new();
        
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            
            for child in params_node.children(&mut cursor) {
                let param_name = match child.kind() {
                    "identifier" => child.utf8_text(self.content.as_bytes()).ok(),
                    "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                        child.child_by_field_name("name")
                            .and_then(|n| n.utf8_text(self.content.as_bytes()).ok())
                    }
                    _ => None,
                };
                
                if let Some(name) = param_name {
                    if name != "self" && name != "cls" {
                        params.push(name.to_string());
                    }
                }
            }
        }
        
        params
    }
    
    fn create_fixture_from_function(
        &self,
        name: &str,
        line_number: usize,
        is_async: bool,
        decorators: Vec<String>,
    ) -> FixtureDefinition {
        let mut scope = "function".to_string();
        let mut autouse = false;
        let mut params = Vec::new();
        
        // Parse fixture decorator parameters
        for decorator in &decorators {
            if decorator.contains("fixture") {
                // Simple extraction - can be enhanced
                if decorator.contains("scope=") {
                    if let Some(s) = extract_string_value(decorator, "scope") {
                        scope = s;
                    }
                }
                if decorator.contains("autouse=True") {
                    autouse = true;
                }
            }
        }
        
        FixtureDefinition {
            name: name.to_string(),
            line_number,
            is_async,
            scope,
            autouse,
            params,
            decorators,
        }
    }
    
    fn into_result(self) -> ParseResult {
        ParseResult {
            fixtures: self.fixtures,
            tests: self.tests,
            module_metadata: self.module_metadata,
            class_metadata: self.class_metadata,
        }
    }
}

/// Parse result containing all extracted information
struct ParseResult {
    fixtures: Vec<FixtureDefinition>,
    tests: Vec<TestFunction>,
    module_metadata: ModuleMetadata,
    class_metadata: HashMap<String, ClassMetadata>,
}

/// Extract string value from decorator argument
fn extract_string_value(decorator: &str, key: &str) -> Option<String> {
    let pattern = format!("{}=", key);
    if let Some(start) = decorator.find(&pattern) {
        let value_start = start + pattern.len();
        let value_part = &decorator[value_start..];
        
        // Handle quoted strings
        if let Some(quote) = value_part.chars().next() {
            if quote == '"' || quote == '\'' {
                if let Some(end) = value_part[1..].find(quote) {
                    return Some(value_part[1..end].to_string());
                }
            }
        }
    }
    None
}

/// Optimized parser using tree-sitter
pub struct Parser {
    parser: TSParser,
}

impl Parser {
    /// Create a new parser instance
    pub fn new() -> Result<Self> {
        let mut parser = TSParser::new();
        let language = tree_sitter_python::language();
        parser.set_language(&language)
            .map_err(|_| anyhow!("Failed to set Python language"))?;
        
        Ok(Self { parser })
    }
    
    /// Parse a file and extract tests and fixtures
    pub fn parse_fixtures_and_tests(
        path: &Path,
    ) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>)> {
        let content = std::fs::read_to_string(path)?;
        let mut parser = Self::new()?;
        let (fixtures, tests, _, _) = parser.parse_content(&content)?;
        Ok((fixtures, tests))
    }
    
    /// Parse content with single-pass traversal
    pub fn parse_content(
        &mut self,
        content: &str,
    ) -> Result<(
        Vec<FixtureDefinition>,
        Vec<TestFunction>,
        ModuleMetadata,
        HashMap<String, ClassMetadata>,
    )> {
        let tree = self.parser.parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse Python content"))?;
        
        let mut visitor = SinglePassVisitor::new(content);
        visitor.visit_node(tree.root_node());
        
        let result = visitor.into_result();
        Ok((result.fixtures, result.tests, result.module_metadata, result.class_metadata))
    }
}