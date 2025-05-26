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

        // For now, just extract the name
        // TODO: Parse arguments and kwargs
        if let Some(paren_pos) = marker_str.find('(') {
            let name = marker_str[..paren_pos].to_string();
            // TODO: Parse args inside parentheses
            Ok(Self::new(name))
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
                    return Some(
                        marker
                            .kwargs
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Skipped")
                            .to_string(),
                    );
                }
                "skipif" => {
                    // TODO: Evaluate condition
                    // For now, always skip if marker is present
                    return Some("Conditional skip".to_string());
                }
                _ => {}
            }
        }
        None
    }

    /// Check if test is expected to fail
    pub fn is_xfail(markers: &[Marker]) -> bool {
        markers.iter().any(|m| m.name == "xfail")
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
        if expr.starts_with("not ") {
            let rest = expr[4..].trim();
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
