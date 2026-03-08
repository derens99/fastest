# Development Guide

This guide helps you set up and contribute to the Fastest project.

## Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs/))
- Python 3.9 or later
- Git

## Setting Up Development Environment

### 1. Clone the Repository

```bash
git clone https://github.com/derens99/fastest.git
cd fastest
```

### 2. Install Rust Tools

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add required components
rustup component add rustfmt clippy
```

### 3. Build and Test

```bash
# Debug build
cargo build --workspace

# Release build (with LTO)
cargo build --release --workspace

# Run all tests
cargo test --workspace

# Run clippy lints
cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

### 4. Set Up Python Environment (Optional)

Only needed for running benchmarks or Python-based integration tests:

```bash
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install pytest  # For comparison benchmarks
```

## Project Structure

```
fastest/
├── crates/
│   ├── fastest-core/       # Discovery, parsing, config, markers, fixtures, plugins
│   │   ├── src/
│   │   │   ├── lib.rs      # Public API and re-exports
│   │   │   ├── config.rs   # Multi-format config loading
│   │   │   ├── model.rs    # TestItem, TestResult, TestOutcome
│   │   │   ├── discovery/  # AST-based test discovery (rustpython-parser)
│   │   │   ├── fixtures/   # Fixture system (conftest, scoping, builtins)
│   │   │   ├── markers.rs  # Marker system and expression parser
│   │   │   ├── parametrize.rs  # @pytest.mark.parametrize expansion
│   │   │   ├── plugins/    # Plugin trait and manager
│   │   │   ├── incremental/ # Git-based change detection
│   │   │   └── watch.rs    # File watcher
│   │   └── Cargo.toml
│   ├── fastest-execution/  # Hybrid executor (PyO3 in-process + subprocess pool)
│   │   ├── src/
│   │   │   ├── executor.rs    # HybridExecutor strategy selection
│   │   │   ├── inprocess.rs   # PyO3 in-process execution
│   │   │   ├── subprocess.rs  # Subprocess pool with crossbeam work-stealing
│   │   │   ├── worker_harness.py  # Python-side test runner
│   │   │   └── timeout.rs    # Timeout configuration
│   │   └── Cargo.toml
│   └── fastest-cli/        # CLI interface (clap) and output formatting
│       ├── src/
│       │   ├── main.rs     # CLI entry point and test pipeline
│       │   ├── output.rs   # Pretty, JSON, count, JUnit XML formatters
│       │   └── progress.rs # Spinner/progress bar
│       └── Cargo.toml
├── tests/                  # Integration tests and test fixtures
│   ├── checks/             # Python test files used to validate fastest
│   └── integration/        # Python integration test scripts
├── testing_files/          # Additional Python test fixtures
├── benchmarks/             # Performance benchmarks (fastest vs pytest)
├── .github/workflows/      # CI and semantic-release
└── scripts/                # Build and release helper scripts
```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/my-new-feature
```

### 2. Make Changes

Follow the coding standards:
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes with zero warnings
- Add tests for new functionality
- Use conventional commits (`feat:`, `fix:`, `docs:`, etc.)

### 3. Run Tests

```bash
# Run all tests
cargo test --workspace

# Run with output
cargo test --workspace -- --nocapture

# Run specific test
cargo test test_name
```

### 4. Test with Python Files

```bash
# Build release binary
cargo build --release

# Test with example Python files
./target/release/fastest tests/checks/

# Compare with pytest
pytest tests/checks/ -v
```

## Debugging

### Rust Debugging

```rust
// Use eprintln! for debug output (goes to stderr)
eprintln!("Debug: variable = {:?}", variable);
```

### Python Test Debugging

```bash
# Verbose mode shows each test's status
./target/release/fastest tests/ -v

# Show full tracebacks
./target/release/fastest tests/ --tb=long
```

## CI/CD

- **CI** (`ci.yml`): Runs on every push — fmt, clippy, test (Linux/macOS/Windows), release build
- **Release** (`semantic-release.yml`): Triggered after CI passes on `main` — conventional commits drive versioning, binaries uploaded for 5 targets, wheels published to PyPI

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [PyO3 Guide](https://pyo3.rs/) (for Python integration)
- [Rayon Docs](https://docs.rs/rayon/) (for parallelism)
- [Clap Docs](https://docs.rs/clap/) (for CLI)
