use crate::discovery::TestItem;
use crate::error::Result;
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
    pub id: String,              // Custom ID if provided
    pub values: Vec<Value>,      // Parameter values
    pub marks: Vec<String>,      // Additional marks for this param set
}

/// Parse parametrize decorator and extract parameter information
pub fn parse_parametrize_decorator(decorator: &str) -> Option<(Vec<String>, Vec<Vec<Value>>)> {
    // Remove @pytest.mark.parametrize or @fastest.mark.parametrize prefix and handle multi-line
    let decorator = decorator.replace('\n', " ").replace("  ", " ");
    let content = decorator
        .trim_start_matches('@')
        .trim_start_matches("pytest.mark.parametrize")
        .trim_start_matches("fastest.mark.parametrize")
        .trim_start_matches("mark.parametrize")
        .trim_start_matches("parametrize")
        .trim();

    // Find the opening parenthesis
    if !content.starts_with('(') {
        return None;
    }

    // Extract the content between parentheses
    let content = content.trim_start_matches('(').trim_end_matches(')');
    
    // Find the comma after the parameter names (which are in quotes)
    let mut quote_count = 0;
    let mut split_pos = None;
    for (i, ch) in content.chars().enumerate() {
        match ch {
            '"' | '\'' => quote_count += 1,
            ',' if quote_count % 2 == 0 => {
                split_pos = Some(i);
                break;
            }
            _ => {}
        }
    }
    
    let split_pos = split_pos?;
    let param_names_str = content[..split_pos].trim().trim_matches('"').trim_matches('\'');
    let values_str = content[split_pos + 1..].trim();
    
    // Parse parameter names
    let param_names: Vec<String> = if param_names_str.contains(',') {
        param_names_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        vec![param_names_str.to_string()]
    };

    // Parse parameter values
    let param_values = parse_param_values(values_str)?;

    Some((param_names, param_values))
}

/// Parse the parameter values from the decorator string
fn parse_param_values(values_str: &str) -> Option<Vec<Vec<Value>>> {
    // For now, we'll do a simple parsing that handles basic cases
    // In the future, we might want to use a proper Python AST parser
    
    let values_str = values_str.trim();
    if !values_str.starts_with('[') || !values_str.ends_with(']') {
        return None;
    }

    let inner = &values_str[1..values_str.len()-1];
    let mut param_sets = Vec::new();
    
    // Simple parsing for tuples and single values
    let mut in_tuple = false;
    let mut current_value = String::new();
    let mut paren_depth = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    
    for ch in inner.chars() {
        match ch {
            '"' | '\'' if !in_string => {
                in_string = true;
                string_char = ch;
                current_value.push(ch);
            }
            '"' | '\'' if in_string && ch == string_char => {
                in_string = false;
                current_value.push(ch);
            }
            '(' if !in_string => {
                if paren_depth == 0 {
                    in_tuple = true;
                } else {
                    current_value.push(ch);
                }
                paren_depth += 1;
            }
            ')' if !in_string => {
                paren_depth -= 1;
                if paren_depth == 0 && in_tuple {
                    in_tuple = false;
                    // Parse the accumulated values
                    if let Some(values) = parse_tuple_values(&current_value) {
                        param_sets.push(values);
                    }
                    current_value.clear();
                } else {
                    current_value.push(ch);
                }
            }
            ',' if !in_string && paren_depth == 0 => {
                if !current_value.trim().is_empty() {
                    // Single value, not in a tuple
                    if let Some(value) = parse_single_value(&current_value) {
                        param_sets.push(vec![value]);
                    }
                    current_value.clear();
                }
            }
            _ => {
                current_value.push(ch);
            }
        }
    }
    
    // Handle last value
    if !current_value.trim().is_empty() {
        if let Some(value) = parse_single_value(&current_value) {
            param_sets.push(vec![value]);
        }
    }

    if param_sets.is_empty() {
        None
    } else {
        Some(param_sets)
    }
}

/// Parse values from a tuple string
fn parse_tuple_values(tuple_str: &str) -> Option<Vec<Value>> {
    let values: Vec<Value> = tuple_str
        .split(',')
        .filter_map(|s| parse_single_value(s.trim()))
        .collect();
    
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

/// Parse a single value from string representation
fn parse_single_value(value_str: &str) -> Option<Value> {
    let trimmed = value_str.trim();
    
    // String literal
    if (trimmed.starts_with('"') && trimmed.ends_with('"')) ||
       (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
        let inner = &trimmed[1..trimmed.len()-1];
        return Some(Value::String(inner.to_string()));
    }
    
    // Boolean
    if trimmed == "True" {
        return Some(Value::Bool(true));
    }
    if trimmed == "False" {
        return Some(Value::Bool(false));
    }
    
    // None
    if trimmed == "None" {
        return Some(Value::Null);
    }
    
    // Number
    if let Ok(num) = trimmed.parse::<i64>() {
        return Some(Value::Number(serde_json::Number::from(num)));
    }
    if let Ok(num) = trimmed.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(num) {
            return Some(Value::Number(n));
        }
    }
    
    // Fallback to string
    Some(Value::String(trimmed.to_string()))
}

/// Expand a test function with parametrize decorators into multiple test items
pub fn expand_parametrized_tests(
    test: &TestItem,
    decorators: &[String],
) -> Result<Vec<TestItem>> {
    let mut expanded_tests = Vec::new();
    let mut param_info: Vec<(Vec<String>, Vec<Vec<Value>>)> = Vec::new();
    
    // Collect all parametrize decorators
    for decorator in decorators {
        if decorator.contains("parametrize") {
            if let Some(info) = parse_parametrize_decorator(decorator) {
                param_info.push(info);
            }
        }
    }
    
    if param_info.is_empty() {
        // Not a parametrized test, return as-is
        return Ok(vec![test.clone()]);
    }
    
    // Generate test cases
    let test_cases = generate_test_cases(&param_info);
    
    for (_idx, case) in test_cases.iter().enumerate() {
        let mut expanded_test = test.clone();
        
        // Create unique ID for this test case
        let param_id = format_param_id(&case.params);
        expanded_test.id = format!("{}[{}]", test.id, param_id);
        expanded_test.name = format!("{}[{}]", test.function_name, param_id);
        
        // Store parameter info in decorators (for now)
        // In the future, we might want to add a proper field for this
        expanded_test.decorators.push(format!("__params__={}", serde_json::to_string(&case.params).unwrap_or_default()));
        
        expanded_tests.push(expanded_test);
    }
    
    Ok(expanded_tests)
}

#[derive(Debug)]
struct TestCase {
    params: HashMap<String, Value>,
}

/// Generate all test cases from multiple parametrize decorators
fn generate_test_cases(param_info: &[(Vec<String>, Vec<Vec<Value>>)]) -> Vec<TestCase> {
    if param_info.is_empty() {
        return vec![];
    }
    
    // Start with the first parametrize decorator
    let (first_names, first_values) = &param_info[0];
    let mut cases: Vec<TestCase> = first_values
        .iter()
        .map(|values| {
            let mut params = HashMap::new();
            for (i, name) in first_names.iter().enumerate() {
                if let Some(value) = values.get(i) {
                    params.insert(name.clone(), value.clone());
                }
            }
            TestCase { params }
        })
        .collect();
    
    // Apply remaining parametrize decorators (cartesian product)
    for (names, value_sets) in param_info.iter().skip(1) {
        let mut new_cases = Vec::new();
        
        for existing_case in &cases {
            for values in value_sets {
                let mut new_params = existing_case.params.clone();
                for (i, name) in names.iter().enumerate() {
                    if let Some(value) = values.get(i) {
                        new_params.insert(name.clone(), value.clone());
                    }
                }
                new_cases.push(TestCase { params: new_params });
            }
        }
        
        cases = new_cases;
    }
    
    cases
}

/// Format parameter ID for test naming
fn format_param_id(params: &HashMap<String, Value>) -> String {
    let mut parts = Vec::new();
    let mut keys: Vec<_> = params.keys().collect();
    keys.sort(); // Ensure consistent ordering
    
    for key in keys {
        if let Some(value) = params.get(key) {
            let formatted = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => if *b { "True".to_string() } else { "False".to_string() },
                Value::Null => "None".to_string(),
                _ => value.to_string(),
            };
            parts.push(formatted);
        }
    }
    
    parts.join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_parametrize() {
        let decorator = r#"@pytest.mark.parametrize("x", [1, 2, 3])"#;
        let result = parse_parametrize_decorator(decorator);
        
        assert!(result.is_some());
        let (names, values) = result.unwrap();
        assert_eq!(names, vec!["x"]);
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_parse_tuple_parametrize() {
        let decorator = r#"@pytest.mark.parametrize("x,y,expected", [(2, 3, 5), (4, 5, 9)])"#;
        let result = parse_parametrize_decorator(decorator);
        
        assert!(result.is_some());
        let (names, values) = result.unwrap();
        assert_eq!(names, vec!["x", "y", "expected"]);
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].len(), 3);
    }

    #[test]
    fn test_parse_string_parametrize() {
        let decorator = r#"@pytest.mark.parametrize("word", ["hello", "world"])"#;
        let result = parse_parametrize_decorator(decorator);
        
        assert!(result.is_some());
        let (names, values) = result.unwrap();
        assert_eq!(names, vec!["word"]);
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_parse_multiline_parametrize() {
        let decorator = r#"pytest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (4, 5, 9),
    (10, -5, 5),
])"#;
        let result = parse_parametrize_decorator(decorator);
        
        assert!(result.is_some());
        let (names, values) = result.unwrap();
        assert_eq!(names, vec!["x", "y", "expected"]);
        assert_eq!(values.len(), 3);
        assert_eq!(values[0].len(), 3);
        
        // Check first parameter set
        assert_eq!(values[0][0], Value::Number(serde_json::Number::from(2)));
        assert_eq!(values[0][1], Value::Number(serde_json::Number::from(3)));
        assert_eq!(values[0][2], Value::Number(serde_json::Number::from(5)));
    }

    #[test]
    fn test_parse_fastest_parametrize() {
        // Test with fastest prefix
        let decorator = r#"@fastest.mark.parametrize("x", [1, 2, 3])"#;
        let result = parse_parametrize_decorator(decorator);
        
        assert!(result.is_some());
        let (names, values) = result.unwrap();
        assert_eq!(names, vec!["x"]);
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_parse_both_prefixes() {
        // Test that both pytest and fastest prefixes work
        let decorators = vec![
            r#"@pytest.mark.parametrize("x,y", [(1, 2), (3, 4)])"#,
            r#"@fastest.mark.parametrize("x,y", [(1, 2), (3, 4)])"#,
            r#"@mark.parametrize("x,y", [(1, 2), (3, 4)])"#,
            r#"parametrize("x,y", [(1, 2), (3, 4)])"#,
        ];
        
        for decorator in decorators {
            let result = parse_parametrize_decorator(decorator);
            assert!(result.is_some(), "Failed to parse: {}", decorator);
            let (names, values) = result.unwrap();
            assert_eq!(names, vec!["x", "y"]);
            assert_eq!(values.len(), 2);
        }
    }
} 