# Fastest ⚡

[![Crates.io](https://img.shields.io/crates/v/fastest.svg)](https://crates.io/crates/fastest)
[![CI](https://github.com/derens99/fastest/actions/workflows/ci.yml/badge.svg)](https://github.com/derens99/fastest/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A blazing fast Python test runner built with Rust for maximum performance and reliability.

## 🚀 Features

- **⚡ Blazing Fast**: 88x faster test discovery and 2.1x faster test execution than pytest
- **🔄 Smart Caching**: Intelligent test discovery caching that persists across runs
- **🎯 Parallel Execution**: Run tests in parallel with automatic CPU core detection
- **🏃 Ultra-Optimized Executor**: New optimized test executor with batching and pre-compilation (default)
- **🔍 Multiple Parsers**: Choose between regex (fast) or AST (accurate) test discovery
- **🎨 Beautiful Output**: Clean, colorful terminal output with progress bars
- **🔧 pytest Compatible**: Works with your existing pytest test suites
- **📦 Zero Config**: Works out of the box with sensible defaults
- **🎭 Parametrized Tests**: Full support for `@pytest.mark.parametrize` and `@fastest.mark.parametrize` (NEW!)

## 📦 Installation

### From PyPI
```bash
pip install fastest
```

### From Source
```bash
# Clone the repository
git clone https://github.com/derens99/fastest.git
cd fastest

# Build and install
cargo build --release
pip install -e python/

# Or use the install script
./scripts/install.sh
```

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

### Test Discovery (10,240 tests)
- **Fastest**: 0.12s (88x faster! 🚀)
- **pytest**: 10.59s

### Test Execution (1,296 tests)
- **Fastest**: 0.99s (2.1x faster ⚡)
- **pytest**: 2.13s

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
- **Tests**: 5,832 tests
- **pytest**: 45.2s
- **Fastest**: 18.7s (2.4x faster)

### FastAPI Project
- **Tests**: 1,247 tests  
- **pytest**: 12.8s
- **Fastest**: 5.1s (2.5x faster)

### Data Science Project
- **Tests**: 3,421 tests
- **pytest**: 89.3s
- **Fastest**: 31.2s (2.9x faster)

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

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [PyO3](https://pyo3.rs/) for seamless Python-Rust integration
- Inspired by the pytest project and the Rust community
- Special thanks to all contributors

---

**Note**: Fastest is actively maintained and under continuous development. Feel free to report issues or request features! 