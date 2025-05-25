pub mod regex;
pub mod ast;

// Re-export common types
pub use regex::{parse_test_file, TestFunction};
pub use ast::AstParser;

// Parser selection enum
pub enum Parser {
    Regex,
    Ast,
} 