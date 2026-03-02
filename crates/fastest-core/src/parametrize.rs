//! Expansion of `@pytest.mark.parametrize` decorated tests.
//!
//! Takes a list of [`TestItem`]s and expands any that carry one or more
//! `@pytest.mark.parametrize(...)` decorators into multiple test items,
//! one per parameter set. Multiple decorators on the same function produce
//! a cross-product of all parameter combinations.

use crate::error::{Error, Result};
use crate::model::{Parameters, TestItem};
use rustpython_parser::ast::{self, Constant, Expr, Stmt};
use rustpython_parser::Parse;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Expand parametrized tests into individual test items.
///
/// For each [`TestItem`] whose `decorators` contain `"pytest.mark.parametrize"`,
/// this function re-parses the source file, extracts the parametrize arguments
/// from the AST, and expands into one [`TestItem`] per parameter set.
///
/// Tests without parametrize decorators pass through unchanged.
/// If parsing fails for a given test, it passes through unexpanded.
pub fn expand_parametrized_tests(tests: Vec<TestItem>) -> Result<Vec<TestItem>> {
    let mut result = Vec::with_capacity(tests.len());

    // Group tests by file to avoid re-parsing the same file multiple times
    let mut by_file: HashMap<std::path::PathBuf, Vec<TestItem>> = HashMap::new();
    let mut non_parametrized: Vec<TestItem> = Vec::new();

    for test in tests {
        if test.decorators.contains(&"pytest.mark.parametrize".to_string()) {
            by_file.entry(test.path.clone()).or_default().push(test);
        } else {
            non_parametrized.push(test);
        }
    }

    result.extend(non_parametrized);

    for (path, tests_in_file) in by_file {
        let source = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => {
                // If we can't read the file, pass tests through unexpanded
                result.extend(tests_in_file);
                continue;
            }
        };

        let stmts = match ast::Suite::parse(&source, &path.display().to_string()) {
            Ok(s) => s,
            Err(_) => {
                result.extend(tests_in_file);
                continue;
            }
        };

        for test in tests_in_file {
            match expand_single_test(&test, &stmts) {
                Some(expanded) => result.extend(expanded),
                None => result.push(test),
            }
        }
    }

    Ok(result)
}

/// A single `@pytest.mark.parametrize(...)` specification parsed from the AST.
struct ParametrizeSpec {
    /// Parameter names (e.g., `["x"]` or `["x", "y", "expected"]`)
    names: Vec<String>,
    /// Each element is one set of values. For a single parameter this is `vec![value]`.
    value_sets: Vec<Vec<serde_json::Value>>,
    /// Optional explicit IDs for each parameter set
    ids: Option<Vec<String>>,
}

/// Try to expand a single test item using parametrize decorators found in the AST.
/// Returns `None` if expansion cannot be performed (parse failure, no matching function, etc.).
fn expand_single_test(test: &TestItem, stmts: &[Stmt]) -> Option<Vec<TestItem>> {
    let specs = find_parametrize_specs(test, stmts)?;
    if specs.is_empty() {
        return None;
    }

    // Compute the cross-product of all parametrize specs
    let combinations = cross_product(&specs);
    if combinations.is_empty() {
        return None;
    }

    let mut expanded = Vec::with_capacity(combinations.len());

    for combo in &combinations {
        let mut names = Vec::new();
        let mut values = HashMap::new();
        let mut id_parts = Vec::new();

        for (spec_idx, set_idx) in combo {
            let spec = &specs[*spec_idx];
            let vals = &spec.value_sets[*set_idx];

            for (i, name) in spec.names.iter().enumerate() {
                names.push(name.clone());
                if let Some(val) = vals.get(i) {
                    values.insert(name.clone(), val.clone());
                }
            }

            // Build ID part for this spec
            if let Some(ref ids) = spec.ids {
                if let Some(id) = ids.get(*set_idx) {
                    id_parts.push(id.clone());
                }
            } else {
                // Generate ID from values
                let part: Vec<String> = vals.iter().map(|v| value_to_id_string(v)).collect();
                id_parts.push(part.join("-"));
            }
        }

        let id_suffix = id_parts.join("-");
        let new_id = format!("{}[{}]", test.id, id_suffix);
        let new_name = format!("{}[{}]", test.name, id_suffix);

        let mut item = test.clone();
        item.id = new_id;
        item.name = new_name;
        item.parameters = Some(Parameters {
            names: names.clone(),
            values,
            id_suffix,
        });

        expanded.push(item);
    }

    Some(expanded)
}

/// Find all `@pytest.mark.parametrize(...)` decorator Call expressions for a test function.
fn find_parametrize_specs(test: &TestItem, stmts: &[Stmt]) -> Option<Vec<ParametrizeSpec>> {
    let decorators = find_function_decorators(test, stmts)?;
    let mut specs = Vec::new();

    for decorator_expr in decorators {
        if let Some(call) = extract_parametrize_call(decorator_expr) {
            if let Some(spec) = parse_parametrize_call(call) {
                specs.push(spec);
            }
        }
    }

    if specs.is_empty() {
        None
    } else {
        Some(specs)
    }
}

/// Find the decorator expressions for the function matching `test`.
fn find_function_decorators<'a>(test: &TestItem, stmts: &'a [Stmt]) -> Option<Vec<&'a Expr>> {
    for stmt in stmts {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                if test.class_name.is_none() && func_def.name.as_str() == test.function_name {
                    return Some(func_def.decorator_list.iter().collect());
                }
            }
            Stmt::AsyncFunctionDef(func_def) => {
                if test.class_name.is_none() && func_def.name.as_str() == test.function_name {
                    return Some(func_def.decorator_list.iter().collect());
                }
            }
            Stmt::ClassDef(class_def) => {
                if let Some(ref cls) = test.class_name {
                    if class_def.name.as_str() == cls.as_str() {
                        return find_function_decorators_in_class(test, &class_def.body);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Find function decorators inside a class body.
fn find_function_decorators_in_class<'a>(
    test: &TestItem,
    body: &'a [Stmt],
) -> Option<Vec<&'a Expr>> {
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                if func_def.name.as_str() == test.function_name {
                    return Some(func_def.decorator_list.iter().collect());
                }
            }
            Stmt::AsyncFunctionDef(func_def) => {
                if func_def.name.as_str() == test.function_name {
                    return Some(func_def.decorator_list.iter().collect());
                }
            }
            _ => {}
        }
    }
    None
}

/// Check if an expression is a `pytest.mark.parametrize(...)` call and return the Call node.
fn extract_parametrize_call(expr: &Expr) -> Option<&ast::ExprCall> {
    if let Expr::Call(call) = expr {
        if is_parametrize_name(&call.func) {
            return Some(call);
        }
    }
    None
}

/// Check whether an expression represents `pytest.mark.parametrize`.
fn is_parametrize_name(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(attr) => {
            if attr.attr.as_str() != "parametrize" {
                return false;
            }
            // Check for `pytest.mark`
            if let Expr::Attribute(inner) = attr.value.as_ref() {
                if inner.attr.as_str() != "mark" {
                    return false;
                }
                if let Expr::Name(name) = inner.value.as_ref() {
                    return name.id.as_str() == "pytest";
                }
            }
            false
        }
        _ => false,
    }
}

/// Parse a `pytest.mark.parametrize(argnames, argvalues, ids=...)` call into a [`ParametrizeSpec`].
fn parse_parametrize_call(call: &ast::ExprCall) -> Option<ParametrizeSpec> {
    // First positional arg: parameter names (string like "x" or "x,y,expected")
    let names_expr = call.args.first()?;
    let names = parse_param_names(names_expr)?;

    // Second positional arg: values (list of values or list of tuples)
    let values_expr = call.args.get(1)?;
    let value_sets = parse_param_values(values_expr, names.len())?;

    // Optional `ids=` keyword argument
    let ids = parse_ids_kwarg(call);

    Some(ParametrizeSpec {
        names,
        value_sets,
        ids,
    })
}

/// Parse parameter names from the first argument of `@pytest.mark.parametrize`.
/// Handles: `"x"` and `"x,y,expected"`
fn parse_param_names(expr: &Expr) -> Option<Vec<String>> {
    if let Expr::Constant(c) = expr {
        if let Constant::Str(s) = &c.value {
            let names: Vec<String> = s.split(',').map(|n| n.trim().to_string()).collect();
            if names.iter().all(|n| !n.is_empty()) {
                return Some(names);
            }
        }
    }
    None
}

/// Parse parameter values from the second argument of `@pytest.mark.parametrize`.
/// For a single parameter: `[1, 2, 3]` -> `[[1], [2], [3]]`
/// For multiple parameters: `[(1,2,3), (4,5,9)]` -> `[[1,2,3], [4,5,9]]`
fn parse_param_values(expr: &Expr, num_names: usize) -> Option<Vec<Vec<serde_json::Value>>> {
    let elements = match expr {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        _ => return None,
    };

    let mut result = Vec::new();

    for elem in elements {
        if num_names == 1 {
            // Single parameter: each element is a single value
            result.push(vec![expr_to_json(elem)]);
        } else {
            // Multiple parameters: each element should be a tuple
            match elem {
                Expr::Tuple(tuple) => {
                    let vals: Vec<serde_json::Value> =
                        tuple.elts.iter().map(expr_to_json).collect();
                    result.push(vals);
                }
                Expr::List(list) => {
                    let vals: Vec<serde_json::Value> =
                        list.elts.iter().map(expr_to_json).collect();
                    result.push(vals);
                }
                _ => {
                    // Fallback: treat as single value in a list
                    result.push(vec![expr_to_json(elem)]);
                }
            }
        }
    }

    Some(result)
}

/// Parse the optional `ids=` keyword argument.
fn parse_ids_kwarg(call: &ast::ExprCall) -> Option<Vec<String>> {
    for kw in &call.keywords {
        if let Some(ref arg) = kw.arg {
            if arg.as_str() == "ids" {
                return parse_ids_value(&kw.value);
            }
        }
    }
    None
}

/// Parse an `ids=[...]` value into a list of string IDs.
fn parse_ids_value(expr: &Expr) -> Option<Vec<String>> {
    let elements = match expr {
        Expr::List(list) => &list.elts,
        Expr::Tuple(tuple) => &tuple.elts,
        _ => return None,
    };

    let mut ids = Vec::new();
    for elem in elements {
        if let Expr::Constant(c) = elem {
            if let Constant::Str(s) = &c.value {
                ids.push(s.clone());
            } else {
                ids.push(constant_to_string(&c.value));
            }
        } else {
            ids.push(format!("{}", ids.len()));
        }
    }
    Some(ids)
}

/// Convert an AST expression to a JSON value for storage in [`Parameters`].
fn expr_to_json(expr: &Expr) -> serde_json::Value {
    match expr {
        Expr::Constant(c) => constant_to_json(&c.value),
        Expr::UnaryOp(unary) => {
            if let ast::UnaryOp::USub = unary.op {
                if let Expr::Constant(c) = unary.operand.as_ref() {
                    match &c.value {
                        Constant::Int(n) => {
                            let s = n.to_string();
                            if let Ok(i) = s.parse::<i64>() {
                                return serde_json::Value::Number((-i).into());
                            }
                        }
                        Constant::Float(f) => {
                            if let Some(n) = serde_json::Number::from_f64(-f) {
                                return serde_json::Value::Number(n);
                            }
                        }
                        _ => {}
                    }
                }
            }
            serde_json::Value::Null
        }
        Expr::List(list) => {
            let items: Vec<serde_json::Value> = list.elts.iter().map(expr_to_json).collect();
            serde_json::Value::Array(items)
        }
        Expr::Tuple(tuple) => {
            let items: Vec<serde_json::Value> = tuple.elts.iter().map(expr_to_json).collect();
            serde_json::Value::Array(items)
        }
        Expr::Name(name) => {
            match name.id.as_str() {
                "True" => serde_json::Value::Bool(true),
                "False" => serde_json::Value::Bool(false),
                "None" => serde_json::Value::Null,
                _ => serde_json::Value::String(name.id.to_string()),
            }
        }
        _ => serde_json::Value::Null,
    }
}

/// Convert a Python constant to a JSON value.
fn constant_to_json(c: &Constant) -> serde_json::Value {
    match c {
        Constant::Int(n) => {
            let s = n.to_string();
            if let Ok(i) = s.parse::<i64>() {
                serde_json::Value::Number(i.into())
            } else {
                serde_json::Value::String(s)
            }
        }
        Constant::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Constant::Str(s) => serde_json::Value::String(s.clone()),
        Constant::Bool(b) => serde_json::Value::Bool(*b),
        Constant::None => serde_json::Value::Null,
        Constant::Tuple(items) => {
            let json_items: Vec<serde_json::Value> =
                items.iter().map(constant_to_json).collect();
            serde_json::Value::Array(json_items)
        }
        _ => serde_json::Value::Null,
    }
}

/// Convert a constant to a display string for use in test IDs.
fn constant_to_string(c: &Constant) -> String {
    match c {
        Constant::Int(n) => n.to_string(),
        Constant::Float(f) => f.to_string(),
        Constant::Str(s) => s.clone(),
        Constant::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Constant::None => "None".to_string(),
        _ => "?".to_string(),
    }
}

/// Convert a JSON value to a compact string for use in test IDs.
fn value_to_id_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(value_to_id_string).collect();
            parts.join("-")
        }
        serde_json::Value::Object(_) => "object".to_string(),
    }
}

/// Compute the cross-product of all parametrize specs.
///
/// Each combination is represented as a vec of `(spec_index, value_set_index)` pairs.
fn cross_product(specs: &[ParametrizeSpec]) -> Vec<Vec<(usize, usize)>> {
    let mut combinations: Vec<Vec<(usize, usize)>> = vec![vec![]];

    for (spec_idx, spec) in specs.iter().enumerate() {
        let mut new_combinations = Vec::new();
        for combo in &combinations {
            for set_idx in 0..spec.value_sets.len() {
                let mut new_combo = combo.clone();
                new_combo.push((spec_idx, set_idx));
                new_combinations.push(new_combo);
            }
        }
        combinations = new_combinations;
    }

    combinations
}

/// Expand parametrized tests from in-memory source code rather than reading from disk.
/// This is primarily used for testing.
pub fn expand_parametrized_tests_from_source(
    tests: Vec<TestItem>,
    source: &str,
    path: &Path,
) -> Result<Vec<TestItem>> {
    let stmts = ast::Suite::parse(source, &path.display().to_string())
        .map_err(|e| Error::Parse(format!("Failed to parse {}: {}", path.display(), e)))?;

    let mut result = Vec::with_capacity(tests.len());

    for test in tests {
        if test.decorators.contains(&"pytest.mark.parametrize".to_string()) {
            match expand_single_test(&test, &stmts) {
                Some(expanded) => result.extend(expanded),
                None => result.push(test),
            }
        } else {
            result.push(test);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TestItem;
    use std::path::PathBuf;

    /// Helper to create a minimal TestItem for testing
    fn make_test_item(
        function_name: &str,
        decorators: Vec<&str>,
        path: &Path,
    ) -> TestItem {
        let id = format!("{}::{}", path.display(), function_name);
        TestItem {
            id,
            path: path.to_path_buf(),
            function_name: function_name.to_string(),
            line_number: Some(1),
            decorators: decorators.into_iter().map(String::from).collect(),
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
            name: function_name.to_string(),
        }
    }

    #[test]
    fn test_expand_simple_parametrize() {
        let source = r#"
import pytest

@pytest.mark.parametrize("x", [1, 2, 3])
def test_square(x):
    assert x * x >= 0
"#;
        let path = PathBuf::from("tests/test_math.py");
        let test = make_test_item(
            "test_square",
            vec!["pytest.mark.parametrize"],
            &path,
        );

        let result =
            expand_parametrized_tests_from_source(vec![test], source, &path).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "test_square[1]");
        assert_eq!(result[1].name, "test_square[2]");
        assert_eq!(result[2].name, "test_square[3]");

        // Check parameters are set
        let params = result[0].parameters.as_ref().unwrap();
        assert_eq!(params.names, vec!["x"]);
        assert_eq!(params.values.get("x"), Some(&serde_json::json!(1)));
        assert_eq!(params.id_suffix, "1");

        let params = result[2].parameters.as_ref().unwrap();
        assert_eq!(params.values.get("x"), Some(&serde_json::json!(3)));
        assert_eq!(params.id_suffix, "3");
    }

    #[test]
    fn test_expand_multi_param() {
        let source = r#"
import pytest

@pytest.mark.parametrize("x,y,expected", [(1, 2, 3), (4, 5, 9)])
def test_add(x, y, expected):
    assert x + y == expected
"#;
        let path = PathBuf::from("tests/test_math.py");
        let test = make_test_item(
            "test_add",
            vec!["pytest.mark.parametrize"],
            &path,
        );

        let result =
            expand_parametrized_tests_from_source(vec![test], source, &path).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "test_add[1-2-3]");
        assert_eq!(result[1].name, "test_add[4-5-9]");

        let params = result[0].parameters.as_ref().unwrap();
        assert_eq!(params.names, vec!["x", "y", "expected"]);
        assert_eq!(params.values.get("x"), Some(&serde_json::json!(1)));
        assert_eq!(params.values.get("y"), Some(&serde_json::json!(2)));
        assert_eq!(params.values.get("expected"), Some(&serde_json::json!(3)));

        let params = result[1].parameters.as_ref().unwrap();
        assert_eq!(params.values.get("x"), Some(&serde_json::json!(4)));
        assert_eq!(params.values.get("y"), Some(&serde_json::json!(5)));
        assert_eq!(params.values.get("expected"), Some(&serde_json::json!(9)));
    }

    #[test]
    fn test_non_parametrized_passthrough() {
        let source = r#"
def test_simple():
    assert True

@pytest.mark.slow
def test_slow():
    pass
"#;
        let path = PathBuf::from("tests/test_basic.py");
        let tests = vec![
            make_test_item("test_simple", vec![], &path),
            make_test_item("test_slow", vec!["pytest.mark.slow"], &path),
        ];

        let result =
            expand_parametrized_tests_from_source(tests, source, &path).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "test_simple");
        assert_eq!(result[1].name, "test_slow");
        assert!(result[0].parameters.is_none());
        assert!(result[1].parameters.is_none());
    }

    #[test]
    fn test_expand_with_ids() {
        let source = r#"
import pytest

@pytest.mark.parametrize("x", [1, 2], ids=["one", "two"])
def test_named(x):
    assert x > 0
"#;
        let path = PathBuf::from("tests/test_ids.py");
        let test = make_test_item(
            "test_named",
            vec!["pytest.mark.parametrize"],
            &path,
        );

        let result =
            expand_parametrized_tests_from_source(vec![test], source, &path).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "test_named[one]");
        assert_eq!(result[1].name, "test_named[two]");

        let params = result[0].parameters.as_ref().unwrap();
        assert_eq!(params.id_suffix, "one");
        assert_eq!(params.values.get("x"), Some(&serde_json::json!(1)));

        let params = result[1].parameters.as_ref().unwrap();
        assert_eq!(params.id_suffix, "two");
        assert_eq!(params.values.get("x"), Some(&serde_json::json!(2)));
    }

    #[test]
    fn test_cross_product_multiple_decorators() {
        let source = r#"
import pytest

@pytest.mark.parametrize("x", [1, 2])
@pytest.mark.parametrize("y", [10, 20])
def test_multiply(x, y):
    assert x * y > 0
"#;
        let path = PathBuf::from("tests/test_cross.py");
        let test = make_test_item(
            "test_multiply",
            vec!["pytest.mark.parametrize"],
            &path,
        );

        let result =
            expand_parametrized_tests_from_source(vec![test], source, &path).unwrap();

        assert_eq!(result.len(), 4);

        // Collect all generated names
        let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"test_multiply[1-10]"));
        assert!(names.contains(&"test_multiply[1-20]"));
        assert!(names.contains(&"test_multiply[2-10]"));
        assert!(names.contains(&"test_multiply[2-20]"));
    }

    #[test]
    fn test_string_param_values() {
        let source = r#"
import pytest

@pytest.mark.parametrize("name", ["alice", "bob"])
def test_greet(name):
    assert len(name) > 0
"#;
        let path = PathBuf::from("tests/test_strings.py");
        let test = make_test_item(
            "test_greet",
            vec!["pytest.mark.parametrize"],
            &path,
        );

        let result =
            expand_parametrized_tests_from_source(vec![test], source, &path).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "test_greet[alice]");
        assert_eq!(result[1].name, "test_greet[bob]");
    }

    #[test]
    fn test_expand_preserves_test_fields() {
        let source = r#"
import pytest

@pytest.mark.parametrize("x", [1])
def test_one(x):
    pass
"#;
        let path = PathBuf::from("tests/test_fields.py");
        let mut test = make_test_item(
            "test_one",
            vec!["pytest.mark.parametrize"],
            &path,
        );
        test.is_async = false;
        test.fixture_deps = vec!["x".to_string()];
        test.line_number = Some(4);

        let result =
            expand_parametrized_tests_from_source(vec![test], source, &path).unwrap();

        assert_eq!(result.len(), 1);
        let expanded = &result[0];
        assert_eq!(expanded.path, path);
        assert_eq!(expanded.function_name, "test_one");
        assert_eq!(expanded.fixture_deps, vec!["x"]);
        assert_eq!(expanded.line_number, Some(4));
        assert!(!expanded.is_async);
    }
}
