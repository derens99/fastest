//! AST-based parser for Python test files.
//!
//! Uses `rustpython-parser` to parse Python source into an AST, then extracts
//! test functions and test classes into [`TestItem`] instances.

use crate::error::{Error, Result};
use crate::model::TestItem;
use rustpython_parser::ast::{self, Expr, Ranged, Stmt};
use rustpython_parser::Parse;
use std::path::Path;

/// Parse a Python test file and return all discovered test items.
///
/// This function parses the given source code using the rustpython AST parser
/// and extracts top-level `def test_*()` functions and `class Test*` classes
/// with their nested `def test_*()` methods.
pub fn parse_test_file(source: &str, path: &Path) -> Result<Vec<TestItem>> {
    let stmts = ast::Suite::parse(source, &path.display().to_string())
        .map_err(|e| Error::Parse(format!("Failed to parse {}: {}", path.display(), e)))?;

    let line_index = LineIndex::new(source);
    let path_str = path.display().to_string();
    let mut items = Vec::new();

    for stmt in &stmts {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                let name = func_def.name.as_str();
                if is_test_function_name(name) {
                    let item = build_test_item_from_function(
                        name,
                        &func_def.args,
                        &func_def.decorator_list,
                        false, // not async
                        func_def.range(),
                        None, // no class
                        &path_str,
                        path,
                        &line_index,
                    );
                    items.push(item);
                }
            }
            Stmt::AsyncFunctionDef(func_def) => {
                let name = func_def.name.as_str();
                if is_test_function_name(name) {
                    let item = build_test_item_from_function(
                        name,
                        &func_def.args,
                        &func_def.decorator_list,
                        true, // async
                        func_def.range(),
                        None,
                        &path_str,
                        path,
                        &line_index,
                    );
                    items.push(item);
                }
            }
            Stmt::ClassDef(class_def) => {
                let class_name = class_def.name.as_str();
                if is_test_class_name(class_name) {
                    extract_class_methods(class_def, &path_str, path, &line_index, &mut items);
                }
            }
            _ => {}
        }
    }

    Ok(items)
}

/// Extract test methods from a class definition.
fn extract_class_methods(
    class_def: &ast::StmtClassDef,
    path_str: &str,
    path: &Path,
    line_index: &LineIndex,
    items: &mut Vec<TestItem>,
) {
    let class_name = class_def.name.as_str();

    for stmt in &class_def.body {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                let name = func_def.name.as_str();
                if is_test_function_name(name) {
                    let item = build_test_item_from_function(
                        name,
                        &func_def.args,
                        &func_def.decorator_list,
                        false,
                        func_def.range(),
                        Some(class_name),
                        path_str,
                        path,
                        line_index,
                    );
                    items.push(item);
                }
            }
            Stmt::AsyncFunctionDef(func_def) => {
                let name = func_def.name.as_str();
                if is_test_function_name(name) {
                    let item = build_test_item_from_function(
                        name,
                        &func_def.args,
                        &func_def.decorator_list,
                        true,
                        func_def.range(),
                        Some(class_name),
                        path_str,
                        path,
                        line_index,
                    );
                    items.push(item);
                }
            }
            _ => {}
        }
    }
}

/// Build a TestItem from function definition components.
#[allow(clippy::too_many_arguments)]
fn build_test_item_from_function(
    func_name: &str,
    args: &ast::Arguments,
    decorator_list: &[Expr],
    is_async: bool,
    range: rustpython_parser::text_size::TextRange,
    class_name: Option<&str>,
    path_str: &str,
    path: &Path,
    line_index: &LineIndex,
) -> TestItem {
    let line_number = line_index.line_number(range.start());

    let fixture_deps = extract_fixture_deps(args, class_name.is_some());
    let decorators = extract_decorators(decorator_list);

    let id = if let Some(cls) = class_name {
        format!("{}::{}::{}", path_str, cls, func_name)
    } else {
        format!("{}::{}", path_str, func_name)
    };

    TestItem {
        id,
        path: path.to_path_buf(),
        function_name: func_name.to_string(),
        line_number: Some(line_number),
        decorators,
        is_async,
        fixture_deps,
        class_name: class_name.map(|s| s.to_string()),
        markers: Vec::new(),
        parameters: None,
        name: func_name.to_string(),
    }
}

/// Extract fixture dependencies from function arguments, excluding `self`.
fn extract_fixture_deps(args: &ast::Arguments, is_method: bool) -> Vec<String> {
    let mut deps = Vec::new();

    let all_args = args
        .posonlyargs
        .iter()
        .chain(args.args.iter())
        .chain(args.kwonlyargs.iter());

    for (i, arg_with_default) in all_args.enumerate() {
        let arg_name = arg_with_default.def.arg.as_str();
        // Skip `self` (first arg of methods)
        if is_method && i == 0 && arg_name == "self" {
            continue;
        }
        deps.push(arg_name.to_string());
    }

    deps
}

/// Extract decorator names from a list of decorator expressions.
///
/// Handles three forms:
/// - `@decorator` (Name)
/// - `@module.decorator` (Attribute)
/// - `@decorator(args)` (Call wrapping Name or Attribute)
fn extract_decorators(decorator_list: &[Expr]) -> Vec<String> {
    decorator_list
        .iter()
        .filter_map(extract_decorator_name)
        .collect()
}

/// Extract the name from a single decorator expression.
fn extract_decorator_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Name(name) => Some(name.id.to_string()),
        Expr::Attribute(attr) => {
            let base = extract_decorator_name(&attr.value)?;
            Some(format!("{}.{}", base, attr.attr.as_str()))
        }
        Expr::Call(call) => extract_decorator_name(&call.func),
        _ => None,
    }
}

/// Check if a function name matches the test function naming convention.
fn is_test_function_name(name: &str) -> bool {
    name.starts_with("test_") || name == "test"
}

/// Check if a class name matches the test class naming convention.
fn is_test_class_name(name: &str) -> bool {
    name.starts_with("Test")
}

/// A simple line index that maps byte offsets to 1-based line numbers.
struct LineIndex {
    /// Byte offsets of the start of each line (0-indexed line numbers).
    line_starts: Vec<u32>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        LineIndex { line_starts }
    }

    /// Convert a byte offset (TextSize) to a 1-based line number.
    fn line_number(&self, offset: rustpython_parser::text_size::TextSize) -> usize {
        let offset: u32 = offset.into();
        // Binary search for the line containing this offset
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line + 1,
            Err(line) => line, // line is the index of the next line start, so line == 1-based line number
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_simple_functions() {
        let source = r#"
def test_addition():
    assert 1 + 1 == 2

def test_subtraction():
    assert 2 - 1 == 1

def helper_function():
    return 42
"#;
        let path = PathBuf::from("tests/test_math.py");
        let items = parse_test_file(source, &path).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].function_name, "test_addition");
        assert_eq!(items[1].function_name, "test_subtraction");

        // Verify line numbers (1-based)
        assert_eq!(items[0].line_number, Some(2));
        assert_eq!(items[1].line_number, Some(5));

        // Verify IDs
        assert_eq!(items[0].id, "tests/test_math.py::test_addition");
        assert_eq!(items[1].id, "tests/test_math.py::test_subtraction");

        // Helper function should not be included
        assert!(items.iter().all(|i| i.function_name != "helper_function"));

        // Should not be async
        assert!(!items[0].is_async);
        assert!(!items[1].is_async);
    }

    #[test]
    fn test_parse_class_tests() {
        let source = r#"
class TestCalculator:
    def test_add(self):
        assert 1 + 1 == 2

    def test_multiply(self, tmp_path):
        pass

    def helper(self):
        pass

class HelperClass:
    def test_should_not_appear(self):
        pass
"#;
        let path = PathBuf::from("tests/test_calc.py");
        let items = parse_test_file(source, &path).unwrap();

        // Only tests from TestCalculator (HelperClass is not Test*)
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].function_name, "test_add");
        assert_eq!(items[0].class_name, Some("TestCalculator".to_string()));
        assert_eq!(items[0].id, "tests/test_calc.py::TestCalculator::test_add");

        assert_eq!(items[1].function_name, "test_multiply");
        assert_eq!(items[1].class_name, Some("TestCalculator".to_string()));

        // `self` should be excluded from fixture_deps
        assert!(items[0].fixture_deps.is_empty());
        // `tmp_path` should be in fixture_deps (self excluded)
        assert_eq!(items[1].fixture_deps, vec!["tmp_path"]);
    }

    #[test]
    fn test_parse_fixtures() {
        let source = r#"
def test_with_fixtures(tmp_path, db_connection, capsys):
    pass

def test_no_fixtures():
    pass
"#;
        let path = PathBuf::from("tests/test_fixtures.py");
        let items = parse_test_file(source, &path).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].fixture_deps,
            vec!["tmp_path", "db_connection", "capsys"]
        );
        assert!(items[1].fixture_deps.is_empty());
    }

    #[test]
    fn test_parse_async() {
        let source = r#"
async def test_async_fetch():
    await something()

def test_sync():
    pass

async def test_async_db(db):
    await db.query()
"#;
        let path = PathBuf::from("tests/test_async.py");
        let items = parse_test_file(source, &path).unwrap();

        assert_eq!(items.len(), 3);

        assert_eq!(items[0].function_name, "test_async_fetch");
        assert!(items[0].is_async);
        assert!(items[0].fixture_deps.is_empty());

        assert_eq!(items[1].function_name, "test_sync");
        assert!(!items[1].is_async);

        assert_eq!(items[2].function_name, "test_async_db");
        assert!(items[2].is_async);
        assert_eq!(items[2].fixture_deps, vec!["db"]);
    }

    #[test]
    fn test_parse_decorators() {
        let source = r#"
import pytest

@pytest.mark.slow
def test_slow():
    pass

@pytest.mark.parametrize("x", [1, 2, 3])
def test_param(x):
    pass

@custom_decorator
def test_custom():
    pass

@pytest.mark.skipif(True, reason="skip")
@pytest.mark.timeout(30)
def test_multiple_decorators():
    pass
"#;
        let path = PathBuf::from("tests/test_decorators.py");
        let items = parse_test_file(source, &path).unwrap();

        assert_eq!(items.len(), 4);

        assert_eq!(items[0].function_name, "test_slow");
        assert_eq!(items[0].decorators, vec!["pytest.mark.slow"]);

        assert_eq!(items[1].function_name, "test_param");
        assert_eq!(items[1].decorators, vec!["pytest.mark.parametrize"]);
        assert_eq!(items[1].fixture_deps, vec!["x"]);

        assert_eq!(items[2].function_name, "test_custom");
        assert_eq!(items[2].decorators, vec!["custom_decorator"]);

        assert_eq!(items[3].function_name, "test_multiple_decorators");
        assert_eq!(items[3].decorators.len(), 2);
        assert_eq!(items[3].decorators[0], "pytest.mark.skipif");
        assert_eq!(items[3].decorators[1], "pytest.mark.timeout");
    }

    #[test]
    fn test_line_index() {
        let source = "line1\nline2\nline3\n";
        let index = LineIndex::new(source);
        // "line1" starts at offset 0 -> line 1
        assert_eq!(
            index.line_number(rustpython_parser::text_size::TextSize::from(0)),
            1
        );
        // "line2" starts at offset 6 -> line 2
        assert_eq!(
            index.line_number(rustpython_parser::text_size::TextSize::from(6)),
            2
        );
        // "line3" starts at offset 12 -> line 3
        assert_eq!(
            index.line_number(rustpython_parser::text_size::TextSize::from(12)),
            3
        );
    }
}
