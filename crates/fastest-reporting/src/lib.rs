//! Rich test reporting and output formatting
//!
//! This crate provides comprehensive test result reporting, including enhanced output,
//! assertion formatting, and various output formats.

pub mod reporter;
pub mod enhanced_reporter;
pub mod assertions;

// Re-export main types
pub use reporter::*;
pub use enhanced_reporter::{EnhancedReporter, FailureReport, ReporterConfig};
pub use assertions::{
    format_assertion_error, AssertionConfig, AssertionHelpers, AssertionRewriter,
};