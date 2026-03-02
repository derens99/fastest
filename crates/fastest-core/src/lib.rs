//! Core types, discovery, parsing, and configuration for Fastest test runner

pub mod config;
pub mod error;
pub mod model;

pub use config::Config;
pub use error::{Error, Result};
pub use model::{TestItem, TestOutcome, TestResult};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
