use crate::error::Result;
use crate::test::discovery::TestItem;
use crate::utils::simd_json; // 🚀 REVOLUTIONARY SIMD JSON OPTIMIZATION
use rustpython_parser::ast;
use rustpython_parser::Parse;
use serde_json::Value;
use std::collections::HashMap;

/// Represents a parametrized test case
#[derive(Debug, Clone)]
pub struct ParametrizedTest {
    pub base_test: TestItem,
    pub param_sets: Vec<ParamSet>,
}

/// A set of parameters for a single test invocation
#[derive(Debug, Clone)]
pub struct ParamSet {
    pub id: Option<String>,
    pub values: Vec<Value>,
    pub marks: Vec<String>,
    pub is_xfail: bool,
}

/// Parse parametrize decorator and extract parameter information
pub fn parse_parametrize_decorator(
    decorator: &str,
) -> Option<(Vec<String>, Vec<ParamSet>, Option<Vec<String>>)> {
    // Remove @ prefix if present and extract the call expression
    let cleaned = decorator.trim_start_matches('@');

    // Parse as a function call expression
    let expr = ast::Expr::parse(cleaned, "<decorator>").ok()?;

    // Handle both direct calls and attribute calls
    match &expr {
        ast::Expr::Call(call) => parse_parametrize_call(call),
        ast::Expr::Attribute(attr) => {
            // Handle pytest.mark.parametrize(...) style
            if attr.attr.as_str() == "parametrize" {
                if let ast::Expr::Attribute(mark_attr) = attr.value.as_ref() {
                    if mark_attr.attr.as_str() == "mark" {
                        // This is just pytest.mark.parametrize without the call
                        // The actual call should be in the parent expression
                        None
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse parametrize decorator from AST expression
pub fn parse_parametrize_expr(
    expr: &ast::Expr,
) -> Option<(Vec<String>, Vec<ParamSet>, Option<Vec<String>>)> {
    match expr {
        ast::Expr::Call(call) => parse_parametrize_call(call),
        _ => None,
    }
}

fn parse_parametrize_call(
    call: &ast::ExprCall,
) -> Option<(Vec<String>, Vec<ParamSet>, Option<Vec<String>>)> {
    // Check if it's a parametrize call
    if !is_parametrize_call(&call.func) {
        return None;
    }

    // Extract parameter names (first argument)
    let param_names = call.args.get(0).and_then(extract_param_names)?;

    // Extract parameter values (second argument)
    let values = call.args.get(1)?;

    // Extract ids if provided
    let ids = call
        .keywords
        .iter()
        .find(|kw| kw.arg.as_deref() == Some("ids"))
        .and_then(|kw| extract_ids(&kw.value));

    // Extract indirect parameter if provided
    let indirect = call
        .keywords
        .iter()
        .find(|kw| kw.arg.as_deref() == Some("indirect"))
        .and_then(|kw| extract_indirect(&kw.value, &param_names));

    // Parse parameter sets
    let param_sets = parse_param_values(values, &param_names, &ids)?;

    Some((param_names, param_sets, indirect))
}

fn is_parametrize_call(func: &ast::Expr) -> bool {
    expr_to_string(func).contains("parametrize")
}

fn extract_param_names(expr: &ast::Expr) -> Option<Vec<String>> {
    match expr {
        ast::Expr::Constant(c) => {
            if let ast::Constant::Str(s) = &c.value {
                Some(s.split(',').map(|s| s.trim().to_string()).collect())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_ids(expr: &ast::Expr) -> Option<Vec<String>> {
    match expr {
        ast::Expr::List(list) => Some(
            list.elts
                .iter()
                .filter_map(|e| {
                    if let ast::Expr::Constant(c) = e {
                        if let ast::Constant::Str(s) = &c.value {
                            return Some(s.clone());
                        }
                    }
                    None
                })
                .collect(),
        ),
        _ => None,
    }
}

fn extract_indirect(expr: &ast::Expr, param_names: &[String]) -> Option<Vec<String>> {
    match expr {
        // indirect=True means all parameters are indirect
        ast::Expr::Constant(c) if matches!(&c.value, ast::Constant::Bool(true)) => {
            Some(param_names.to_vec())
        }
        // indirect="param" means a single parameter is indirect
        ast::Expr::Constant(c) if matches!(&c.value, ast::Constant::Str(_)) => {
            if let ast::Constant::Str(s) = &c.value {
                Some(vec![s.clone()])
            } else {
                None
            }
        }
        // indirect=["param1", "param2"] means specific parameters are indirect
        ast::Expr::List(list) => Some(
            list.elts
                .iter()
                .filter_map(|e| {
                    if let ast::Expr::Constant(c) = e {
                        if let ast::Constant::Str(s) = &c.value {
                            return Some(s.clone());
                        }
                    }
                    None
                })
                .collect(),
        ),
        _ => None,
    }
}

fn parse_param_values(
    expr: &ast::Expr,
    param_names: &[String],
    ids: &Option<Vec<String>>,
) -> Option<Vec<ParamSet>> {
    match expr {
        ast::Expr::List(list) => {
            let mut param_sets = Vec::new();

            for (idx, item) in list.elts.iter().enumerate() {
                let mut param_set = parse_single_param_set(item, param_names.len())?;

                // Apply overall id if no specific id
                if param_set.id.is_none() {
                    if let Some(ids_vec) = ids {
                        if let Some(id) = ids_vec.get(idx) {
                            param_set.id = Some(id.clone());
                        }
                    }
                }

                param_sets.push(param_set);
            }

            Some(param_sets)
        }
        _ => None,
    }
}

fn parse_single_param_set(expr: &ast::Expr, expected_params: usize) -> Option<ParamSet> {
    match expr {
        // pytest.param(...) call
        ast::Expr::Call(call) if is_pytest_param_call(&call.func) => {
            parse_pytest_param(call, expected_params)
        }
        // Single value (for single parameter)
        _ if expected_params == 1 => Some(ParamSet {
            id: None,
            values: vec![ast_expr_to_json(expr)],
            marks: Vec::new(),
            is_xfail: false,
        }),
        // Tuple of values
        ast::Expr::Tuple(tuple) if tuple.elts.len() == expected_params => Some(ParamSet {
            id: None,
            values: tuple.elts.iter().map(ast_expr_to_json).collect(),
            marks: Vec::new(),
            is_xfail: false,
        }),
        // List of values (less common but valid)
        ast::Expr::List(list) if list.elts.len() == expected_params => Some(ParamSet {
            id: None,
            values: list.elts.iter().map(ast_expr_to_json).collect(),
            marks: Vec::new(),
            is_xfail: false,
        }),
        _ => None,
    }
}

fn is_pytest_param_call(func: &ast::Expr) -> bool {
    expr_to_string(func).ends_with("param")
}

fn parse_pytest_param(call: &ast::ExprCall, expected_params: usize) -> Option<ParamSet> {
    // Extract values from positional arguments
    let values: Vec<Value> = call
        .args
        .iter()
        .take(expected_params)
        .map(ast_expr_to_json)
        .collect();

    if values.len() != expected_params {
        return None;
    }

    let mut id = None;
    let mut marks = Vec::new();
    let mut is_xfail = false;

    // Process keyword arguments
    for kw in &call.keywords {
        match kw.arg.as_deref() {
            Some("id") => {
                if let ast::Expr::Constant(c) = &kw.value {
                    if let ast::Constant::Str(s) = &c.value {
                        id = Some(s.clone());
                    }
                }
            }
            Some("marks") => {
                let extracted_marks = extract_marks(&kw.value);
                is_xfail = extracted_marks.iter().any(|m| m == "xfail");
                marks = extracted_marks;
            }
            _ => {}
        }
    }

    Some(ParamSet {
        id,
        values,
        marks,
        is_xfail,
    })
}

fn extract_marks(expr: &ast::Expr) -> Vec<String> {
    match expr {
        ast::Expr::List(list) => list
            .elts
            .iter()
            .filter_map(|e| {
                let s = expr_to_string(e);
                if s.contains("xfail") {
                    Some("xfail".to_string())
                } else if s.contains("skip") {
                    Some("skip".to_string())
                } else {
                    None
                }
            })
            .collect(),
        _ => {
            let s = expr_to_string(expr);
            if s.contains("xfail") {
                vec!["xfail".to_string()]
            } else {
                vec![]
            }
        }
    }
}

fn ast_expr_to_json(expr: &ast::Expr) -> Value {
    match expr {
        ast::Expr::Constant(c) => constant_to_json(&c.value),
        ast::Expr::List(list) => Value::Array(list.elts.iter().map(ast_expr_to_json).collect()),
        ast::Expr::Tuple(tuple) => Value::Array(tuple.elts.iter().map(ast_expr_to_json).collect()),
        ast::Expr::Dict(dict) => {
            let mut map = serde_json::Map::new();
            for (key_expr, value_expr) in dict.keys.iter().zip(&dict.values) {
                if let Some(key) = key_expr {
                    if let ast::Expr::Constant(c) = key {
                        if let ast::Constant::Str(s) = &c.value {
                            map.insert(s.clone(), ast_expr_to_json(value_expr));
                        }
                    }
                }
            }
            Value::Object(map)
        }
        ast::Expr::UnaryOp(unop) => {
            if let ast::UnaryOp::USub = unop.op {
                match ast_expr_to_json(&unop.operand) {
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Value::from(-i)
                        } else if let Some(f) = n.as_f64() {
                            serde_json::Number::from_f64(-f)
                                .map(Value::Number)
                                .unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    }
                    _ => Value::Null,
                }
            } else {
                Value::Null
            }
        }
        _ => Value::String(expr_to_string(expr)),
    }
}

fn constant_to_json(constant: &ast::Constant) -> Value {
    match constant {
        ast::Constant::Str(s) => Value::String(s.clone()),
        ast::Constant::Int(i) => {
            if let Ok(num) = i.to_string().parse::<i64>() {
                Value::from(num)
            } else {
                Value::String(i.to_string())
            }
        }
        ast::Constant::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ast::Constant::Bool(b) => Value::Bool(*b),
        ast::Constant::None => Value::Null,
        _ => Value::Null,
    }
}

fn expr_to_string(expr: &ast::Expr) -> String {
    match expr {
        ast::Expr::Name(name) => name.id.as_str().to_string(),
        ast::Expr::Attribute(attr) => {
            format!("{}.{}", expr_to_string(&attr.value), attr.attr.as_str())
        }
        ast::Expr::Call(call) => {
            format!("{}(...)", expr_to_string(&call.func))
        }
        _ => "...".to_string(),
    }
}

/// Expand a test function with parametrize decorators into multiple test items
pub fn expand_parametrized_tests(test: &TestItem, decorators: &[String]) -> Result<Vec<TestItem>> {
    let mut param_info_list = Vec::new();

    // Extract all parametrize decorators
    for decorator in decorators {
        if decorator.contains("parametrize") {
            // Try parsing with @ prefix
            let decorator_with_at = if decorator.starts_with('@') {
                decorator.to_string()
            } else {
                format!("@{}", decorator)
            };

            if let Some((names, param_sets, indirect)) =
                parse_parametrize_decorator(&decorator_with_at)
            {
                param_info_list.push((names, param_sets, indirect));
            }
        }
    }

    // Check for simple xfail marker
    let base_xfail = decorators
        .iter()
        .any(|d| d.contains("xfail") && !d.contains("parametrize"));

    if param_info_list.is_empty() {
        let mut test_clone = test.clone();
        if base_xfail {
            test_clone.is_xfail = true;
        }
        return Ok(vec![test_clone]);
    }

    // Generate test cases
    let test_cases = generate_test_cases(&param_info_list);

    // Create expanded tests
    let mut expanded_tests = Vec::new();
    for case in test_cases {
        let mut expanded_test = test.clone();

        let id_str = case.id.unwrap_or_else(|| format_param_id(&case.params));
        expanded_test.id = format!("{}[{}]", test.id, id_str);
        expanded_test.name = format!("{}[{}]", test.function_name, id_str);
        expanded_test.is_xfail = base_xfail || case.is_xfail;

        // Store parameters
        let params_json = simd_json::to_string(&case.params).unwrap_or_default();
        expanded_test
            .decorators
            .push(format!("__params__={}", params_json));

        // Store indirect parameters if any
        if !case.indirect_params.is_empty() {
            let indirect_json = simd_json::to_string(&case.indirect_params).unwrap_or_default();
            expanded_test
                .decorators
                .push(format!("__indirect__={}", indirect_json));
            
            // Also populate the indirect_params field in TestItem
            for param_name in &case.indirect_params {
                expanded_test.indirect_params.insert(param_name.clone(), true);
            }
        }

        if expanded_test.is_xfail {
            expanded_test.decorators.push("__xfail__=True".to_string());
        }

        expanded_tests.push(expanded_test);
    }

    Ok(expanded_tests)
}

#[derive(Debug)]
struct TestCase {
    params: HashMap<String, Value>,
    indirect_params: Vec<String>,
    id: Option<String>,
    is_xfail: bool,
}

fn generate_test_cases(
    param_info_list: &[(Vec<String>, Vec<ParamSet>, Option<Vec<String>>)],
) -> Vec<TestCase> {
    if param_info_list.is_empty() {
        return vec![];
    }

    // Start with first decorator
    let (first_names, first_sets, first_indirect) = &param_info_list[0];
    let mut cases: Vec<TestCase> = first_sets
        .iter()
        .map(|set| TestCase {
            params: create_params_map(first_names, &set.values),
            indirect_params: first_indirect.clone().unwrap_or_default(),
            id: set.id.clone(),
            is_xfail: set.is_xfail,
        })
        .collect();

    // Cross product with remaining decorators
    for (names, param_sets, indirect) in param_info_list.iter().skip(1) {
        let mut new_cases = Vec::new();

        for case in &cases {
            for set in param_sets {
                let mut params = case.params.clone();

                // Add new parameters
                for (idx, name) in names.iter().enumerate() {
                    if let Some(value) = set.values.get(idx) {
                        params.insert(name.clone(), value.clone());
                    }
                }

                // Merge indirect params
                let mut merged_indirect = case.indirect_params.clone();
                if let Some(indirect_params) = indirect {
                    for param in indirect_params {
                        if !merged_indirect.contains(param) {
                            merged_indirect.push(param.clone());
                        }
                    }
                }

                new_cases.push(TestCase {
                    params,
                    indirect_params: merged_indirect,
                    id: set.id.clone().or_else(|| case.id.clone()),
                    is_xfail: case.is_xfail || set.is_xfail,
                });
            }
        }

        cases = new_cases;
    }

    cases
}

fn create_params_map(names: &[String], values: &[Value]) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    for (name, value) in names.iter().zip(values.iter()) {
        map.insert(name.clone(), value.clone());
    }
    map
}

fn format_param_id(params: &HashMap<String, Value>) -> String {
    let mut parts = Vec::new();
    let mut keys: Vec<_> = params.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = params.get(key) {
            parts.push(format_value(value));
        }
    }

    parts.join("-")
}

fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => {
            // Handle unicode strings by keeping alphanumeric chars (including unicode)
            // and replacing other chars with underscore
            s.chars()
                .map(|c| if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                })
                .collect()
        },
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => "None".to_string(),
        Value::Array(arr) => {
            let items: Vec<_> = arr.iter().map(format_value).collect();
            format!("{}", items.join("_"))
        }
        Value::Object(_) => "object".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_parametrize() {
        let decorator = r#"@pytest.mark.parametrize("x", [1, 2, 3])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets, _ids) = result.unwrap();
        assert_eq!(names, vec!["x"]);
        assert_eq!(param_sets.len(), 3);
        assert_eq!(param_sets[0].values, vec![Value::from(1)]);
    }

    #[test]
    fn test_parse_tuple_parametrize() {
        let decorator = r#"@pytest.mark.parametrize("x,y,expected", [(1,2,3), (4,5,9)])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets, _ids) = result.unwrap();
        assert_eq!(names, vec!["x", "y", "expected"]);
        assert_eq!(param_sets.len(), 2);
        assert_eq!(
            param_sets[0].values,
            vec![Value::from(1), Value::from(2), Value::from(3)]
        );
    }

    #[test]
    fn test_parse_pytest_param() {
        let decorator = r#"@pytest.mark.parametrize("x", [pytest.param(1, id="one"), pytest.param(2, marks=pytest.mark.xfail)])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets, _ids) = result.unwrap();
        assert_eq!(names, vec!["x"]);
        assert_eq!(param_sets.len(), 2);
        assert_eq!(param_sets[0].id, Some("one".to_string()));
        assert!(param_sets[1].is_xfail);
    }

    #[test]
    fn test_parse_with_ids() {
        let decorator = r#"@pytest.mark.parametrize("x", [1, 2], ids=["first", "second"])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());
        let (names, param_sets, _ids) = result.unwrap();
        assert_eq!(names, vec!["x"]);
        assert_eq!(param_sets[0].id, Some("first".to_string()));
        assert_eq!(param_sets[1].id, Some("second".to_string()));
    }

    #[test]
    fn test_expand_simple() {
        let test = TestItem {
            id: "test_module::test_func".to_string(),
            path: std::path::PathBuf::from("test.py"),
            name: "test_func".to_string(),
            function_name: "test_func".to_string(),
            line_number: Some(1),
            is_async: false,
            class_name: None,
            decorators: vec!["pytest.mark.parametrize(\"x\", [1, 2])".to_string()],
            fixture_deps: vec![],
            is_xfail: false,
        };

        let expanded = expand_parametrized_tests(&test, &test.decorators).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0].id, "test_module::test_func[1]");
        assert_eq!(expanded[1].id, "test_module::test_func[2]");
    }
}
