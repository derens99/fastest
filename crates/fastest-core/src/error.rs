use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Walk error: {0}")]
    Walk(#[from] walkdir::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Test execution error: {0}")]
    Execution(String),
    
    #[error("Discovery error: {0}")]
    Discovery(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>; 