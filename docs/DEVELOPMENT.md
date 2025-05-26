# Development Guide

This guide helps you set up and contribute to the Fastest project.

## Prerequisites

- Rust 1.75 or later
- Python 3.8 or later
- Git

## Setting Up Development Environment

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/fastest.git
cd fastest
```

### 2. Install Rust

```bash
# Install rustup if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add required components
rustup component add rustfmt clippy
```

### 3. Install Development Tools

```bash
# Install cargo-watch for auto-recompilation
cargo install cargo-watch

# Install cargo-tarpaulin for coverage
cargo install cargo-tarpaulin

# Install cargo-audit for security checks
cargo install cargo-audit

# Install cargo-criterion for benchmarks
cargo install cargo-criterion
```

### 4. Set Up Python Environment

```bash
# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install Python dependencies
pip install -r requirements-dev.txt
```

### 5. Pre-commit Hooks

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install
```

## Project Structure

```
fastest/
├── crates/
│   ├── fastest-core/        # Core functionality
│   │   ├── src/
│   │   │   ├── lib.rs      # Library root
│   │   │   ├── discovery.rs # Test discovery
│   │   │   ├── executor/   # Test execution
│   │   │   ├── parser/     # Python parsing
│   │   │   └── ...
│   │   └── tests/          # Unit tests
│   ├── fastest-cli/        # CLI application
│   │   ├── src/
│   │   │   └── main.rs     # CLI entry point
│   │   └── tests/          # CLI tests
│   └── fastest-python/     # Python bindings (optional)
├── tests/                  # Integration tests
├── benchmarks/            # Performance benchmarks
├── docs/                  # Documentation
└── examples/              # Example usage
```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/my-new-feature
```

### 2. Make Changes

Follow the coding standards:
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes
- Add tests for new functionality
- Update documentation

### 3. Run Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test '*'
```

### 4. Check Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check for security issues
cargo audit

# Run coverage
cargo tarpaulin --out Html
```

### 5. Test with Python

```bash
# Build release binary
cargo build --release

# Test with example Python files
./target/release/fastest examples/

# Test with coverage
./target/release/fastest tests/ --cov
```

## Debugging

### Rust Debugging

1. **Using println! debugging**:
   ```rust
   eprintln!("Debug: variable = {:?}", variable);
   ```

2. **Using env_logger**:
   ```bash
   RUST_LOG=debug cargo run
   ```

3. **Using debugger (VS Code)**:
   - Install CodeLLDB extension
   - Use provided launch.json configuration

### Python Test Debugging

1. **Verbose mode**:
   ```bash
   ./target/release/fastest tests/ -v
   ```

2. **Check generated Python code**:
   ```bash
   # After running with -v, check:
   cat /tmp/fastest_debug.py
   ```

## Adding New Features

### 1. Core Features

Add to `crates/fastest-core/src/`:

```rust
// new_feature.rs
pub struct NewFeature {
    // Implementation
}

impl NewFeature {
    pub fn new() -> Self {
        // Constructor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_feature() {
        // Test implementation
    }
}
```

Don't forget to export from `lib.rs`:

```rust
pub mod new_feature;
pub use new_feature::NewFeature;
```

### 2. CLI Features

Add new command or flag in `crates/fastest-cli/src/main.rs`:

```rust
#[derive(Parser)]
struct Cli {
    /// New feature flag
    #[arg(long)]
    new_feature: bool,
}
```

### 3. Parser Features

For new Python parsing features, modify:
- `crates/fastest-core/src/parser/ast.rs` for AST parsing
- `crates/fastest-core/src/parser/regex.rs` for regex parsing

## Performance Optimization

### 1. Profiling

```bash
# CPU profiling with flamegraph
cargo install flamegraph
cargo flamegraph --bin fastest -- tests/

# Memory profiling with heaptrack
heaptrack ./target/release/fastest tests/
heaptrack --analyze heaptrack.fastest.*.gz
```

### 2. Benchmarking

Create benchmarks in `benchmarks/`:

```python
# bench_feature.py
import time
import subprocess

def benchmark():
    start = time.perf_counter()
    subprocess.run(["./target/release/fastest", "tests/"])
    return time.perf_counter() - start
```

## Common Tasks

### Update Dependencies

```bash
# Check outdated dependencies
cargo outdated

# Update dependencies
cargo update

# Update Cargo.toml versions
cargo upgrade
```

### Run CI Locally

```bash
# Install act
brew install act  # macOS

# Run CI
act -j test
```

### Generate Documentation

```bash
# Generate and open docs
cargo doc --open

# Generate docs with private items
cargo doc --document-private-items
```

## Troubleshooting

### Build Issues

1. **Linking errors**: Ensure you have required system libraries
2. **Python not found**: Check PYTHON_CMD detection in utils/python.rs
3. **Cache issues**: Clear with `cargo clean`

### Test Issues

1. **Flaky tests**: Run with `--test-threads=1`
2. **Discovery cache**: Clear with `rm -rf ~/Library/Caches/fastest`
3. **Python path**: Set VIRTUAL_ENV or use explicit Python path

## Contributing Guidelines

1. **Code Style**: Follow Rust conventions and run `cargo fmt`
2. **Documentation**: Update relevant docs and add inline comments
3. **Tests**: Add tests for new features (aim for 80%+ coverage)
4. **Commits**: Use conventional commits (feat:, fix:, docs:, etc.)
5. **PR Description**: Clearly describe changes and link issues

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [PyO3 Guide](https://pyo3.rs/) (for Python bindings)
- [Tree-sitter Docs](https://tree-sitter.github.io/tree-sitter/)
- [Rayon Docs](https://docs.rs/rayon/) (for parallelism) 