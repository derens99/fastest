//! Advanced Assertion Rewriting System
//!
//! This module provides sophisticated assertion rewriting capabilities that transform
//! Python assert statements into more informative versions that provide better error
//! messages and detailed diff output when assertions fail.
//!
//! Features:
//! - AST-based assertion rewriting
//! - Rich comparison output (strings, lists, dicts, objects)
//! - Context-aware error messages
//! - Integration with pytest assertion introspection
//! - Custom assertion helpers

use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::fmt::Write;

/// Configuration for assertion rewriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionConfig {
    pub enabled: bool,
    pub detailed_diffs: bool,
    pub max_diff_lines: usize,
    pub show_locals: bool,
    pub rewrite_assert_messages: bool,
}

impl Default for AssertionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detailed_diffs: true,
            max_diff_lines: 100,
            show_locals: false,
            rewrite_assert_messages: true,
        }
    }
}

/// Types of assertions that can be rewritten
#[derive(Debug, Clone, PartialEq)]
pub enum AssertionType {
    Compare {
        op: CompareOp,
        left: String,
        right: String,
    },
    Contains {
        item: String,
        container: String,
    },
    IsInstance {
        obj: String,
        type_name: String,
    },
    StartsWith {
        string: String,
        prefix: String,
    },
    EndsWith {
        string: String,
        suffix: String,
    },
    In {
        item: String,
        container: String,
    },
    Is {
        left: String,
        right: String,
    },
    Boolean {
        expr: String,
    },
    Custom {
        expr: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Eq,    // ==
    NotEq, // !=
    Lt,    // <
    LtE,   // <=
    Gt,    // >
    GtE,   // >=
}

/// Represents a rewritten assertion with enhanced error reporting
#[derive(Debug, Clone)]
pub struct RewrittenAssertion {
    pub original_line: String,
    pub rewritten_code: String,
    pub assertion_type: AssertionType,
    pub line_number: usize,
}

/// Main assertion rewriter
pub struct AssertionRewriter {
    config: AssertionConfig,
    helpers: AssertionHelpers,
}

impl AssertionRewriter {
    pub fn new(config: AssertionConfig) -> Self {
        Self {
            config,
            helpers: AssertionHelpers::new(),
        }
    }

    /// Rewrite assertions in Python source code
    pub fn rewrite_source(&self, source: &str, file_path: &str) -> Result<String> {
        if !self.config.enabled {
            return Ok(source.to_string());
        }

        let lines: Vec<&str> = source.lines().collect();
        let mut rewritten_lines = Vec::new();
        let mut in_multiline_assert = false;
        let mut current_assert = String::new();
        let mut assert_start_line = 0;

        for (line_no, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check if this line starts or continues an assert statement
            if trimmed.starts_with("assert ") || in_multiline_assert {
                if !in_multiline_assert {
                    // Starting a new assert
                    in_multiline_assert = true;
                    current_assert.clear();
                    assert_start_line = line_no;
                }

                current_assert.push_str(line);
                current_assert.push('\n');

                // Check if the assert statement is complete
                if self.is_assert_complete(&current_assert) {
                    // Rewrite the complete assert statement
                    match self.rewrite_assert_statement(
                        &current_assert,
                        assert_start_line + 1,
                        file_path,
                    ) {
                        Ok(rewritten) => {
                            // Replace the original lines with rewritten code
                            let rewritten_lines_vec: Vec<&str> = rewritten.lines().collect();
                            for rewritten_line in rewritten_lines_vec {
                                rewritten_lines.push(rewritten_line.to_string());
                            }
                        }
                        Err(_) => {
                            // If rewriting fails, keep original
                            for original_line in current_assert.lines() {
                                if !original_line.trim().is_empty() {
                                    rewritten_lines.push(original_line.to_string());
                                }
                            }
                        }
                    }
                    in_multiline_assert = false;
                    current_assert.clear();
                } else if line_no == lines.len() - 1 {
                    // End of file, keep original incomplete assert
                    for original_line in current_assert.lines() {
                        if !original_line.trim().is_empty() {
                            rewritten_lines.push(original_line.to_string());
                        }
                    }
                    in_multiline_assert = false;
                }
            } else if !in_multiline_assert {
                // Regular line, keep as-is
                rewritten_lines.push(line.to_string());
            }
        }

        Ok(rewritten_lines.join("\n"))
    }

    /// Check if an assert statement is syntactically complete
    fn is_assert_complete(&self, assert_text: &str) -> bool {
        // Simple heuristic: count parentheses, brackets, and braces
        let mut paren_count = 0;
        let mut bracket_count = 0;
        let mut brace_count = 0;
        let mut in_string = false;
        let mut string_char = '"';
        let mut escaped = false;

        for ch in assert_text.chars() {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' if in_string => escaped = true,
                '"' | '\'' if !in_string => {
                    in_string = true;
                    string_char = ch;
                }
                ch if in_string && ch == string_char => in_string = false,
                '(' if !in_string => paren_count += 1,
                ')' if !in_string => paren_count -= 1,
                '[' if !in_string => bracket_count += 1,
                ']' if !in_string => bracket_count -= 1,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => brace_count -= 1,
                _ => {}
            }
        }

        // Statement is complete if all brackets are balanced and it doesn't end with a continuation
        let balanced = paren_count == 0 && bracket_count == 0 && brace_count == 0;
        let not_continued = !assert_text.trim_end().ends_with('\\');

        balanced && not_continued && !in_string
    }

    /// Rewrite a single assert statement
    fn rewrite_assert_statement(
        &self,
        assert_stmt: &str,
        line_number: usize,
        file_path: &str,
    ) -> Result<String> {
        let assertion_type = self.analyze_assertion(assert_stmt)?;
        let rewritten_code =
            self.generate_rewritten_assertion(&assertion_type, line_number, file_path)?;

        Ok(rewritten_code)
    }

    /// Analyze an assert statement to determine its type
    fn analyze_assertion(&self, assert_stmt: &str) -> Result<AssertionType> {
        let cleaned = assert_stmt.replace('\n', " ").replace("assert ", "");
        let parts: Vec<&str> = cleaned.split(',').collect();
        let main_expr = parts[0].trim();

        // Try to parse comparison expressions
        if let Some(assertion_type) = self.parse_comparison(main_expr) {
            return Ok(assertion_type);
        }

        // Try to parse method calls
        if let Some(assertion_type) = self.parse_method_call(main_expr) {
            return Ok(assertion_type);
        }

        // Try to parse membership tests
        if let Some(assertion_type) = self.parse_membership(main_expr) {
            return Ok(assertion_type);
        }

        // Try to parse identity tests
        if let Some(assertion_type) = self.parse_identity(main_expr) {
            return Ok(assertion_type);
        }

        // Fallback to custom expression
        Ok(AssertionType::Custom {
            expr: main_expr.to_string(),
        })
    }

    fn parse_comparison(&self, expr: &str) -> Option<AssertionType> {
        let operators = [
            ("==", CompareOp::Eq),
            ("!=", CompareOp::NotEq),
            ("<=", CompareOp::LtE),
            (">=", CompareOp::GtE),
            ("<", CompareOp::Lt),
            (">", CompareOp::Gt),
        ];

        for (op_str, op) in &operators {
            if let Some(pos) = expr.find(op_str) {
                let left = expr[..pos].trim().to_string();
                let right = expr[pos + op_str.len()..].trim().to_string();

                return Some(AssertionType::Compare {
                    op: op.clone(),
                    left,
                    right,
                });
            }
        }

        None
    }

    fn parse_method_call(&self, expr: &str) -> Option<AssertionType> {
        if let Some(pos) = expr.find(".startswith(") {
            let string = expr[..pos].trim().to_string();
            let rest = &expr[pos + 12..]; // ".startswith(".len()
            if let Some(end_pos) = rest.find(')') {
                let prefix = rest[..end_pos].trim().to_string();
                return Some(AssertionType::StartsWith { string, prefix });
            }
        }

        if let Some(pos) = expr.find(".endswith(") {
            let string = expr[..pos].trim().to_string();
            let rest = &expr[pos + 10..]; // ".endswith(".len()
            if let Some(end_pos) = rest.find(')') {
                let suffix = rest[..end_pos].trim().to_string();
                return Some(AssertionType::EndsWith { string, suffix });
            }
        }

        if expr.contains("isinstance(") {
            // Parse isinstance(obj, type)
            if let Some(start) = expr.find('(') {
                if let Some(end) = expr.rfind(')') {
                    let args = &expr[start + 1..end];
                    let parts: Vec<&str> = args.split(',').collect();
                    if parts.len() == 2 {
                        let obj = parts[0].trim().to_string();
                        let type_name = parts[1].trim().to_string();
                        return Some(AssertionType::IsInstance { obj, type_name });
                    }
                }
            }
        }

        None
    }

    fn parse_membership(&self, expr: &str) -> Option<AssertionType> {
        if expr.contains(" in ") {
            let parts: Vec<&str> = expr.split(" in ").collect();
            if parts.len() == 2 {
                let item = parts[0].trim().to_string();
                let container = parts[1].trim().to_string();
                return Some(AssertionType::In { item, container });
            }
        }

        None
    }

    fn parse_identity(&self, expr: &str) -> Option<AssertionType> {
        if expr.contains(" is ") {
            let parts: Vec<&str> = expr.split(" is ").collect();
            if parts.len() == 2 {
                let left = parts[0].trim().to_string();
                let right = parts[1].trim().to_string();
                return Some(AssertionType::Is { left, right });
            }
        }

        None
    }

    /// Generate rewritten assertion code
    fn generate_rewritten_assertion(
        &self,
        assertion_type: &AssertionType,
        line_number: usize,
        file_path: &str,
    ) -> Result<String> {
        let mut code = String::new();

        // Add helper import
        writeln!(code, "from fastest_assertions import assert_helper")?;

        match assertion_type {
            AssertionType::Compare { op, left, right } => {
                let op_str = match op {
                    CompareOp::Eq => "==",
                    CompareOp::NotEq => "!=",
                    CompareOp::Lt => "<",
                    CompareOp::LtE => "<=",
                    CompareOp::Gt => ">",
                    CompareOp::GtE => ">=",
                };

                writeln!(
                    code,
                    "assert_helper.compare({}, {}, '{}', {}, '{}', '{}')",
                    left,
                    right,
                    op_str,
                    line_number,
                    file_path,
                    format!("{} {} {}", left, op_str, right)
                )?;
            }

            AssertionType::Contains { item, container } => {
                writeln!(
                    code,
                    "assert_helper.contains({}, {}, {}, '{}', '{} in {}')",
                    item, container, line_number, file_path, item, container
                )?;
            }

            AssertionType::IsInstance { obj, type_name } => {
                writeln!(
                    code,
                    "assert_helper.isinstance({}, {}, {}, '{}', 'isinstance({}, {})')",
                    obj, type_name, line_number, file_path, obj, type_name
                )?;
            }

            AssertionType::StartsWith { string, prefix } => {
                writeln!(
                    code,
                    "assert_helper.startswith({}, {}, {}, '{}', '{}.startswith({})')",
                    string, prefix, line_number, file_path, string, prefix
                )?;
            }

            AssertionType::EndsWith { string, suffix } => {
                writeln!(
                    code,
                    "assert_helper.endswith({}, {}, {}, '{}', '{}.endswith({})')",
                    string, suffix, line_number, file_path, string, suffix
                )?;
            }

            AssertionType::In { item, container } => {
                writeln!(
                    code,
                    "assert_helper.membership({}, {}, {}, '{}', '{} in {}')",
                    item, container, line_number, file_path, item, container
                )?;
            }

            AssertionType::Is { left, right } => {
                writeln!(
                    code,
                    "assert_helper.identity({}, {}, {}, '{}', '{} is {}')",
                    left, right, line_number, file_path, left, right
                )?;
            }

            AssertionType::Boolean { expr } => {
                writeln!(
                    code,
                    "assert_helper.boolean({}, {}, '{}', '{}')",
                    expr, line_number, file_path, expr
                )?;
            }

            AssertionType::Custom { expr } => {
                writeln!(
                    code,
                    "assert_helper.custom({}, {}, '{}', '{}')",
                    expr, line_number, file_path, expr
                )?;
            }
        }

        Ok(code)
    }
}

/// Helper functions for enhanced assertions
pub struct AssertionHelpers {
    pub diff_threshold: usize,
}

impl AssertionHelpers {
    pub fn new() -> Self {
        Self {
            diff_threshold: 80, // Number of characters before showing diff
        }
    }

    /// Generate Python code for assertion helpers
    pub fn generate_helper_code(&self) -> String {
        format!(
            r#"
"""
Fastest assertion helpers for enhanced error reporting.
"""

import sys
import difflib
import pprint
from typing import Any, Optional


class AssertionHelper:
    """Enhanced assertion helper with detailed error reporting."""
    
    def __init__(self, diff_threshold: int = {}):
        self.diff_threshold = diff_threshold
    
    def compare(self, left: Any, right: Any, op: str, line_number: int, file_path: str, original_expr: str):
        """Handle comparison assertions with detailed diff output."""
        try:
            if op == "==":
                result = left == right
            elif op == "!=":
                result = left != right
            elif op == "<":
                result = left < right
            elif op == "<=":
                result = left <= right
            elif op == ">":
                result = left > right
            elif op == ">=":
                result = left >= right
            else:
                result = False
            
            if not result:
                self._raise_comparison_error(left, right, op, line_number, file_path, original_expr)
                
        except Exception as e:
            self._raise_assertion_error(f"Comparison failed: {{e}}", line_number, file_path, original_expr)
    
    def contains(self, item: Any, container: Any, line_number: int, file_path: str, original_expr: str):
        """Handle containment assertions."""
        try:
            if item not in container:
                self._raise_assertion_error(
                    f"{{repr(item)}} not found in {{type(container).__name__}} of length {{len(container) if hasattr(container, '__len__') else 'unknown'}}",
                    line_number, file_path, original_expr
                )
        except Exception as e:
            self._raise_assertion_error(f"Containment check failed: {{e}}", line_number, file_path, original_expr)
    
    def isinstance(self, obj: Any, type_class: type, line_number: int, file_path: str, original_expr: str):
        """Handle isinstance assertions."""
        if not isinstance(obj, type_class):
            self._raise_assertion_error(
                f"{{repr(obj)}} is not an instance of {{type_class.__name__}}, got {{type(obj).__name__}}",
                line_number, file_path, original_expr
            )
    
    def startswith(self, string: str, prefix: str, line_number: int, file_path: str, original_expr: str):
        """Handle startswith assertions."""
        if not string.startswith(prefix):
            self._raise_assertion_error(
                f"{{repr(string)}} does not start with {{repr(prefix)}}",
                line_number, file_path, original_expr
            )
    
    def endswith(self, string: str, suffix: str, line_number: int, file_path: str, original_expr: str):
        """Handle endswith assertions."""
        if not string.endswith(suffix):
            self._raise_assertion_error(
                f"{{repr(string)}} does not end with {{repr(suffix)}}",
                line_number, file_path, original_expr
            )
    
    def membership(self, item: Any, container: Any, line_number: int, file_path: str, original_expr: str):
        """Handle membership tests (in operator)."""
        if item not in container:
            self._raise_assertion_error(
                f"{{repr(item)}} not found in {{type(container).__name__}}",
                line_number, file_path, original_expr
            )
    
    def identity(self, left: Any, right: Any, line_number: int, file_path: str, original_expr: str):
        """Handle identity tests (is operator)."""
        if left is not right:
            self._raise_assertion_error(
                f"{{repr(left)}} is not {{repr(right)}}",
                line_number, file_path, original_expr
            )
    
    def boolean(self, expr: Any, line_number: int, file_path: str, original_expr: str):
        """Handle boolean assertions."""
        if not expr:
            self._raise_assertion_error(
                f"Expression evaluated to falsy value: {{repr(expr)}}",
                line_number, file_path, original_expr
            )
    
    def custom(self, expr: Any, line_number: int, file_path: str, original_expr: str):
        """Handle custom expressions."""
        if not expr:
            self._raise_assertion_error(
                f"Assertion failed: {{original_expr}}",
                line_number, file_path, original_expr
            )
    
    def _raise_comparison_error(self, left: Any, right: Any, op: str, line_number: int, file_path: str, original_expr: str):
        """Raise a detailed comparison error."""
        left_repr = repr(left)
        right_repr = repr(right)
        
        # Generate detailed diff for strings and sequences
        diff_output = ""
        if op in ["==", "!="] and isinstance(left, str) and isinstance(right, str):
            if len(left_repr) > self.diff_threshold or len(right_repr) > self.diff_threshold:
                diff_output = self._generate_string_diff(left, right)
        elif op in ["==", "!="] and hasattr(left, '__iter__') and hasattr(right, '__iter__'):
            try:
                if len(str(left)) > self.diff_threshold or len(str(right)) > self.diff_threshold:
                    diff_output = self._generate_sequence_diff(left, right)
            except:
                pass
        
        error_msg = f"{{left_repr}} {{op}} {{right_repr}}"
        if diff_output:
            error_msg += f"\\n\\nDetailed diff:\\n{{diff_output}}"
        
        self._raise_assertion_error(error_msg, line_number, file_path, original_expr)
    
    def _generate_string_diff(self, left: str, right: str) -> str:
        """Generate a unified diff for strings."""
        left_lines = left.splitlines(keepends=True)
        right_lines = right.splitlines(keepends=True)
        
        diff = difflib.unified_diff(
            left_lines, right_lines,
            fromfile='expected', tofile='actual',
            lineterm=''
        )
        
        return ''.join(diff)
    
    def _generate_sequence_diff(self, left: Any, right: Any) -> str:
        """Generate a diff for sequences."""
        try:
            left_formatted = pprint.pformat(left, width=80)
            right_formatted = pprint.pformat(right, width=80)
            
            left_lines = left_formatted.splitlines(keepends=True)
            right_lines = right_formatted.splitlines(keepends=True)
            
            diff = difflib.unified_diff(
                left_lines, right_lines,
                fromfile='expected', tofile='actual',
                lineterm=''
            )
            
            return ''.join(diff)
        except:
            return f"Left: {{repr(left)}}\\nRight: {{repr(right)}}"
    
    def _raise_assertion_error(self, message: str, line_number: int, file_path: str, original_expr: str):
        """Raise an AssertionError with enhanced details."""
        full_message = f"""AssertionError at {{file_path}}:{{line_number}}
Original: {{original_expr}}
Details: {{message}}"""
        raise AssertionError(full_message)


# Global instance
assert_helper = AssertionHelper()
"#,
            self.diff_threshold
        )
    }
}

impl Default for AssertionHelpers {
    fn default() -> Self {
        Self::new()
    }
}

/// Format assertion failures with better diffs (legacy function)
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
fn enhance_membership_assertion(_error: &str) -> Option<String> {
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
fn extract_comparison_values(_error: &str) -> Option<(String, String)> {
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
