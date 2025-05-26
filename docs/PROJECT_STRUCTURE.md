# Fastest Project Structure

This document provides an overview of the Fastest codebase organization.

## Directory Layout

```
fastest/
├── crates/                    # Rust workspace
│   ├── fastest-cli/          # Command-line interface
│   ├── fastest-core/         # Core test discovery and execution
│   └── fastest-python/       # Python bindings (PyO3)
├── python/                   # Python package
├── benchmarks/              # Performance benchmarks
├── tests/                   # Integration tests
├── docs/                    # Documentation
└── scripts/                 # Build and installation scripts
```

## Core Components

### fastest-cli
The command-line interface that users interact with.

**Key files:**
- `main.rs` - Entry point and CLI argument parsing
- `commands/` - Command implementations (run, discover, version)
- `output.rs` - Formatted output and progress bars

### fastest-core
The heart of Fastest - handles test discovery, execution, and all core functionality.

**Key modules:**
- `discovery/` - Test discovery with regex and AST parsers
- `execution/` - Test execution strategies (single, batch, parallel)
- `fixtures/` - Fixture system implementation
- `markers/` - Test marker support
- `cache/` - Discovery caching for performance
- `config/` - Configuration file handling

### fastest-python
Python bindings using PyO3 to expose Rust functionality to Python.

**Key files:**
- `lib.rs` - PyO3 module definition
- `test_info.rs` - Python-compatible test information structures
- `discovery.rs` - Python API for test discovery
- `execution.rs` - Python API for test execution

## Architecture Highlights

### Test Discovery
1. **File Walker** - Efficiently traverses directories looking for test files
2. **Parser** - Two strategies:
   - Regex parser (default): Fast pattern matching
   - AST parser (--parser ast): Tree-sitter based, more accurate
3. **Cache** - Stores discovery results with file modification tracking

### Test Execution
1. **Single** - Run tests one by one (debugging)
2. **Batch** - Group tests by module for efficiency (default)
3. **Parallel** - Work-stealing thread pool for maximum performance

### Python Integration
- Uses subprocess for test execution to ensure isolation
- JSON serialization for data exchange
- Minimal overhead design

## Development Setup

### Prerequisites
- Rust 1.70+ (via [rustup](https://rustup.rs/))
- Python 3.8+
- maturin (for Python bindings)

### Building
```bash
# Build all Rust crates
cargo build --release

# Build Python bindings
maturin develop

# Run tests
cargo test
python -m pytest tests/
```

### Key Dependencies
- **clap** - CLI argument parsing
- **rayon** - Parallel processing
- **serde** - Serialization
- **pyo3** - Python bindings
- **tree-sitter** - AST parsing
- **indicatif** - Progress bars

## Contributing

When contributing to Fastest:

1. **Rust code** goes in the appropriate crate under `crates/`
2. **Python code** for bindings goes in `crates/fastest-python/`
3. **Tests** should cover both Rust (unit) and Python (integration) levels
4. **Benchmarks** should be added for performance-critical changes

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines 