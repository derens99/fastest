# Fastest Project Structure

This document provides an overview of the Fastest project organization.

## Directory Structure

```
fastest/
├── crates/                      # Rust workspace members
│   ├── fastest-cli/            # Command-line interface
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs         # CLI entry point
│   │
│   ├── fastest-core/           # Core functionality
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          # Core library exports
│   │       ├── discovery.rs    # Test discovery logic
│   │       ├── cache.rs        # Discovery caching
│   │       ├── error.rs        # Error types
│   │       ├── utils.rs        # Utility functions
│   │       ├── config/         # Configuration handling
│   │       ├── executor/       # Test execution engines
│   │       │   ├── mod.rs      # Public executor API
│   │       │   ├── single.rs   # Single test execution
│   │       │   ├── batch.rs    # Batch test execution
│   │       │   ├── parallel.rs # Parallel execution
│   │       │   └── process_pool.rs # Process pool (WIP)
│   │       ├── fixtures/       # Fixture support (WIP)
│   │       ├── markers/        # Test markers and filtering
│   │       └── parser/         # Test file parsing
│   │           ├── mod.rs      # Parser module exports
│   │           ├── regex.rs    # Regex-based parser
│   │           └── ast.rs      # Tree-sitter AST parser
│   │
│   └── fastest-python/         # Python bindings
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs          # PyO3 bindings
│
├── benchmarks/                 # Performance benchmarks
│   ├── benchmark.py           # Discovery performance
│   ├── benchmark_parsers.py   # Parser comparison
│   ├── benchmark_parallel.py  # Parallel execution
│   └── format_benchmark_json.py # CI benchmark formatter
│
├── docs/                      # Documentation
│   ├── FASTEST_MARKERS.md    # Marker system guide
│   ├── PROJECT_STRUCTURE.md  # This file
│   ├── parallel-execution-guide.md
│   ├── TREE_SITTER_PLAN.md
│   └── development/          # Development notes
│
├── tests/                     # Test scripts
│   ├── test_fastest.py       # Basic functionality tests
│   └── test_enhanced.py      # Advanced feature tests
│
├── test_project/             # Sample project for testing
│   └── tests/               # Sample test files
│
├── .github/                  # GitHub configuration
│   └── workflows/
│       └── benchmark.yml    # CI benchmark workflow
│
├── fastest.py               # Python module for markers
├── Cargo.toml              # Workspace configuration
├── Cargo.lock              # Dependency lock file
├── Makefile                # Common development tasks
├── requirements-dev.txt    # Python dev dependencies
├── README.md               # Project documentation
├── LICENSE                 # MIT license
├── Dockerfile              # Container build file
└── .gitignore             # Git ignore patterns
```

## Key Components

### fastest-cli
The command-line interface that users interact with. Handles:
- Argument parsing with clap
- Test discovery and filtering
- Progress reporting
- Output formatting

### fastest-core
The core library containing all the business logic:
- **discovery.rs**: Walks directories to find test files
- **parser/**: Two parsing strategies (regex for speed, AST for accuracy)
- **executor/**: Different execution strategies (single, batch, parallel)
- **markers/**: Test marker support for both pytest and fastest
- **cache.rs**: Caches discovery results with file modification tracking
- **config/**: Configuration file support (pytest.ini, pyproject.toml)

### fastest-python
Python bindings using PyO3 that expose:
- `discover_tests()`: Find tests in a directory
- `run_test()`: Run a single test
- `run_tests_batch()`: Run tests in batches
- `run_tests_parallel()`: Run tests in parallel

## Development Workflow

1. **Build**: `cargo build --release`
2. **Test**: `cargo test`
3. **Python bindings**: `cd crates/fastest-python && maturin develop`
4. **Format**: `cargo fmt`
5. **Lint**: `cargo clippy`
6. **Clean**: `make clean`

## Adding New Features

1. **Core functionality**: Add to `fastest-core`
2. **CLI options**: Update `fastest-cli/src/main.rs`
3. **Python API**: Update `fastest-python/src/lib.rs`
4. **Documentation**: Update relevant docs and README

## Performance Considerations

- Regex parser is faster for large codebases (>5000 tests)
- AST parser is more accurate and faster for smaller codebases
- Batch execution groups tests by module to minimize overhead
- Parallel execution uses work-stealing for load balancing
- Discovery cache avoids re-parsing unchanged files 