# Fastest - High-Performance Python Test Runner

[![CI](https://github.com/derens99/fastest/actions/workflows/ci.yml/badge.svg)](https://github.com/derens99/fastest/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Fastest** is a Python test runner written in Rust that discovers and executes pytest-style tests with parallel execution. It uses a hybrid execution engine that automatically selects between in-process (PyO3) and subprocess pool strategies based on suite size.

> **Beta Release** - Fastest v2 is a complete rewrite. Core discovery and execution work well, but some features (markers, parametrize values, watch mode) are still being refined. Bug reports welcome!

## Quick Start

### Install with pip (recommended)

```bash
pip install fastest-runner
```

### Install from source

```bash
git clone https://github.com/derens99/fastest
cd fastest
cargo build --release

# Binary at target/release/fastest
```

### Install from GitHub releases

Pre-built binaries are available for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64) on the [Releases](https://github.com/derens99/fastest/releases) page.

### Usage

```bash
# Run all tests in current directory
fastest

# Run tests in specific directory
fastest tests/

# Discover tests without running
fastest discover tests/

# Filter by keyword expression
fastest -k "test_add or test_sub"

# Filter by marker expression
fastest -m "slow and not integration"

# Stop on first failure
fastest -x

# Verbose output
fastest -v

# Set worker count
fastest -j 4

# JSON output
fastest --output json

# JUnit XML report
fastest --junit-xml results.xml

# Incremental mode (only tests affected by uncommitted changes)
fastest --incremental

# Watch mode (re-run on file changes)
fastest --watch
```

## Architecture

Fastest is a Rust workspace with 3 crates:

```
fastest/
├── crates/
│   ├── fastest-core/       # Discovery, parsing, config, markers, fixtures, plugins
│   ├── fastest-execution/  # Hybrid executor (PyO3 in-process + subprocess pool)
│   └── fastest-cli/        # CLI interface (clap) and output formatting
├── .github/workflows/      # CI and semantic-release
└── scripts/                # Build and release helpers
```

### Discovery

- **AST-based parsing** with `rustpython-parser` for reliable Python test extraction
- Parallel file discovery and parsing via `rayon`
- Supports `test_*.py` and `*_test.py` files, `Test*` classes, `test_*` functions
- Configurable via `pyproject.toml`, `pytest.ini`, `setup.cfg`, or `tox.ini`

### Execution

The hybrid executor automatically selects a strategy:

| Test Count | Strategy | Description |
|-----------|----------|-------------|
| 1-20 | In-process | Direct PyO3 execution, minimal overhead |
| 21+ | Subprocess pool | Isolated processes with crossbeam work-stealing |

### Features

- **Keyword filtering** (`-k`): Boolean expressions against test names (`-k "add or sub"`)
- **Marker filtering** (`-m`): Boolean expressions against markers (`-m "slow and not integration"`)
- **Parametrize expansion**: `@pytest.mark.parametrize` with cross-product support
- **Fixture system**: Dependency resolution, conftest.py discovery, scope-aware caching
- **Plugin system**: Trait-based with 4 built-in plugins (fixture, marker, reporting, capture)
- **Incremental testing**: git-based change detection for running only affected tests
- **Watch mode**: File system monitoring with debounced re-execution
- **Multiple output formats**: Pretty (colored), JSON, count, JUnit XML

## Configuration

Fastest reads pytest-compatible configuration from `pyproject.toml`:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py", "*_test.py"]
python_classes = ["Test*"]
python_functions = ["test_*"]

[tool.fastest]
workers = 4          # Number of parallel workers (default: CPU count)
incremental = false  # Enable incremental testing
verbose = false      # Verbose output
```

Also supports `pytest.ini`, `setup.cfg` (`[tool:pytest]`), and `tox.ini` (`[pytest]`).

## Development

### Prerequisites

- Rust stable toolchain (managed via `rust-toolchain.toml`)
- Python 3.9+ (required for PyO3)

### Building

```bash
cargo build --workspace              # Debug build
cargo build --release --workspace    # Release build (LTO enabled)
```

### Testing

```bash
cargo test --workspace               # All tests
cargo clippy --workspace -- -D warnings  # Lint
cargo fmt --all -- --check           # Format check
```

### Docker

```bash
docker build -t fastest .
docker run -v $(pwd)/tests:/workspace fastest tests/
```

## CI/CD

- **CI** runs on every push: fmt, clippy, test (Linux/macOS/Windows), release build
- **Releases** via semantic-release on `main`: conventional commits drive versioning, binaries uploaded for 5 platform targets

## License

MIT License - see [LICENSE](LICENSE) for details.
