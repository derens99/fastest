use super::FixtureDefinition;
use crate::error::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct TestFunction {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub class_name: Option<String>,
    pub decorators: Vec<String>,
}

pub struct RegexParser;

impl RegexParser {
    pub fn parse_file(path: &Path) -> Result<Vec<TestFunction>> {
        parse_test_file(path)
    }

    pub fn parse_fixtures_and_tests(
        path: &Path,
    ) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>)> {
        let content = std::fs::read_to_string(path)?;
        let mut tests = Vec::new();
        let mut fixtures = Vec::new();
        let mut current_class: Option<String> = None;
        let mut class_indent = 0;
        let mut pending_decorators = Vec::new();
        let mut in_decorator = false;
        let mut current_decorator = String::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();

            // Handle multi-line decorators
            if in_decorator {
                // Continue collecting decorator content
                current_decorator.push(' ');
                current_decorator.push_str(trimmed);

                // Check if decorator ends on this line
                let open_parens = current_decorator.matches('(').count();
                let close_parens = current_decorator.matches(')').count();

                if open_parens > 0 && open_parens == close_parens {
                    // Decorator is complete
                    pending_decorators.push(current_decorator.clone());
                    current_decorator.clear();
                    in_decorator = false;
                }
                continue;
            }

            // Start of a new decorator
            if trimmed.starts_with('@') {
                // Check if it's a complete single-line decorator
                let open_parens = trimmed.matches('(').count();
                let close_parens = trimmed.matches(')').count();

                if open_parens == 0 || (open_parens > 0 && open_parens == close_parens) {
                    // Complete decorator on one line
                    pending_decorators.push(trimmed.to_string());
                } else {
                    // Multi-line decorator starts here
                    in_decorator = true;
                    current_decorator = trimmed.to_string();
                }
                continue;
            }

            // Class definition
            if trimmed.starts_with("class ") && trimmed.ends_with(':') {
                if let Some(class_name) = extract_class_name(trimmed) {
                    current_class = Some(class_name);
                    class_indent = indent;
                }
                pending_decorators.clear();
            }

            // Check if we've left the class
            if current_class.is_some() && !line.trim().is_empty() && indent <= class_indent {
                current_class = None;
            }

            // Function definition
            if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                if let Some(func_name) = extract_function_name(trimmed) {
                    let is_async = trimmed.starts_with("async ");

                    // Check if this is a fixture
                    let is_fixture = pending_decorators.iter().any(|d| {
                        d.contains("@pytest.fixture")
                            || d.contains("@fixture")
                            || d.contains("@fastest.fixture")
                    });

                    if is_fixture {
                        // Parse fixture parameters
                        let (scope, autouse) = parse_fixture_decorator(&pending_decorators);
                        fixtures.push(FixtureDefinition {
                            name: func_name,
                            line_number: line_num + 1,
                            is_async,
                            scope,
                            autouse,
                            params: Vec::new(), // TODO: Parse params
                            decorators: pending_decorators.clone(),
                        });
                    } else if func_name.starts_with("test_") {
                        // It's a test function
                        tests.push(TestFunction {
                            name: func_name,
                            line_number: line_num + 1,
                            is_async,
                            class_name: current_class.clone(),
                            decorators: pending_decorators.clone(),
                        });
                    }
                }
                pending_decorators.clear();
            }
        }

        Ok((fixtures, tests))
    }
}

pub fn parse_test_file(path: &Path) -> Result<Vec<TestFunction>> {
    let (_, tests) = RegexParser::parse_fixtures_and_tests(path)?;
    Ok(tests)
}

fn parse_fixture_decorator(decorators: &[String]) -> (String, bool) {
    let mut scope = "function".to_string();
    let mut autouse = false;

    for decorator in decorators {
        if decorator.contains("fixture") {
            // Extract scope parameter
            if let Some(scope_start) = decorator.find("scope=") {
                let scope_part = &decorator[scope_start + 6..];
                if let Some(quote_end) = scope_part.find(&['"', '\'', ',', ')'][..]) {
                    let extracted_scope = scope_part[..quote_end].trim_matches(&['"', '\''][..]);
                    scope = extracted_scope.to_string();
                }
            }

            // Check for autouse
            if decorator.contains("autouse=True") {
                autouse = true;
            }
        }
    }

    (scope, autouse)
}

fn extract_class_name(line: &str) -> Option<String> {
    let class_start = "class ".len();
    let class_part = &line[class_start..];
    if let Some(paren_pos) = class_part.find('(') {
        Some(class_part[..paren_pos].trim().to_string())
    } else {
        class_part
            .find(':')
            .map(|colon_pos| class_part[..colon_pos].trim().to_string())
    }
}

fn extract_function_name(line: &str) -> Option<String> {
    let def_pos = if line.starts_with("async ") {
        line.find("def ")? + 4
    } else {
        4 // "def ".len()
    };

    let func_part = &line[def_pos..];
    func_part
        .find('(')
        .map(|paren_pos| func_part[..paren_pos].trim().to_string())
}
