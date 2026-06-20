use crate::error::Result;
use crate::test::discovery::TestItem;
use crate::utils::simd_json; // 🚀 REVOLUTIONARY SIMD JSON OPTIMIZATION
use rustpython_parser::ast;
use rustpython_parser::Parse;
use serde_json::Value;
use std::collections::HashMap;

type ParametrizeInfo = (Vec<String>, Vec<ParamSet>, Option<Vec<String>>);

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
pub fn parse_parametrize_decorator(decorator: &str) -> Option<ParametrizeInfo> {
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
pub fn parse_parametrize_expr(expr: &ast::Expr) -> Option<ParametrizeInfo> {
    match expr {
        ast::Expr::Call(call) => parse_parametrize_call(call),
        _ => None,
    }
}

fn parse_parametrize_call(call: &ast::ExprCall) -> Option<ParametrizeInfo> {
    // Check if it's a parametrize call
    if !is_parametrize_call(&call.func) {
        return None;
    }

    // Extract parameter names (first argument)
    let param_names = call.args.first().and_then(extract_param_names)?;

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
        ast::Expr::List(list) => parse_param_sequence(list.elts.iter(), param_names.len(), ids),
        ast::Expr::Tuple(tuple) => parse_param_sequence(tuple.elts.iter(), param_names.len(), ids),
        ast::Expr::Call(call) if param_names.len() == 1 => parse_range_param_sets(call, ids),
        _ => None,
    }
}

fn parse_param_sequence<'a, I>(
    items: I,
    expected_params: usize,
    ids: &Option<Vec<String>>,
) -> Option<Vec<ParamSet>>
where
    I: IntoIterator<Item = &'a ast::Expr>,
{
    let mut param_sets = Vec::new();

    for (idx, item) in items.into_iter().enumerate() {
        let mut param_set = parse_single_param_set(item, expected_params)?;

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

fn parse_range_param_sets(
    call: &ast::ExprCall,
    ids: &Option<Vec<String>>,
) -> Option<Vec<ParamSet>> {
    let values = expand_range_call(call)?;

    Some(
        values
            .into_iter()
            .enumerate()
            .map(|(idx, value)| ParamSet {
                id: ids.as_ref().and_then(|ids_vec| ids_vec.get(idx).cloned()),
                values: vec![value],
                marks: Vec::new(),
                is_xfail: false,
            })
            .collect(),
    )
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
            } else if s.contains("skip") {
                vec!["skip".to_string()]
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
        ast::Expr::Set(set) => set_to_json(set.elts.iter().map(ast_expr_to_json).collect()),
        ast::Expr::Dict(dict) => {
            let mut map = serde_json::Map::new();
            for (key_expr, value_expr) in dict.keys.iter().zip(&dict.values) {
                if let Some(ast::Expr::Constant(c)) = key_expr {
                    if let ast::Constant::Str(s) = &c.value {
                        map.insert(s.clone(), ast_expr_to_json(value_expr));
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
        ast::Expr::BinOp(binop) => evaluate_string_repetition(binop)
            .map(Value::String)
            .unwrap_or_else(|| Value::String(expr_to_string(expr))),
        ast::Expr::Call(call) => {
            parse_static_call(call).unwrap_or_else(|| Value::String(expr_to_string(expr)))
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
        ast::Constant::Float(f) => float_to_json(*f),
        ast::Constant::Bool(b) => Value::Bool(*b),
        ast::Constant::None => Value::Null,
        _ => Value::Null,
    }
}

fn parse_static_call(call: &ast::ExprCall) -> Option<Value> {
    parse_float_call(call).or_else(|| parse_set_call(call))
}

fn parse_float_call(call: &ast::ExprCall) -> Option<Value> {
    if expr_to_string(&call.func) != "float" || !call.keywords.is_empty() {
        return None;
    }

    if call.args.is_empty() {
        return Some(float_to_json(0.0));
    }

    if call.args.len() != 1 {
        return None;
    }

    let value = match &call.args[0] {
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::Str(s) => parse_float_literal(s)?,
            ast::Constant::Int(i) => i.to_string().parse::<f64>().ok()?,
            ast::Constant::Float(f) => *f,
            _ => return None,
        },
        ast::Expr::UnaryOp(_) => match ast_expr_to_json(&call.args[0]) {
            Value::Number(number) => number.as_f64()?,
            _ => return None,
        },
        _ => return None,
    };

    Some(float_to_json(value))
}

fn parse_float_literal(value: &str) -> Option<f64> {
    match value.trim().to_ascii_lowercase().as_str() {
        "inf" | "+inf" | "infinity" | "+infinity" => Some(f64::INFINITY),
        "-inf" | "-infinity" => Some(f64::NEG_INFINITY),
        "nan" | "+nan" | "-nan" => Some(f64::NAN),
        other => other.parse::<f64>().ok(),
    }
}

fn float_to_json(value: f64) -> Value {
    if value.is_nan() {
        special_float_json("nan")
    } else if value == f64::INFINITY {
        special_float_json("inf")
    } else if value == f64::NEG_INFINITY {
        special_float_json("-inf")
    } else {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

fn special_float_json(kind: &str) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "__fastest_float__".to_string(),
        Value::String(kind.to_string()),
    );
    Value::Object(map)
}

fn parse_set_call(call: &ast::ExprCall) -> Option<Value> {
    if expr_to_string(&call.func) != "set" || !call.keywords.is_empty() {
        return None;
    }

    if call.args.is_empty() {
        return Some(set_to_json(Vec::new()));
    }

    if call.args.len() != 1 {
        return None;
    }

    match &call.args[0] {
        ast::Expr::List(list) => Some(set_to_json(
            list.elts.iter().map(ast_expr_to_json).collect(),
        )),
        ast::Expr::Tuple(tuple) => Some(set_to_json(
            tuple.elts.iter().map(ast_expr_to_json).collect(),
        )),
        ast::Expr::Set(set) => Some(set_to_json(set.elts.iter().map(ast_expr_to_json).collect())),
        _ => None,
    }
}

fn set_to_json(values: Vec<Value>) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("__fastest_set__".to_string(), Value::Array(values));
    Value::Object(map)
}

fn expand_range_call(call: &ast::ExprCall) -> Option<Vec<Value>> {
    if expr_to_string(&call.func) != "range" || !call.keywords.is_empty() {
        return None;
    }

    let args: Option<Vec<i64>> = call.args.iter().map(ast_expr_to_i64).collect();
    let args = args?;

    let (start, stop, step) = match args.as_slice() {
        [stop] => (0, *stop, 1),
        [start, stop] => (*start, *stop, 1),
        [start, stop, step] => (*start, *stop, *step),
        _ => return None,
    };

    if step == 0 {
        return None;
    }

    let mut values = Vec::new();
    let mut current = start;
    while (step > 0 && current < stop) || (step < 0 && current > stop) {
        values.push(Value::from(current));
        current = current.saturating_add(step);
        if values.len() > 100_000 {
            return None;
        }
    }

    Some(values)
}

fn ast_expr_to_i64(expr: &ast::Expr) -> Option<i64> {
    match expr {
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::Int(i) => i.to_string().parse::<i64>().ok(),
            _ => None,
        },
        ast::Expr::UnaryOp(unop) => match unop.op {
            ast::UnaryOp::USub => ast_expr_to_i64(&unop.operand).map(|value| -value),
            ast::UnaryOp::UAdd => ast_expr_to_i64(&unop.operand),
            _ => None,
        },
        _ => None,
    }
}

fn evaluate_string_repetition(binop: &ast::ExprBinOp) -> Option<String> {
    if binop.op != ast::Operator::Mult {
        return None;
    }

    if let (Some(text), Some(count)) = (
        ast_expr_to_string_literal(&binop.left),
        ast_expr_to_repeat_count(&binop.right),
    ) {
        return repeat_string(text, count);
    }

    if let (Some(count), Some(text)) = (
        ast_expr_to_repeat_count(&binop.left),
        ast_expr_to_string_literal(&binop.right),
    ) {
        return repeat_string(text, count);
    }

    None
}

fn ast_expr_to_string_literal(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Constant(c) => match &c.value {
            ast::Constant::Str(s) => Some(s.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn ast_expr_to_repeat_count(expr: &ast::Expr) -> Option<i64> {
    ast_expr_to_i64(expr)
}

fn repeat_string(text: String, count: i64) -> Option<String> {
    let count = usize::try_from(count.max(0)).ok()?;
    if count > 100_000 {
        return None;
    }

    Some(text.repeat(count))
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
                expanded_test
                    .indirect_params
                    .insert(param_name.clone(), true);
            }
        }

        if expanded_test.is_xfail {
            expanded_test.decorators.push(
                "@pytest.mark.xfail(reason=\"Parametrized case expected failure\")".to_string(),
            );
        }

        if case.marks.iter().any(|mark| mark == "skip") {
            expanded_test
                .decorators
                .push("@pytest.mark.skip(reason=\"Parametrized case skipped\")".to_string());
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
    marks: Vec<String>,
}

fn generate_test_cases(param_info_list: &[ParametrizeInfo]) -> Vec<TestCase> {
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
            marks: set.marks.clone(),
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
                    marks: merge_marks(&case.marks, &set.marks),
                });
            }
        }

        cases = new_cases;
    }

    cases
}

fn merge_marks(left: &[String], right: &[String]) -> Vec<String> {
    let mut marks = left.to_vec();
    for mark in right {
        if !marks.contains(mark) {
            marks.push(mark.clone());
        }
    }
    marks
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
                .map(|c| {
                    if c.is_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect()
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Null => "None".to_string(),
        Value::Array(arr) => arr.iter().map(format_value).collect::<Vec<_>>().join("_"),
        Value::Object(map) => {
            if let Some(Value::String(kind)) = map.get("__fastest_float__") {
                kind.replace('-', "neg_")
            } else if map.contains_key("__fastest_set__") {
                "set".to_string()
            } else {
                "object".to_string()
            }
        }
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
    fn test_parse_range_call_values() {
        let decorator = r#"@pytest.mark.parametrize("number", range(3))"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());

        let (names, param_sets, _ids) = result.unwrap();

        assert_eq!(names, vec!["number"]);
        assert_eq!(param_sets.len(), 3);
        assert_eq!(param_sets[0].values, vec![Value::from(0)]);
        assert_eq!(param_sets[1].values, vec![Value::from(1)]);
        assert_eq!(param_sets[2].values, vec![Value::from(2)]);
    }

    #[test]
    fn test_parse_safe_static_expression_values() {
        let decorator = r#"@pytest.mark.parametrize("value", ["a" * 3, float("inf"), set()])"#;
        let result = parse_parametrize_decorator(decorator);
        assert!(result.is_some());

        let (_names, param_sets, _ids) = result.unwrap();

        assert_eq!(param_sets[0].values, vec![Value::from("aaa")]);
        assert_eq!(
            param_sets[1].values,
            vec![serde_json::json!({ "__fastest_float__": "inf" })]
        );
        assert_eq!(
            param_sets[2].values,
            vec![serde_json::json!({ "__fastest_set__": [] })]
        );
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
            decorators: vec!["pytest.mark.parametrize(\"x\", [1, 2])".to_string()].into(),
            fixture_deps: vec![].into(),
            is_xfail: false,
            indirect_params: HashMap::new(),
        };

        let expanded = expand_parametrized_tests(&test, &test.decorators).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0].id, "test_module::test_func[1]");
        assert_eq!(expanded[1].id, "test_module::test_func[2]");
    }

    #[test]
    fn test_expand_pytest_param_marks() {
        let test = TestItem {
            id: "test_module::test_func".to_string(),
            path: std::path::PathBuf::from("test.py"),
            name: "test_func".to_string(),
            function_name: "test_func".to_string(),
            line_number: Some(1),
            is_async: false,
            class_name: None,
            decorators: vec![
                r#"pytest.mark.parametrize("x", [pytest.param(1, marks=pytest.mark.skip(reason="skip")), pytest.param(2, marks=pytest.mark.xfail)])"#.to_string(),
            ]
            .into(),
            fixture_deps: vec![].into(),
            is_xfail: false,
            indirect_params: HashMap::new(),
        };

        let expanded = expand_parametrized_tests(&test, &test.decorators).unwrap();

        assert_eq!(expanded.len(), 2);
        assert!(expanded[0]
            .decorators
            .iter()
            .any(|decorator| decorator.starts_with("@pytest.mark.skip")));
        assert!(expanded[1]
            .decorators
            .iter()
            .any(|decorator| decorator.starts_with("@pytest.mark.xfail")));
    }
}
