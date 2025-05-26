use super::TestFunction;
use anyhow::{anyhow, Result};
use tree_sitter::{Node, Parser};

pub struct AstParser {
    parser: Parser,
}

impl AstParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser
            .set_language(&language)
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;
        Ok(Self { parser })
    }

    pub fn parse_file(&mut self, content: &str, file_path: &str) -> Result<Vec<TestFunction>> {
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse Python file: {}", file_path))?;

        let root = tree.root_node();
        let mut tests = Vec::new();

        // Use visitor pattern for now, can optimize with queries later
        self.visit_node(root, content, &mut tests, None)?;

        Ok(tests)
    }

    fn visit_node(
        &self,
        node: Node,
        source: &str,
        tests: &mut Vec<TestFunction>,
        current_class: Option<&str>,
    ) -> Result<()> {
        match node.kind() {
            "function_definition" => {
                // Check if this function is inside a decorated_definition
                let decorators = if let Some(parent) = node.parent() {
                    if parent.kind() == "decorated_definition" {
                        self.get_decorators(parent, source)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Check if it's an async function by looking at the first child
                let is_async = node.child(0).map(|n| n.kind() == "async").unwrap_or(false);

                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = &source[name_node.byte_range()];
                    if name.starts_with("test_") {
                        let line_number = name_node.start_position().row + 1;

                        tests.push(TestFunction {
                            name: name.to_string(),
                            line_number,
                            is_async: is_async || self.has_async_decorator(&decorators),
                            class_name: current_class.map(String::from),
                            decorators,
                        });
                    }
                }
            }
            "decorated_definition" => {
                // Handle decorated functions - pass decorators down
                if let Some(definition) = node.child_by_field_name("definition") {
                    if definition.kind() == "function_definition" {
                        // Don't recurse into the function_definition here
                        // Instead, handle it directly with decorators
                        let decorators = self.get_decorators(node, source);

                        // Check if it's an async function
                        let is_async = definition
                            .child(0)
                            .map(|n| n.kind() == "async")
                            .unwrap_or(false);

                        if let Some(name_node) = definition.child_by_field_name("name") {
                            let name = &source[name_node.byte_range()];
                            if name.starts_with("test_") {
                                let line_number = name_node.start_position().row + 1;

                                tests.push(TestFunction {
                                    name: name.to_string(),
                                    line_number,
                                    is_async: is_async || self.has_async_decorator(&decorators),
                                    class_name: current_class.map(String::from),
                                    decorators,
                                });
                            }
                        }
                    }
                } else {
                    // Still visit children in case there are nested structures
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() != "decorator_list" {
                            self.visit_node(child, source, tests, current_class)?;
                        }
                    }
                }
            }
            "class_definition" => {
                // Check if class name starts with Test
                if let Some(name_node) = node.child_by_field_name("name") {
                    let class_name = &source[name_node.byte_range()];
                    if class_name.starts_with("Test") {
                        // Visit class body looking for test methods
                        if let Some(body) = node.child_by_field_name("body") {
                            self.visit_node(body, source, tests, Some(class_name))?;
                        }
                    }
                }
            }
            _ => {
                // Recursively visit children
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.visit_node(child, source, tests, current_class)?;
                }
            }
        }

        Ok(())
    }

    fn get_decorators(&self, node: Node, source: &str) -> Vec<String> {
        let mut decorators = Vec::new();

        // For a decorated_definition node, look for decorator children
        if node.kind() == "decorated_definition" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "decorator" {
                    let text = &source[child.byte_range()];
                    let cleaned = text.trim_start_matches('@').to_string();
                    decorators.push(cleaned);
                }
            }
        }

        decorators
    }

    fn has_async_decorator(&self, decorators: &[String]) -> bool {
        decorators
            .iter()
            .any(|d| d == "async" || d.starts_with("asyncio.") || d.contains("async"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print_tree(node: Node, source: &str, indent: usize) {
        let kind = node.kind();
        let text = if node.child_count() == 0 {
            &source[node.byte_range()]
        } else {
            ""
        };

        println!("{}{} {}", " ".repeat(indent), kind, text);

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            print_tree(child, source, indent + 2);
        }
    }

    #[test]
    fn debug_async_structure() {
        let content = r#"async def test_async():
    await something()"#;

        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser.set_language(&language).unwrap();

        let tree = parser.parse(content, None).unwrap();
        print_tree(tree.root_node(), content, 0);
    }

    #[test]
    fn test_parse_simple_function() {
        let content = r#"
def test_simple():
    assert True

def not_a_test():
    pass
"#;

        let mut parser = AstParser::new().unwrap();
        let tests = parser.parse_file(content, "test.py").unwrap();

        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_simple");
        assert_eq!(tests[0].line_number, 2);
        assert!(!tests[0].is_async);
    }

    #[test]
    fn test_parse_async_function() {
        let content = r#"
async def test_async():
    await something()
"#;

        let mut parser = AstParser::new().unwrap();
        let tests = parser.parse_file(content, "test.py").unwrap();

        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "test_async");
        assert!(tests[0].is_async);
    }

    #[test]
    fn test_parse_class_methods() {
        let content = r#"
class TestMyClass:
    def test_method_one(self):
        pass
    
    def test_method_two(self):
        pass
        
    def not_a_test(self):
        pass

class NotATestClass:
    def test_ignored(self):
        pass
"#;

        let mut parser = AstParser::new().unwrap();
        let tests = parser.parse_file(content, "test.py").unwrap();

        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "test_method_one");
        assert_eq!(tests[0].class_name, Some("TestMyClass".to_string()));
        assert_eq!(tests[1].name, "test_method_two");
        assert_eq!(tests[1].class_name, Some("TestMyClass".to_string()));
    }

    #[test]
    fn test_parse_decorated_function() {
        let content = r#"
@pytest.mark.skip
def test_decorated():
    pass

@pytest.mark.parametrize("x", [1, 2, 3])
def test_parametrized(x):
    assert x > 0
"#;

        let mut parser = AstParser::new().unwrap();
        let tests = parser.parse_file(content, "test.py").unwrap();

        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "test_decorated");
        assert_eq!(tests[1].name, "test_parametrized");
    }
}
