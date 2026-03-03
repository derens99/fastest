//! Marker classification and expression-based filtering for pytest markers and keywords.
//!
//! Supports filtering tests by marker expressions (e.g. `-m "slow and not integration"`)
//! and keyword expressions (e.g. `-k "test_add or test_subtract"`).

use crate::model::{Marker, TestItem};

/// Built-in marker classification for pytest markers.
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinMarker {
    Skip {
        reason: Option<String>,
    },
    Skipif {
        condition: String,
        reason: Option<String>,
    },
    Xfail {
        reason: Option<String>,
    },
    Parametrize,
    Timeout(f64),
    Custom(String),
}

/// Classify a [`Marker`] into a [`BuiltinMarker`].
///
/// Recognizes the standard pytest markers: `skip`, `skipif`, `xfail`,
/// `parametrize`, and `timeout`. Everything else becomes `Custom`.
pub fn classify_marker(marker: &Marker) -> BuiltinMarker {
    match marker.name.as_str() {
        "skip" => {
            let reason = extract_reason(marker);
            BuiltinMarker::Skip { reason }
        }
        "skipif" => {
            let condition = marker
                .args
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let reason = marker
                .kwargs
                .get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            BuiltinMarker::Skipif { condition, reason }
        }
        "xfail" => {
            let reason = extract_reason(marker);
            BuiltinMarker::Xfail { reason }
        }
        "parametrize" => BuiltinMarker::Parametrize,
        "timeout" => {
            let seconds = marker.args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
            BuiltinMarker::Timeout(seconds)
        }
        other => BuiltinMarker::Custom(other.to_string()),
    }
}

/// Filter tests by a marker expression string.
///
/// The expression supports:
/// - Simple names: `slow` matches tests with a marker named "slow"
/// - `and`: `slow and integration`
/// - `or`: `slow or fast`
/// - `not`: `not slow`
/// - Parentheses: `(slow or fast) and not integration`
pub fn filter_by_markers(tests: &[TestItem], expr: &str) -> Vec<TestItem> {
    let expr = expr.trim();
    if expr.is_empty() {
        return tests.to_vec();
    }
    let tokens = tokenize(expr);
    let ast = parse_expression(&tokens);
    tests
        .iter()
        .filter(|test| {
            let marker_names: Vec<&str> = test.markers.iter().map(|m| m.name.as_str()).collect();
            evaluate(&ast, &|name| marker_names.contains(&name))
        })
        .cloned()
        .collect()
}

/// Filter tests by a keyword expression string.
///
/// Keywords are matched against `function_name`, `class_name`, and `id`.
/// The expression supports the same boolean syntax as marker expressions.
pub fn filter_by_keyword(tests: &[TestItem], expr: &str) -> Vec<TestItem> {
    let expr = expr.trim();
    if expr.is_empty() {
        return tests.to_vec();
    }
    let tokens = tokenize(expr);
    let ast = parse_expression(&tokens);
    tests
        .iter()
        .filter(|test| evaluate(&ast, &|keyword| test_matches_keyword(test, keyword)))
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the `reason` kwarg from a marker, falling back to the first
/// positional string arg for markers like `@pytest.mark.skip("reason")`.
fn extract_reason(marker: &Marker) -> Option<String> {
    marker
        .kwargs
        .get("reason")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            marker
                .args
                .first()
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

/// Check whether a test matches a keyword substring (case-insensitive).
fn test_matches_keyword(test: &TestItem, keyword: &str) -> bool {
    let kw = keyword.to_lowercase();
    if test.function_name.to_lowercase().contains(&kw) {
        return true;
    }
    if test.id.to_lowercase().contains(&kw) {
        return true;
    }
    if let Some(ref class) = test.class_name {
        if class.to_lowercase().contains(&kw) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Expression tokenizer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Name(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '(' {
            tokens.push(Token::LParen);
            chars.next();
        } else if ch == ')' {
            tokens.push(Token::RParen);
            chars.next();
        } else if ch.is_alphanumeric() || ch == '_' {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            match word.as_str() {
                "and" => tokens.push(Token::And),
                "or" => tokens.push(Token::Or),
                "not" => tokens.push(Token::Not),
                _ => tokens.push(Token::Name(word)),
            }
        } else {
            // Skip unknown characters
            chars.next();
        }
    }

    tokens
}

// ---------------------------------------------------------------------------
// Expression parser (recursive descent)
// ---------------------------------------------------------------------------

/// AST node for boolean expressions.
#[derive(Debug, Clone)]
enum Expr {
    Name(String),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

/// Parser state wrapping a token slice with a cursor position.
struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    /// `or_expr` = `and_expr` ( "or" `and_expr` )*
    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        while self.peek() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_and();
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        left
    }

    /// `and_expr` = `not_expr` ( "and" `not_expr` )*
    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_not();
        while self.peek() == Some(&Token::And) {
            self.advance();
            let right = self.parse_not();
            left = Expr::And(Box::new(left), Box::new(right));
        }
        left
    }

    /// `not_expr` = "not" `not_expr` | `primary`
    fn parse_not(&mut self) -> Expr {
        if self.peek() == Some(&Token::Not) {
            self.advance();
            let inner = self.parse_not();
            return Expr::Not(Box::new(inner));
        }
        self.parse_primary()
    }

    /// `primary` = `Name` | "(" `or_expr` ")"
    fn parse_primary(&mut self) -> Expr {
        match self.peek().cloned() {
            Some(Token::LParen) => {
                self.advance(); // consume '('
                let expr = self.parse_or();
                if self.peek() == Some(&Token::RParen) {
                    self.advance(); // consume ')'
                }
                expr
            }
            Some(Token::Name(name)) => {
                self.advance();
                Expr::Name(name)
            }
            _ => {
                // Fallback: treat as empty name (should not happen with valid input)
                self.advance();
                Expr::Name(String::new())
            }
        }
    }
}

/// Parse a token stream into an expression AST.
fn parse_expression(tokens: &[Token]) -> Expr {
    let mut parser = Parser::new(tokens);
    parser.parse_or()
}

/// Evaluate an expression AST with a name-matching predicate.
fn evaluate(expr: &Expr, matcher: &dyn Fn(&str) -> bool) -> bool {
    match expr {
        Expr::Name(name) => matcher(name),
        Expr::Not(inner) => !evaluate(inner, matcher),
        Expr::And(left, right) => evaluate(left, matcher) && evaluate(right, matcher),
        Expr::Or(left, right) => evaluate(left, matcher) || evaluate(right, matcher),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Helper to build a Marker with a name and optional kwargs.
    fn make_marker(name: &str) -> Marker {
        Marker {
            name: name.to_string(),
            args: vec![],
            kwargs: HashMap::new(),
        }
    }

    fn make_marker_with_reason(name: &str, reason: &str) -> Marker {
        let mut kwargs = HashMap::new();
        kwargs.insert(
            "reason".to_string(),
            serde_json::Value::String(reason.to_string()),
        );
        Marker {
            name: name.to_string(),
            args: vec![],
            kwargs,
        }
    }

    fn make_marker_with_args(name: &str, args: Vec<serde_json::Value>) -> Marker {
        Marker {
            name: name.to_string(),
            args,
            kwargs: HashMap::new(),
        }
    }

    /// Helper to build a minimal TestItem.
    fn make_test(id: &str, func: &str, class: Option<&str>, markers: Vec<Marker>) -> TestItem {
        TestItem {
            id: id.to_string(),
            path: PathBuf::from("tests/test_example.py"),
            function_name: func.to_string(),
            line_number: Some(1),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: class.map(|s| s.to_string()),
            markers,
            parameters: None,
            name: func.to_string(),
        }
    }

    // ----- classify_marker tests -----

    #[test]
    fn test_classify_skip() {
        let marker = make_marker_with_reason("skip", "not ready");
        let classified = classify_marker(&marker);
        assert_eq!(
            classified,
            BuiltinMarker::Skip {
                reason: Some("not ready".to_string())
            }
        );

        // skip without reason
        let marker_no_reason = make_marker("skip");
        assert_eq!(
            classify_marker(&marker_no_reason),
            BuiltinMarker::Skip { reason: None }
        );
    }

    #[test]
    fn test_classify_xfail() {
        let marker = make_marker_with_reason("xfail", "known bug");
        let classified = classify_marker(&marker);
        assert_eq!(
            classified,
            BuiltinMarker::Xfail {
                reason: Some("known bug".to_string())
            }
        );
    }

    #[test]
    fn test_classify_custom() {
        let marker = make_marker("slow");
        let classified = classify_marker(&marker);
        assert_eq!(classified, BuiltinMarker::Custom("slow".to_string()));
    }

    // ----- filter_by_markers tests -----

    #[test]
    fn test_filter_by_marker_simple() {
        let tests = vec![
            make_test("test_a", "test_a", None, vec![make_marker("slow")]),
            make_test("test_b", "test_b", None, vec![make_marker("fast")]),
            make_test(
                "test_c",
                "test_c",
                None,
                vec![make_marker("slow"), make_marker("integration")],
            ),
        ];

        let result = filter_by_markers(&tests, "slow");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "test_a");
        assert_eq!(result[1].id, "test_c");
    }

    #[test]
    fn test_filter_by_marker_and_or_not() {
        let tests = vec![
            make_test(
                "test_a",
                "test_a",
                None,
                vec![make_marker("slow"), make_marker("integration")],
            ),
            make_test("test_b", "test_b", None, vec![make_marker("slow")]),
            make_test("test_c", "test_c", None, vec![make_marker("fast")]),
            make_test(
                "test_d",
                "test_d",
                None,
                vec![make_marker("fast"), make_marker("integration")],
            ),
        ];

        // "slow and integration" -> only test_a
        let result = filter_by_markers(&tests, "slow and integration");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "test_a");

        // "slow or fast" -> test_a, test_b, test_c, test_d
        let result = filter_by_markers(&tests, "slow or fast");
        assert_eq!(result.len(), 4);

        // "not slow" -> test_c, test_d
        let result = filter_by_markers(&tests, "not slow");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "test_c");
        assert_eq!(result[1].id, "test_d");

        // "(slow or fast) and not integration" -> test_b, test_c
        let result = filter_by_markers(&tests, "(slow or fast) and not integration");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "test_b");
        assert_eq!(result[1].id, "test_c");
    }

    // ----- filter_by_keyword tests -----

    #[test]
    fn test_filter_by_keyword_simple() {
        let tests = vec![
            make_test("tests/test_math.py::test_add", "test_add", None, vec![]),
            make_test(
                "tests/test_math.py::test_subtract",
                "test_subtract",
                None,
                vec![],
            ),
            make_test(
                "tests/test_math.py::TestCalc::test_multiply",
                "test_multiply",
                Some("TestCalc"),
                vec![],
            ),
        ];

        // Match by function name
        let result = filter_by_keyword(&tests, "test_add");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].function_name, "test_add");

        // Match by class name
        let result = filter_by_keyword(&tests, "TestCalc");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].function_name, "test_multiply");

        // Match by id substring
        let result = filter_by_keyword(&tests, "test_math");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_filter_by_keyword_or() {
        let tests = vec![
            make_test("tests/test_a.py::test_add", "test_add", None, vec![]),
            make_test(
                "tests/test_b.py::test_subtract",
                "test_subtract",
                None,
                vec![],
            ),
            make_test(
                "tests/test_c.py::test_multiply",
                "test_multiply",
                None,
                vec![],
            ),
        ];

        let result = filter_by_keyword(&tests, "add or subtract");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].function_name, "test_add");
        assert_eq!(result[1].function_name, "test_subtract");
    }

    // ----- Additional internal tests -----

    #[test]
    fn test_classify_skipif() {
        let marker = make_marker_with_args(
            "skipif",
            vec![serde_json::Value::String("sys.platform == 'win32'".into())],
        );
        let classified = classify_marker(&marker);
        match classified {
            BuiltinMarker::Skipif { condition, reason } => {
                assert_eq!(condition, "sys.platform == 'win32'");
                assert_eq!(reason, None);
            }
            other => panic!("expected Skipif, got {:?}", other),
        }
    }

    #[test]
    fn test_classify_timeout() {
        let marker = make_marker_with_args("timeout", vec![serde_json::json!(5.0)]);
        let classified = classify_marker(&marker);
        assert_eq!(classified, BuiltinMarker::Timeout(5.0));
    }

    #[test]
    fn test_classify_parametrize() {
        let marker = make_marker("parametrize");
        assert_eq!(classify_marker(&marker), BuiltinMarker::Parametrize);
    }

    #[test]
    fn test_tokenizer() {
        let tokens = tokenize("(slow or fast) and not integration");
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Name("slow".into()),
                Token::Or,
                Token::Name("fast".into()),
                Token::RParen,
                Token::And,
                Token::Not,
                Token::Name("integration".into()),
            ]
        );
    }

    #[test]
    fn test_empty_expression() {
        let tests = vec![make_test("test_a", "test_a", None, vec![])];
        let result = filter_by_markers(&tests, "");
        assert_eq!(result.len(), 1);
        let result = filter_by_keyword(&tests, "");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_extract_reason_from_positional_arg() {
        let marker = make_marker_with_args(
            "skip",
            vec![serde_json::Value::String("positional reason".into())],
        );
        let classified = classify_marker(&marker);
        assert_eq!(
            classified,
            BuiltinMarker::Skip {
                reason: Some("positional reason".to_string())
            }
        );
    }
}
