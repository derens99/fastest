use anyhow::Result;
use std::collections::HashMap;

/// Represents a test marker (like @pytest.mark.skip or @fastest.mark.skip)
#[derive(Debug, Clone)]
pub struct Marker {
    pub name: String,
    pub args: Vec<serde_json::Value>,
    pub kwargs: HashMap<String, serde_json::Value>,
}

impl Marker {
    pub fn new(name: String) -> Self {
        Self {
            name,
            args: Vec::new(),
            kwargs: HashMap::new(),
        }
    }

    /// Parse a decorator string into a Marker
    pub fn from_decorator(decorator: &str) -> Result<Self> {
        // Remove pytest.mark. or fastest.mark. prefix if present
        let marker_str = decorator
            .strip_prefix("pytest.mark.")
            .or_else(|| decorator.strip_prefix("fastest.mark."))
            .or_else(|| decorator.strip_prefix("mark."))
            .unwrap_or(decorator);

        // Parse name and arguments
        if let Some(paren_pos) = marker_str.find('(') {
            let name = marker_str[..paren_pos].to_string();
            let args_str = &marker_str[paren_pos..];
            
            // Remove outer parentheses
            let args_str = args_str.trim_start_matches('(').trim_end_matches(')');
            
            let mut marker = Self::new(name);
            
            // Parse arguments - handle common cases
            if !args_str.is_empty() {
                // Split by comma but respect nested parentheses and quotes
                let parts = split_args(args_str);
                
                for part in parts {
                    let part = part.trim();
                    // Check if it's a kwarg (contains =)
                    if let Some(eq_pos) = part.find('=') {
                        let key = part[..eq_pos].trim().trim_matches('"').to_string();
                        let value_str = part[eq_pos + 1..].trim();
                        let value = parse_value(value_str);
                        marker.kwargs.insert(key, value);
                    } else {
                        // It's a positional argument
                        let value = parse_value(part);
                        marker.args.push(value);
                    }
                }
            }
            
            Ok(marker)
        } else {
            Ok(Self::new(marker_str.to_string()))
        }
    }
}

/// Common pytest markers
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinMarker {
    Skip(Option<String>),                        // reason
    Skipif(String, Option<String>),              // condition, reason
    Xfail(Option<String>),                       // reason
    Parametrize(String, Vec<serde_json::Value>), // argnames, argvalues
    Timeout(f64),                                // seconds
    Slow,
    Asyncio,
}

impl BuiltinMarker {
    /// Check if a test should be skipped based on its markers
    pub fn should_skip(markers: &[Marker]) -> Option<String> {
        for marker in markers {
            match marker.name.as_str() {
                "skip" => {
                    // Check for reason in kwargs first, then args
                    let reason = marker
                        .kwargs
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .or_else(|| marker.args.get(0).and_then(|v| v.as_str()))
                        .unwrap_or("Skipped")
                        .to_string();
                    return Some(reason);
                }
                "skipif" => {
                    // For skipif, we need to evaluate the condition
                    // For now, check if first arg is a string that looks like a condition
                    if let Some(condition) = marker.args.get(0).and_then(|v| v.as_str()) {
                        // TODO: Properly evaluate Python condition
                        // For now, handle some common cases
                        if should_skip_condition(condition) {
                            let reason = marker
                                .kwargs
                                .get("reason")
                                .and_then(|v| v.as_str())
                                .or_else(|| marker.args.get(1).and_then(|v| v.as_str()))
                                .unwrap_or("Conditional skip")
                                .to_string();
                            return Some(reason);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Check if test is expected to fail
    pub fn is_xfail(markers: &[Marker]) -> Option<String> {
        for marker in markers {
            if marker.name == "xfail" {
                // Extract reason from kwargs or args
                let reason = marker
                    .kwargs
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .or_else(|| marker.args.get(0).and_then(|v| v.as_str()))
                    .map(|s| s.to_string());
                return Some(reason.unwrap_or_else(|| "Expected to fail".to_string()));
            }
        }
        None
    }
}

/// Extract markers from test decorators
pub fn extract_markers(decorators: &[String]) -> Vec<Marker> {
    decorators
        .iter()
        .filter(|d| {
            d.contains("mark.")
                || d.contains("@mark")
                || d.contains("pytest.mark")
                || d.contains("fastest.mark")
        })
        .filter_map(|d| Marker::from_decorator(d).ok())
        .collect()
}

/// Marker expression for filtering tests
#[derive(Debug, Clone)]
pub enum MarkerExpr {
    Name(String),
    Not(Box<MarkerExpr>),
    And(Box<MarkerExpr>, Box<MarkerExpr>),
    Or(Box<MarkerExpr>, Box<MarkerExpr>),
}

impl MarkerExpr {
    /// Parse a marker expression string
    pub fn parse(expr: &str) -> Result<Self> {
        let expr = expr.trim();

        // Handle "not" expressions
        if let Some(stripped) = expr.strip_prefix("not ") {
            let rest = stripped.trim();
            return Ok(MarkerExpr::Not(Box::new(Self::parse(rest)?)));
        }

        // Handle "and" expressions
        if let Some(and_pos) = expr.find(" and ") {
            let left = &expr[..and_pos];
            let right = &expr[and_pos + 5..];
            return Ok(MarkerExpr::And(
                Box::new(Self::parse(left)?),
                Box::new(Self::parse(right)?),
            ));
        }

        // Handle "or" expressions
        if let Some(or_pos) = expr.find(" or ") {
            let left = &expr[..or_pos];
            let right = &expr[or_pos + 4..];
            return Ok(MarkerExpr::Or(
                Box::new(Self::parse(left)?),
                Box::new(Self::parse(right)?),
            ));
        }

        // Handle parentheses
        if expr.starts_with('(') && expr.ends_with(')') {
            return Self::parse(&expr[1..expr.len() - 1]);
        }

        // Simple marker name
        Ok(MarkerExpr::Name(expr.to_string()))
    }

    /// Evaluate expression against a list of markers
    pub fn evaluate(&self, markers: &[Marker]) -> bool {
        match self {
            MarkerExpr::Name(name) => markers.iter().any(|m| m.name == *name),
            MarkerExpr::Not(expr) => !expr.evaluate(markers),
            MarkerExpr::And(left, right) => left.evaluate(markers) && right.evaluate(markers),
            MarkerExpr::Or(left, right) => left.evaluate(markers) || right.evaluate(markers),
        }
    }
}

/// Filter tests based on marker expressions
pub fn filter_by_markers(
    tests: Vec<crate::TestItem>,
    expression: &str,
) -> Result<Vec<crate::TestItem>> {
    let expr = MarkerExpr::parse(expression)?;

    Ok(tests
        .into_iter()
        .filter(|test| {
            // Extract markers from test decorators
            let markers = extract_markers(&test.decorators);

            // Special case: tests without any markers
            if markers.is_empty() && expression.starts_with("not ") {
                // "not X" should include unmarked tests
                return true;
            }

            expr.evaluate(&markers)
        })
        .collect())
}

/// Split arguments respecting quotes and parentheses
fn split_args(args: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';
    let mut paren_depth = 0;
    let mut chars = args.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '"' | '\'' if paren_depth == 0 => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = ch;
                } else if ch == quote_char {
                    in_quotes = false;
                }
                current.push(ch);
            }
            '(' if !in_quotes => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' if !in_quotes => {
                paren_depth -= 1;
                current.push(ch);
            }
            ',' if !in_quotes && paren_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    
    if !current.is_empty() {
        parts.push(current.trim().to_string());
    }
    
    parts
}

/// Parse a value string into a JSON value
fn parse_value(value: &str) -> serde_json::Value {
    let trimmed = value.trim();
    
    // String values (quoted)
    if (trimmed.starts_with('"') && trimmed.ends_with('"')) ||
       (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
        let unquoted = &trimmed[1..trimmed.len()-1];
        return serde_json::Value::String(unquoted.to_string());
    }
    
    // Boolean values
    if trimmed == "True" || trimmed == "true" {
        return serde_json::Value::Bool(true);
    }
    if trimmed == "False" || trimmed == "false" {
        return serde_json::Value::Bool(false);
    }
    
    // None/null
    if trimmed == "None" || trimmed == "null" {
        return serde_json::Value::Null;
    }
    
    // Try to parse as number
    if let Ok(num) = trimmed.parse::<i64>() {
        return serde_json::Value::Number(num.into());
    }
    if let Ok(num) = trimmed.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(num) {
            return serde_json::Value::Number(n);
        }
    }
    
    // Otherwise treat as string
    serde_json::Value::String(trimmed.to_string())
}

/// Simple condition evaluation for skipif
/// TODO: Implement proper Python expression evaluation
fn should_skip_condition(condition: &str) -> bool {
    let condition = condition.trim();
    
    // Handle some common conditions
    match condition {
        "True" | "true" | "1" => true,
        "False" | "false" | "0" | "" => false,
        _ => {
            // Handle sys.platform checks
            if condition.contains("sys.platform") {
                #[cfg(target_os = "windows")]
                {
                    return condition.contains("win32") || condition.contains("windows");
                }
                #[cfg(target_os = "macos")]
                {
                    return condition.contains("darwin") || condition.contains("macos");
                }
                #[cfg(target_os = "linux")]
                {
                    return condition.contains("linux");
                }
            }
            
            // Handle sys.version_info checks
            if condition.contains("sys.version_info") {
                // For now, assume Python 3.7+ so skip tests for older versions
                if condition.contains("< (3,") {
                    return false;
                }
                if condition.contains(">= (3,") {
                    return true;
                }
            }
            
            // By default, don't skip
            false
        }
    }
}
