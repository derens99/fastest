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

#[derive(Debug, Clone)]
pub struct FixtureDefinition {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub scope: String, // "function", "class", "module", "session"
    pub autouse: bool,
    pub params: Vec<String>, // For parametrized fixtures
    pub decorators: Vec<String>,
}

pub fn parse_fixtures_and_tests(path: &std::path::Path) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>), Box<dyn std::error::Error>> {
    regex::RegexParser::parse_fixtures_and_tests(path)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
} 