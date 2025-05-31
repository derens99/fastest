pub mod tree_sitter;

// Re-export the tree-sitter parser as the default parser
pub use tree_sitter::{FixtureDefinition, Parser, TestFunction};

// For backward compatibility
pub use tree_sitter::Parser as TreeSitterParser;
