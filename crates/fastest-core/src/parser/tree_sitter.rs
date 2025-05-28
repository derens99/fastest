use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser as TSParser};

/// Test function information
#[derive(Debug, Clone)]
pub struct TestFunction {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub class_name: Option<String>,
    pub decorators: Vec<String>,
    pub parameters: Vec<String>,
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

/// Main parser using tree-sitter for fast and accurate Python parsing
pub struct Parser {
    parser: TSParser,
}

impl Parser {
    /// Create a new parser instance
    pub fn new() -> Result<Self> {
        let mut parser = TSParser::new();
        let language = tree_sitter_python::language();
        parser
            .set_language(&language)
            .context("Failed to set Python language")?;

        Ok(Self {
            parser,
        })
    }

    /// Parse a file and extract tests and fixtures
    pub fn parse_fixtures_and_tests(
        path: &Path,
    ) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>)> {
        let mut parser = Self::new()?;
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        parser.parse_content(&content)
    }

    /// Parse content and extract tests and fixtures
    pub fn parse_content(
        &mut self,
        content: &str,
    ) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>)> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse Python content"))?;

        let root = tree.root_node();
        let mut fixtures = Vec::new();
        let mut tests = Vec::new();

        // First pass: collect all functions with their metadata
        let functions = self.collect_all_functions(root, content)?;

        // Second pass: categorize into tests and fixtures
        for func in functions {
            if self.is_fixture(&func) {
                fixtures.push(self.convert_to_fixture(func)?);
            } else if self.is_test(&func) {
                tests.push(func.into());
            }
        }

        Ok((fixtures, tests))
    }

    fn collect_all_functions(&self, root: Node, content: &str) -> Result<Vec<FunctionInfo>> {
        let mut functions = Vec::new();
        let mut class_map = HashMap::new();

        // First collect all classes
        self.collect_classes(root, content, &mut class_map)?;

        // Then collect all functions
        self.collect_functions(root, content, &mut functions, &class_map)?;

        Ok(functions)
    }

    fn collect_classes(
        &self,
        node: Node,
        content: &str,
        class_map: &mut HashMap<String, String>,
    ) -> Result<()> {
        if node.kind() == "class_definition" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let class_name = name_node.utf8_text(content.as_bytes())?;
                
                // Find all methods in this class
                if let Some(body) = node.child_by_field_name("body") {
                    self.collect_class_methods(body, content, class_name, class_map)?;
                }
            }
        }

        // Recurse into children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_classes(child, content, class_map)?;
        }

        Ok(())
    }

    fn collect_class_methods(
        &self,
        body: Node,
        content: &str,
        class_name: &str,
        class_map: &mut HashMap<String, String>,
    ) -> Result<()> {
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let method_name = name_node.utf8_text(content.as_bytes())?;
                        class_map.insert(method_name.to_string(), class_name.to_string());
                    }
                }
                "decorated_definition" => {
                    if let Some(def) = child.child_by_field_name("definition") {
                        if def.kind() == "function_definition" {
                            if let Some(name_node) = def.child_by_field_name("name") {
                                let method_name = name_node.utf8_text(content.as_bytes())?;
                                class_map.insert(method_name.to_string(), class_name.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn collect_functions(
        &self,
        node: Node,
        content: &str,
        functions: &mut Vec<FunctionInfo>,
        class_map: &HashMap<String, String>,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                if let Some(func_info) = self.parse_function(node, content, class_map)? {
                    functions.push(func_info);
                }
            }
            "decorated_definition" => {
                if let Some(def) = node.child_by_field_name("definition") {
                    if def.kind() == "function_definition" {
                        if let Some(mut func_info) = self.parse_function(def, content, class_map)? {
                            // Parse decorators
                            func_info.decorators = self.parse_decorators(node, content)?;
                            functions.push(func_info);
                        }
                    }
                }
            }
            _ => {
                // Recurse into child nodes
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.collect_functions(child, content, functions, class_map)?;
                }
            }
        }

        Ok(())
    }

    fn parse_function(
        &self,
        node: Node,
        content: &str,
        class_map: &HashMap<String, String>,
    ) -> Result<Option<FunctionInfo>> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| anyhow!("Function without name"))?;
        let name = name_node.utf8_text(content.as_bytes())?;

        // Check if it's async
        let is_async = node
            .child(0)
            .map(|n| n.kind() == "async")
            .unwrap_or(false);

        // Parse parameters
        let params = if let Some(params_node) = node.child_by_field_name("parameters") {
            self.parse_parameters(params_node, content)?
        } else {
            Vec::new()
        };

        // Get class name if this is a method
        let class_name = class_map.get(name).cloned();

        Ok(Some(FunctionInfo {
            name: name.to_string(),
            line_number: name_node.start_position().row + 1,
            params,
            is_async,
            class_name,
            decorators: Vec::new(),
        }))
    }

    fn parse_decorators(&self, node: Node, content: &str) -> Result<Vec<String>> {
        let mut decorators = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                let text = child.utf8_text(content.as_bytes())?;
                decorators.push(text.trim_start_matches('@').to_string());
            }
        }

        Ok(decorators)
    }

    fn parse_parameters(&self, node: Node, content: &str) -> Result<Vec<String>> {
        let mut params = Vec::new();
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            let param_name = match child.kind() {
                "identifier" => Some(child.utf8_text(content.as_bytes())?),
                "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                    child
                        .child_by_field_name("name")
                        .and_then(|n| n.utf8_text(content.as_bytes()).ok())
                }
                _ => None,
            };

            if let Some(name) = param_name {
                if name != "self" && name != "cls" {
                    params.push(name.to_string());
                }
            }
        }

        Ok(params)
    }

    fn is_test(&self, func: &FunctionInfo) -> bool {
        func.name.starts_with("test_") || func.name == "test"
    }

    fn is_fixture(&self, func: &FunctionInfo) -> bool {
        func.decorators.iter().any(|d| {
            d.contains("pytest.fixture") || d.contains("fixture") || d.ends_with(".fixture")
        })
    }

    fn convert_to_fixture(&self, func: FunctionInfo) -> Result<FixtureDefinition> {
        let mut scope = "function".to_string();
        let mut autouse = false;
        let mut params = Vec::new();

        // Parse fixture decorator parameters
        for decorator in &func.decorators {
            if decorator.contains("fixture") {
                // Extract scope
                if let Some(scope_match) = self.extract_kwarg(decorator, "scope") {
                    scope = scope_match;
                }

                // Extract autouse
                if decorator.contains("autouse=True") {
                    autouse = true;
                }

                // Extract params
                if let Some(params_str) = self.extract_kwarg(decorator, "params") {
                    params = self.parse_params_list(&params_str)?;
                }
            }
        }

        Ok(FixtureDefinition {
            name: func.name,
            line_number: func.line_number,
            is_async: func.is_async,
            scope,
            autouse,
            params,
            decorators: func.decorators,
        })
    }

    fn extract_kwarg(&self, decorator: &str, key: &str) -> Option<String> {
        let pattern = format!("{}=", key);
        if let Some(start) = decorator.find(&pattern) {
            let value_start = start + pattern.len();
            let value_part = &decorator[value_start..];
            
            // Handle quoted strings
            if let Some(quote_char) = value_part.chars().next() {
                if quote_char == '"' || quote_char == '\'' {
                    if let Some(end) = value_part[1..].find(quote_char) {
                        return Some(value_part[1..=end].to_string());
                    }
                }
            }
            
            // Handle unquoted values
            if let Some(end) = value_part.find(&[',', ')'][..]) {
                return Some(value_part[..end].trim().to_string());
            }
        }
        None
    }

    fn parse_params_list(&self, params_str: &str) -> Result<Vec<String>> {
        // Remove brackets and split by comma
        let cleaned = params_str.trim_start_matches('[').trim_end_matches(']');
        Ok(cleaned
            .split(',')
            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

// Internal function info structure
#[derive(Debug)]
struct FunctionInfo {
    name: String,
    line_number: usize,
    params: Vec<String>,
    is_async: bool,
    class_name: Option<String>,
    decorators: Vec<String>,
}

impl From<FunctionInfo> for TestFunction {
    fn from(func: FunctionInfo) -> Self {
        TestFunction {
            name: func.name,
            line_number: func.line_number,
            is_async: func.is_async,
            class_name: func.class_name,
            decorators: func.decorators,
            parameters: func.params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_test() {
        let content = r#"
def test_simple():
    assert True
"#;
        let mut parser = Parser::new().unwrap();
        let (_, tests) = parser.parse_content(content).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_simple");
        assert!(!tests[0].is_async);
    }

    #[test]
    fn test_parse_async_test() {
        let content = r#"
async def test_async():
    await something()
"#;
        let mut parser = Parser::new().unwrap();
        let (_, tests) = parser.parse_content(content).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_async");
        assert!(tests[0].is_async);
    }

    #[test]
    fn test_parse_fixture() {
        let content = r#"
@pytest.fixture(scope="module", autouse=True)
def setup_module():
    return "setup"
"#;
        let mut parser = Parser::new().unwrap();
        let (fixtures, _) = parser.parse_content(content).unwrap();
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "setup_module");
        assert_eq!(fixtures[0].scope, "module");
        assert!(fixtures[0].autouse);
    }

    #[test]
    fn test_parse_parametrized_test() {
        let content = r#"
@pytest.mark.parametrize("x,y", [(1, 2), (3, 4)])
def test_add(x, y):
    assert x + y > 0
"#;
        let mut parser = Parser::new().unwrap();
        let (_, tests) = parser.parse_content(content).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_add");
        assert_eq!(tests[0].parameters, vec!["x", "y"]);
        assert!(tests[0].decorators[0].contains("parametrize"));
    }

    #[test]
    fn test_parse_class_tests() {
        let content = r#"
class TestMyClass:
    def test_method_one(self):
        pass
    
    async def test_method_two(self):
        pass
"#;
        let mut parser = Parser::new().unwrap();
        let (_, tests) = parser.parse_content(content).unwrap();
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].class_name, Some("TestMyClass".to_string()));
        assert!(!tests[0].is_async);
        assert!(tests[1].is_async);
    }
}
