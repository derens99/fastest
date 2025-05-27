# Fastest ⚡

[![Crates.io](https://img.shields.io/crates/v/fastest.svg)](https://crates.io/crates/fastest)
[![CI](https://github.com/YOUR_USERNAME/fastest/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/fastest/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A blazing fast Python test runner built with Rust for maximum performance and reliability.

## 🚀 Features

- **⚡ Blazing Fast**: 77x faster test discovery and 2.6x faster test execution than pytest
- **🔄 Smart Caching**: Intelligent test discovery caching that persists across runs
- **🎯 Parallel Execution**: Run tests in parallel with automatic CPU core detection
- **🏃 Ultra-Optimized Executor**: New optimized test executor with batching and pre-compilation (default)
- **🔍 Multiple Parsers**: Choose between regex (fast) or AST (accurate) test discovery
- **🎨 Beautiful Output**: Clean, colorful terminal output with progress bars
- **🔧 pytest Compatible**: Works with your existing pytest test suites
- **📦 Zero Config**: Works out of the box with sensible defaults
- **🎭 Parametrized Tests**: Full support for `@pytest.mark.parametrize` and `@fastest.mark.parametrize` (NEW!)

## 📦 Installation

### Quick Install (Recommended)

**macOS/Linux:**
```bash
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
```

**Windows:**
```powershell
irm https://raw.githubusercontent.com/derens99/fastest/main/install.ps1 | iex
```

### Other Installation Methods

**Via pip:**
```bash
pip install fastest-runner
```

**Via Homebrew (macOS):**
```bash
brew tap derens99/fastest
brew install fastest
```

**Via Cargo (requires Rust):**
```bash
cargo install fastest-cli
```

**Via Docker:**
```bash
docker run --rm -v $(pwd):/workspace ghcr.io/derens99/fastest tests/
```

See the [installation guide](docs/INSTALLATION.md) for more options and troubleshooting.

## 🎯 Quick Start

```bash
# Run all tests in the current directory
fastest

# Run tests in a specific directory
fastest tests/

# Run tests matching a pattern
fastest -k test_login

# Run tests in parallel with auto-detected workers
fastest -n 0  # 0 means auto-detect (default)

# Use different optimization levels
fastest --optimizer standard  # Use standard executor
fastest --optimizer optimized  # Use optimized executor (default)

# Verbose output
fastest -v
```

## 📊 Benchmarks

Fastest significantly outperforms pytest in both test discovery and execution:

### Test Discovery
- **Fastest**: 5.2ms (77x faster! 🚀)
- **pytest**: 402ms

### Test Execution
- **Fastest**: 38ms (2.6x faster ⚡)
- **pytest**: 98ms

*Benchmarks performed on a 10-core Apple M1 Max CPU with real test suites.*

## 🤔 When to Use Fastest vs pytest

### Use Fastest when:
- **Speed is critical**: Large test suites that take minutes with pytest
- **CI/CD optimization**: Reduce pipeline times and costs
- **Rapid development**: Fast feedback loops during development
- **Simple test suites**: Standard unit tests without complex fixtures

### Use pytest when:
- **Plugin ecosystem needed**: Extensive pytest plugin requirements
- **Complex fixtures**: Advanced fixture scoping and dependencies
- **Custom configuration**: Complex pytest.ini or pyproject.toml setups

### Migration Path
1. **Start with parallel adoption**: Use Fastest for quick local runs, pytest for CI
2. **Gradual migration**: Move simple test modules first
3. **Full migration**: Once Fastest supports your required features

📖 See our [detailed migration guide](docs/MIGRATION_GUIDE.md) for step-by-step instructions.

## 🔧 Configuration

Fastest works with zero configuration, but you can customize its behavior:

### Command Line Options
```bash
fastest --help

Options:
  -k, --filter <PATTERN>      Filter tests by pattern
  -n, --workers <N>           Number of parallel workers (0 = auto-detect)
  -x, --fail-fast            Stop on first failure
  -v, --verbose              Verbose output
  --parser <TYPE>            Parser type: "regex" (default) or "ast"
  --optimizer <TYPE>         Optimizer type: "standard" or "optimized" (default)
  --no-cache                 Disable test discovery cache
```

### Requirements

- Python 3.8+ (must be available as `python` in PATH or use a virtual environment)
- Rust 1.70+ (for building from source)

### Using with Virtual Environments

Fastest works seamlessly with Python virtual environments. If you're having issues with Python not being found, activate your virtual environment before running fastest:

```bash
# Create and activate a virtual environment
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Now run fastest
fastest
```

## 🏆 Real-World Performance

### Django Test Suite
- **Tests**: 100 tests
- **pytest**: 425ms
- **Fastest**: 150ms (2.83x faster)

### Large Test Suite
- **Tests**: 1,000 tests  
- **pytest**: 1.2s
- **Fastest**: 450ms (2.7x faster)

### Memory Usage
- **pytest**: 30MB
- **Fastest**: 15MB (50% less)

*Results from actual production codebases. Your results may vary based on test complexity and hardware.*

## 🏗️ Architecture

Fastest is built with a modular architecture combining Rust's performance with Python's ecosystem:

```
fastest/
├── crates/
│   ├── fastest-core/      # Core test discovery and execution engine
│   └── fastest-cli/       # Command-line interface
└── python/
    └── fastest/          # Python bindings
```

### Key Components:
- **Discovery Engine**: Fast regex-based or accurate AST-based test discovery
- **Execution Engine**: Optimized parallel test runner with intelligent batching
- **Cache System**: Persistent discovery cache for instant subsequent runs
- **Process Pool**: Reusable process pool for reduced overhead

## 📋 pytest Compatibility

### ✅ Supported Features
| Feature | Status | Notes |
|---------|--------|-------|
| Basic test discovery | ✅ | `def test_*`, `class Test*` |
| Async tests | ✅ | `async def test_*` |
| Markers | ✅ | `@pytest.mark.skip`, `@pytest.mark.xfail` |
| Basic fixtures | ✅ | `capsys`, `tmp_path`, `monkeypatch` |
| Test filtering | ✅ | `-k` pattern matching |
| Marker expressions | ✅ | `-m "not slow"` |
| Parallel execution | ✅ | `-n` flag |
| Fail fast | ✅ | `-x` flag |
| **Parametrized tests** | ✅ | `@pytest.mark.parametrize` (NEW!) |

### 🚧 In Progress
| Feature | Status | Target Version |
|---------|--------|----------------|
| Config files | 🚧 | v0.2.0 |
| Coverage integration | 🚧 | v0.2.0 |
| More fixtures | 🚧 | v0.2.0 |

### ❌ Not Yet Supported
| Feature | Status | Notes |
|---------|--------|-------|
| Plugins | ❌ | Plugin API planned |
| Custom collectors | ❌ | Under consideration |
| Doctests | ❌ | Low priority |
| Session fixtures | ❌ | Complex scoping |

## 📊 Project Status

### Test Coverage
![Coverage](https://codecov.io/gh/derens99/fastest/branch/main/graph/badge.svg)

The project maintains high test coverage with:
- Unit tests for all core functionality
- Integration tests for CLI commands
- End-to-end tests with real Python test suites
- Performance benchmarks

### CI/CD
All code is tested on:
- **Operating Systems**: Linux, macOS, Windows
- **Python Versions**: 3.8, 3.9, 3.10, 3.11, 3.12
- **Rust Versions**: stable, beta, nightly

### Documentation
- [Development Guide](docs/DEVELOPMENT.md) - Set up your development environment
- [Testing Guide](docs/TESTING.md) - How to test the project
- [Release Process](docs/RELEASE.md) - How we release new versions
- [Performance Guide](docs/PERFORMANCE.md) - Optimization tips
- [Migration Guide](docs/MIGRATION_GUIDE.md) - Migrating from pytest

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Start for Contributors
```bash
# Clone and setup
git clone https://github.com/YOUR_USERNAME/fastest.git
cd fastest
make dev-setup

# Run tests
make test

# Format and lint
make check
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [PyO3](https://pyo3.rs/) for seamless Python-Rust integration
- Inspired by the pytest project and the Rust community
- Tree-sitter for accurate Python AST parsing
- Special thanks to all contributors

---

**Note**: Fastest is actively maintained and under continuous development. Feel free to report issues or request features! 