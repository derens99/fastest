//! Phase 4: IDE Integration
//!
//! Simple IDE integration for development tools

pub mod simple;

pub use simple::{SimpleIdeIntegration, IdeTestItem, TestStatus, IdeTestResult, TestKind};

// Full LSP implementation would go here when tower-lsp is available
// For now we provide the simple integration