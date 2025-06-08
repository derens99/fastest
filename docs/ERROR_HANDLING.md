# Error Handling Guidelines for Fastest

This document describes the error handling conventions used throughout the Fastest project.

## Overview

Fastest uses a consistent error handling strategy across all crates:
- **Library crates** use `thiserror` for custom error types
- **CLI crate** uses `anyhow` for user-facing error messages
- All library crates define their own `Result<T>` type alias

## Error Handling by Crate

### Library Crates (fastest-core, fastest-execution, fastest-plugins, fastest-advanced)

Library crates should:
1. Define custom error enums using `thiserror`
2. Create a `Result<T>` type alias
3. Include conversions from other crate errors
4. Include an `Other` variant with `anyhow::Error` for flexibility

Example structure:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Specific error: {0}")]
    SpecificError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Core error: {0}")]
    Core(#[from] fastest_core::Error),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### CLI Crate (fastest-cli)

The CLI crate should:
1. Use `anyhow::Result` directly
2. Not define custom error types
3. Use `.context()` to add error context
4. Format errors nicely for users

Example:
```rust
use anyhow::{Result, Context};

fn main() -> Result<()> {
    let config = load_config()
        .context("Failed to load configuration")?;
    Ok(())
}
```

## Error Propagation

### Between Library Crates
Use `#[from]` attribute for automatic conversion:
```rust
#[error("Execution error: {0}")]
Execution(#[from] fastest_execution::ExecutionError),
```

### From Libraries to CLI
The CLI can use `?` operator directly since library errors implement `std::error::Error`.

## Best Practices

1. **Be Specific**: Create specific error variants for different failure modes
2. **Add Context**: Use descriptive error messages that help debugging
3. **Avoid Strings**: Prefer enum variants over `String` errors
4. **Document Errors**: Add doc comments to error variants when not obvious
5. **Test Errors**: Write tests for error cases, not just success paths

## Common Error Types

### Core Errors (fastest-core)
- `Discovery`: Test discovery failures
- `Parse`: Python parsing errors
- `Cache`: Cache operation failures
- `Config`: Configuration errors

### Execution Errors (fastest-execution)
- `PythonRuntime`: Python interpreter issues
- `StrategySelection`: Strategy selection failures
- `Fixture`: Fixture-related errors
- `TestExecution`: Test execution failures
- `Timeout`: Test timeout errors

### Plugin Errors (fastest-plugins)
- `Load`: Plugin loading failures
- `Hook`: Hook execution errors
- `Configuration`: Plugin configuration issues

### Advanced Errors (fastest-advanced)
- `Git`: Git operation failures
- `Coverage`: Coverage collection errors
- `Watch`: File watching errors
- `Update`: Self-update failures

## Migration Guide

When updating code to use standardized error handling:

1. Replace `anyhow::Result` in library crates with crate-specific `Result<T>`
2. Convert string errors to specific enum variants
3. Add `#[from]` conversions for common error types
4. Update error propagation to use `?` operator
5. Add `.context()` in CLI for better error messages

## Example Migration

Before:
```rust
// In library crate
use anyhow::Result;

pub fn execute_test(test: &str) -> Result<String> {
    if test.is_empty() {
        return Err(anyhow::anyhow!("Test name cannot be empty"));
    }
    // ...
}
```

After:
```rust
// In library crate
use crate::error::{Result, ExecutionError};

pub fn execute_test(test: &str) -> Result<String> {
    if test.is_empty() {
        return Err(ExecutionError::TestExecution(
            "Test name cannot be empty".to_string()
        ));
    }
    // ...
}
```