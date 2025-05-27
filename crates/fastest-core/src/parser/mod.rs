pub mod ast;
pub mod regex;
pub mod tree_sitter_parser;
pub mod tree_sitter_impl;

// Re-export common types
pub use ast::AstParser;
pub use regex::{parse_test_file, TestFunction};
pub use tree_sitter_parser::TreeSitterParser;

// Parser selection enum
#[derive(Debug, Clone, Copy)]
pub enum ParserType {
    Regex,
    Ast,
    TreeSitter,
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

pub fn parse_fixtures_and_tests(
    path: &std::path::Path,
    parser_type: ParserType,
) -> Result<(Vec<FixtureDefinition>, Vec<TestFunction>), Box<dyn std::error::Error>> {
    match parser_type {
        ParserType::Regex => regex::RegexParser::parse_fixtures_and_tests(path)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
        ParserType::Ast => ast::AstParser::parse_fixtures_and_tests(path)
            .map_err(Box::<dyn std::error::Error>::from),
        ParserType::TreeSitter => tree_sitter_parser::TreeSitterParser::parse_fixtures_and_tests(path)
            .map_err(Box::<dyn std::error::Error>::from),
    }
}
