//! Error types for fastest-advanced crate

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Watch error: {0}")]
    Watch(#[from] notify::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Dependency tracking error: {0}")]
    DependencyTracking(String),

    #[error("Advanced feature error: {0}")]
    Advanced(String),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("Core error: {0}")]
    Core(#[from] fastest_core::Error),
}
