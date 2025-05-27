use super::{FixtureDefinition, TestFunction};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Node, Parser};

pub struct TreeSitterParser {
    parser: Parser,
}

impl TreeSitterParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser
            .set_language(&language)
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;
        Ok(Self { parser })
    }

    pub fn parse_fixtures_and_tests(
        path: &Path,
    ) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>)> {
        let content = std::fs::read_to_string(path)?;
        let mut parser = Self::new()?;

        let tree = parser
            .parser
            .parse(&content, None)
            .ok_or_else(|| anyhow!("Failed to parse Python file: {}", path.display()))?;

        let root = tree.root_node();

        // First pass: collect all functions and their metadata
        let mut all_functions = Vec::new();
        let mut all_fixtures = Vec::new();
        let mut class_map = HashMap::new();

        // Collect all classes first
        collect_classes(root, &content, &mut class_map)?;

        // Collect all functions (tests and fixtures)
        collect_functions(root, &content, &mut all_functions, &class_map)?;

        // Second pass: categorize into tests and fixtures
        let mut test_functions = Vec::new();

        for func in all_functions {
            if is_fixture(&func) {
                all_fixtures.push(convert_to_fixture(func)?);
            } else if is_test(&func) {
                test_functions.push(TestFunction::from(func));
            }
        }

        Ok((all_fixtures, test_functions))
    }
}

#[derive(Debug)]
struct FunctionInfo {
    name: String,
    line_number: usize,
    params: Vec<String>,
    is_async: bool,
    class_name: Option<String>,
    decorators: Vec<DecoratorInfo>,
    has_body: bool,
}

#[derive(Debug)]
struct DecoratorInfo {
    full_text: String,
    name: String,
    args: HashMap<String, String>,
    positional_args: Vec<String>,
}

fn collect_classes<'a>(
    node: Node<'a>,
    content: &str,
    class_map: &mut HashMap<String, String>,
) -> Result<()> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_definition" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let class_name = name_node.utf8_text(content.as_bytes())?;

                    // Find all methods in this class
                    if let Some(body) = child.child_by_field_name("body") {
                        collect_class_methods(body, content, class_name, class_map)?;
                    }
                }
            }
            _ => {
                // Recurse into child nodes
                collect_classes(child, content, class_map)?;
            }
        }
    }

    Ok(())
}

fn collect_class_methods(
    body: Node,
    content: &str,
    class_name: &str,
    class_map: &mut HashMap<String, String>,
) -> Result<()> {
    let mut cursor = body.walk();

    for child in body.children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let method_name = name_node.utf8_text(content.as_bytes())?;
                class_map.insert(method_name.to_string(), class_name.to_string());
            }
        } else if child.kind() == "decorated_definition" {
            if let Some(def) = child.child_by_field_name("definition") {
                if def.kind() == "function_definition" {
                    if let Some(name_node) = def.child_by_field_name("name") {
                        let method_name = name_node.utf8_text(content.as_bytes())?;
                        class_map.insert(method_name.to_string(), class_name.to_string());
                    }
                }
            }
        }
    }

    Ok(())
}

fn collect_functions(
    node: Node,
    content: &str,
    functions: &mut Vec<FunctionInfo>,
    class_map: &HashMap<String, String>,
) -> Result<()> {
    match node.kind() {
        "function_definition" => {
            if let Some(func_info) = parse_function(node, content, class_map)? {
                functions.push(func_info);
            }
        }
        "decorated_definition" => {
            if let Some(def) = node.child_by_field_name("definition") {
                if def.kind() == "function_definition" {
                    if let Some(mut func_info) = parse_function(def, content, class_map)? {
                        // Parse decorators
                        let mut cursor = node.walk();
                        for child in node.children(&mut cursor) {
                            if child.kind() == "decorator" {
                                if let Some(decorator) = parse_decorator(child, content)? {
                                    func_info.decorators.push(decorator);
                                }
                            }
                        }
                        functions.push(func_info);
                    }
                }
            }
        }
        _ => {
            // Recurse into child nodes
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_functions(child, content, functions, class_map)?;
            }
        }
    }

    Ok(())
}

fn parse_function(
    node: Node,
    content: &str,
    class_map: &HashMap<String, String>,
) -> Result<Option<FunctionInfo>> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| anyhow!("Function without name"))?;
    let name = name_node.utf8_text(content.as_bytes())?;

    // Check if it's async
    let is_async = node.child(0).map(|n| n.kind() == "async").unwrap_or(false);

    // Parse parameters
    let params = if let Some(params_node) = node.child_by_field_name("parameters") {
        parse_parameters(params_node, content)?
    } else {
        Vec::new()
    };

    // Get class name if this is a method
    let class_name = class_map.get(name).cloned();

    Ok(Some(FunctionInfo {
        name: name.to_string(),
        line_number: node.start_position().row + 1,
        params,
        is_async,
        class_name,
        decorators: Vec::new(),
        has_body: node.child_by_field_name("body").is_some(),
    }))
}

fn parse_decorator(node: Node, content: &str) -> Result<Option<DecoratorInfo>> {
    let full_text = node.utf8_text(content.as_bytes())?.trim_start_matches('@');

    // Parse decorator name and arguments
    let mut name = String::new();
    let mut args = HashMap::new();
    let mut positional_args = Vec::new();

    // Handle different decorator patterns
    let _decorator_content = if let Some(expr) = node.child(1) {
        match expr.kind() {
            "call" => {
                // @decorator(args)
                if let Some(func_node) = expr.child_by_field_name("function") {
                    name = extract_decorator_name(func_node, content)?;

                    if let Some(args_node) = expr.child_by_field_name("arguments") {
                        parse_decorator_args(args_node, content, &mut args, &mut positional_args)?;
                    }
                }
            }
            "attribute" => {
                // @module.decorator
                name = expr.utf8_text(content.as_bytes())?.to_string();
            }
            "identifier" => {
                // @decorator
                name = expr.utf8_text(content.as_bytes())?.to_string();
            }
            _ => {}
        }
        expr
    } else {
        return Ok(None);
    };

    Ok(Some(DecoratorInfo {
        full_text: full_text.to_string(),
        name,
        args,
        positional_args,
    }))
}

fn extract_decorator_name(node: Node, content: &str) -> Result<String> {
    match node.kind() {
        "identifier" => Ok(node.utf8_text(content.as_bytes())?.to_string()),
        "attribute" => Ok(node.utf8_text(content.as_bytes())?.to_string()),
        _ => Ok(String::new()),
    }
}

fn parse_decorator_args(
    node: Node,
    content: &str,
    args: &mut HashMap<String, String>,
    positional_args: &mut Vec<String>,
) -> Result<()> {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "keyword_argument" => {
                if let (Some(name_node), Some(value_node)) = (
                    child.child_by_field_name("name"),
                    child.child_by_field_name("value"),
                ) {
                    let name = name_node.utf8_text(content.as_bytes())?;
                    let value = value_node.utf8_text(content.as_bytes())?;
                    args.insert(name.to_string(), value.to_string());
                }
            }
            "string" | "integer" | "float" | "true" | "false" | "none" | "identifier" => {
                positional_args.push(child.utf8_text(content.as_bytes())?.to_string());
            }
            "list" => {
                // Handle list arguments (e.g., params=[1, 2, 3])
                let list_text = child.utf8_text(content.as_bytes())?;
                positional_args.push(list_text.to_string());
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_parameters(node: Node, content: &str) -> Result<Vec<String>> {
    let mut params = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                let param_name = child.utf8_text(content.as_bytes())?;
                if param_name != "self" {
                    params.push(param_name.to_string());
                }
            }
            "typed_parameter" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let param_name = name_node.utf8_text(content.as_bytes())?;
                    if param_name != "self" {
                        params.push(param_name.to_string());
                    }
                }
            }
            "default_parameter" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let param_name = name_node.utf8_text(content.as_bytes())?;
                    if param_name != "self" {
                        params.push(param_name.to_string());
                    }
                }
            }
            "typed_default_parameter" => {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let param_name = name_node.utf8_text(content.as_bytes())?;
                    if param_name != "self" {
                        params.push(param_name.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    Ok(params)
}

fn is_test(func: &FunctionInfo) -> bool {
    func.name.starts_with("test_") || func.name == "test"
}

fn is_fixture(func: &FunctionInfo) -> bool {
    func.decorators.iter().any(|d| {
        d.name == "pytest.fixture" || d.name == "fixture" || d.full_text.contains("pytest.fixture")
    })
}

fn convert_to_fixture(func: FunctionInfo) -> Result<FixtureDefinition> {
    let mut scope = "function".to_string();
    let mut autouse = false;
    let mut params = Vec::new();

    // Find the fixture decorator
    for decorator in &func.decorators {
        if decorator.name.contains("fixture") {
            // Extract scope
            if let Some(scope_value) = decorator.args.get("scope") {
                scope = scope_value
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string();
            }

            // Extract autouse
            if let Some(autouse_value) = decorator.args.get("autouse") {
                autouse = autouse_value == "True";
            }

            // Extract params
            if let Some(params_value) = decorator.args.get("params") {
                params = parse_params_list(params_value)?;
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
        decorators: func
            .decorators
            .iter()
            .map(|d| d.full_text.clone())
            .collect(),
    })
}

fn parse_params_list(params_str: &str) -> Result<Vec<String>> {
    // Remove brackets and split by comma
    let cleaned = params_str.trim_start_matches('[').trim_end_matches(']');
    Ok(cleaned
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

impl From<FunctionInfo> for TestFunction {
    fn from(func: FunctionInfo) -> Self {
        TestFunction {
            name: func.name.clone(),
            line_number: func.line_number,
            is_async: func.is_async,
            class_name: func.class_name,
            decorators: func
                .decorators
                .iter()
                .map(|d| d.full_text.clone())
                .collect(),
        }
    }
}
