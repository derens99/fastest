use colored::*;
use similar::{ChangeTag, TextDiff};
use std::fmt::Display;
use anyhow::Result;
use regex::Regex;
use crate::TestResult;

/// Format assertion failures with better diffs
pub fn format_assertion_error(error: &str) -> String {
    // Check if this is an assertion error
    if error.contains("AssertionError") {
        if let Some(enhanced) = try_enhance_assertion(error) {
            return enhanced;
        }
    }

    // Return original error if we can't enhance it
    error.to_string()
}

/// Try to enhance an assertion error with better formatting
fn try_enhance_assertion(error: &str) -> Option<String> {
    // Look for common assertion patterns
    if error.contains("assert") && error.contains("==") {
        return enhance_equality_assertion(error);
    }

    if error.contains("assert") && error.contains("in") {
        return enhance_membership_assertion(error);
    }

    None
}

/// Enhance equality assertions
fn enhance_equality_assertion(error: &str) -> Option<String> {
    // Try to extract the values being compared
    // This is a simplified version - a full implementation would parse Python AST

    let mut result = String::new();
    result.push_str(
        &"AssertionError: Equality assertion failed"
            .red()
            .to_string(),
    );
    result.push_str("\n\n");

    // If we can extract values, show a diff
    if let Some((left, right)) = extract_comparison_values(error) {
        result.push_str(&format!("Expected: {}\n", left.green()));
        result.push_str(&format!("Actual:   {}\n", right.red()));

        // Show diff for strings
        if left.contains('\n') || right.contains('\n') {
            result.push_str("\nDifference:\n");
            result.push_str(&create_diff(&left, &right));
        }
    }

    Some(result)
}

/// Enhance membership assertions
fn enhance_membership_assertion(error: &str) -> Option<String> {
    let mut result = String::new();
    result.push_str(
        &"AssertionError: Membership assertion failed"
            .red()
            .to_string(),
    );
    result.push_str("\n\n");

    // Add contextual information
    result.push_str("The item was not found in the container\n");

    Some(result)
}

/// Extract values from comparison (simplified)
fn extract_comparison_values(error: &str) -> Option<(String, String)> {
    // TODO: Extract actual and expected values from error message
    // For now, return None
    None
}

/// Create a colored diff between two strings
pub fn create_diff(left: &str, right: &str) -> String {
    let diff = TextDiff::from_lines(left, right);
    let mut result = String::new();

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            result.push_str(&"...\n".dimmed().to_string());
        }

        for op in group {
            for change in diff.iter_changes(op) {
                let (sign, style) = match change.tag() {
                    ChangeTag::Delete => ("-", "red"),
                    ChangeTag::Insert => ("+", "green"),
                    ChangeTag::Equal => (" ", "dimmed"),
                };

                result.push_str(&format!(
                    "{} {}",
                    sign.color(style),
                    change.to_string().trim_end()
                ));

                if change.missing_newline() {
                    result.push('\n');
                }
            }
        }
    }

    result
}

/// Helper to format Python objects nicely
pub fn format_python_value(value: &str) -> String {
    // Pretty format Python values
    if value.starts_with('{') || value.starts_with('[') {
        // Try to format as JSON for better readability
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value) {
            if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                return pretty;
            }
        }
    }

    value.to_string()
}

/// Generate assertion introspection code
pub fn generate_assertion_wrapper() -> &'static str {
    r#"
# Enhanced assertion wrapper
import sys
import traceback
import json

def format_assertion_error(exc_info):
    exc_type, exc_value, exc_tb = exc_info
    
    if exc_type.__name__ != 'AssertionError':
        return None
    
    # Extract the assertion line
    tb_frame = traceback.extract_tb(exc_tb)[-1]
    filename = tb_frame.filename
    line_num = tb_frame.lineno
    line = tb_frame.line
    
    # Try to extract values if possible
    frame = exc_tb.tb_frame
    while frame.f_back:
        frame = frame.f_back
        if 'assert' in frame.f_code.co_name:
            break
    
    locals_dict = frame.f_locals
    
    # Build enhanced error message
    error_data = {
        'type': 'assertion',
        'file': filename,
        'line': line_num,
        'code': line,
        'locals': {k: repr(v) for k, v in locals_dict.items() 
                   if not k.startswith('__')},
        'message': str(exc_value) if str(exc_value) else 'Assertion failed'
    }
    
    return error_data

# Monkey-patch the test runner to use our enhanced assertions
_original_excepthook = sys.excepthook

def enhanced_excepthook(exc_type, exc_value, exc_tb):
    error_data = format_assertion_error((exc_type, exc_value, exc_tb))
    if error_data:
        print(f"ENHANCED_ASSERTION:{json.dumps(error_data)}")
    else:
        _original_excepthook(exc_type, exc_value, exc_tb)

sys.excepthook = enhanced_excepthook
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_creation() {
        let left = "hello\nworld\nfoo";
        let right = "hello\nplanet\nfoo";

        let diff = create_diff(left, right);
        assert!(diff.contains("world"));
        assert!(diff.contains("planet"));
    }

    #[test]
    fn test_format_python_value() {
        let formatted = format_python_value("{\"key\": \"value\"}");
        assert!(formatted.contains("key"));
    }
}
