# 🚀 Fastest - A Blazing Fast Python Test Runner

**Fastest** is a high-performance Python test runner built with Rust, designed to significantly speed up your test discovery and execution. It's a drop-in replacement for pytest with massive performance improvements.

## ✨ Features

- **⚡ 88x faster test discovery** than pytest (with AST parser: even faster for <5000 tests)
- **🏃 2.1x faster test execution** than pytest  
- **🚹 Parallel test execution** with customizable worker count
- **💾 Smart caching** for instant repeated runs
- **🦀 Written in Rust** for maximum performance
- **🐍 Pure Python API** via PyO3 bindings
- **💻 Full-featured CLI** with colored output and progress bars
- **🔍 Smart test filtering** with `-k` pattern matching and `-m` marker expressions
- **🏷️ Full marker support** for both `pytest.mark.*` and `fastest.mark.*`
- **🧩 Fixture support** with discovery, dependency resolution, and built-in fixtures
- **🌳 Tree-sitter AST parser** for accurate Python parsing
- **📦 Zero dependencies** for the test runner (your tests can use any framework)

## 📊 Performance

Based on real benchmarks:

| Operation | Pytest | Fastest | Speedup |
|-----------|--------|---------|---------|
| Discovery (10 tests) | 125ms | 1.4ms | **88x faster** |
| Discovery (1,000 tests) | 358ms | 6.7ms | **53x faster** |
| Execution (10 tests) | 187ms | 89ms | **2.1x faster** |
| Execution (100 tests) | 1,872ms | 892ms | **2.1x faster** |

## 🚀 Quick Start

### Installation

#### Using the installer script (Recommended)

**macOS and Linux:**
```bash
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/yourusername/fastest/main/install.ps1 | iex
```

The installer will:
- Download the latest binary for your platform
- Install it to `~/.fastest/bin` (or `%USERPROFILE%\.fastest\bin` on Windows)
- Add it to your PATH automatically
- Handle shell configuration (bash, zsh, fish, PowerShell)

#### Build from source

```bash
# Clone the repository
git clone https://github.com/yourusername/fastest.git
cd fastest

# Build the project (requires Rust)
cargo build --release

# The binary will be at target/release/fastest
# Copy it to a location in your PATH
cp target/release/fastest /usr/local/bin/
```

#### Install Python bindings (optional)

If you want to use Fastest as a Python library:

```bash
# Install development dependencies
pip install -r requirements-dev.txt

# Build and install Python bindings
maturin develop
```

### Command Line Usage

```bash
# Run all tests in the current directory
fastest

# Run tests in a specific directory
fastest path/to/tests

# Filter tests by pattern
fastest -k "test_important"

# Filter tests by markers
fastest -m "not slow"              # Skip slow tests
fastest -m "unit or integration"   # Run unit or integration tests
fastest -m "smoke and not skip"    # Run smoke tests that aren't skipped

# Run with verbose output
fastest -v

# Run with parallel execution (auto-detect workers)
fastest -n 0

# Run with specific number of workers
fastest -n 4

# Use tree-sitter AST parser (more accurate, faster for <5000 tests)
fastest --parser ast

# Discover tests without running them
fastest discover

# Disable cache for fresh discovery
fastest --no-cache

# Show version
fastest version
```

### Python API Usage

```python
import fastest

# Discover tests
tests = fastest.discover_tests("path/to/tests")
for test in tests:
    print(f"Found: {test.id}")

# Run tests individually
for test in tests:
    result = fastest.run_test(test)
    print(f"{test.id}: {'PASSED' if result.passed else 'FAILED'}")

# Run tests in batch (fastest method)
results = fastest.run_tests_batch(tests)
for result in results:
    if not result.passed:
        print(f"FAILED: {result.test_id}")
        print(f"Error: {result.error}")

# Run tests in parallel (even faster for many tests)
results = fastest.run_tests_parallel(tests, num_workers=4)
# num_workers=None for auto-detection based on CPU cores
```

### Using Markers

```python
import fastest

# Use fastest native markers
@fastest.mark.skip(reason="Not implemented yet")
def test_future_feature():
    pass

@fastest.mark.slow
def test_heavy_computation():
    # Long running test
    pass

# pytest markers also work for compatibility
import pytest

@pytest.mark.xfail
def test_known_issue():
    assert False  # This failure is expected
```

## 📁 Project Structure

```
fastest/
├── crates/
│   ├── fastest-cli/       # Command-line interface
│   ├── fastest-core/      # Core functionality (discovery, execution)
│   └── fastest-python/    # Python bindings via PyO3
├── benchmarks/            # Performance benchmarks
│   ├── benchmark.py       # Main performance comparison
│   ├── benchmark_v2.py    # Batch execution benchmarks
│   └── ...
├── tests/                 # Test scripts for validation
│   ├── test_fastest.py    # Basic functionality test
│   └── test_enhanced.py   # Advanced features test
├── test_project/          # Sample test project for testing
├── Cargo.toml             # Rust workspace configuration
├── requirements-dev.txt   # Python development dependencies
└── README.md              # This file
```

## 🏗️ Architecture

Fastest achieves its performance through several key optimizations:

1. **Rust-based Discovery**: File traversal and regex parsing in Rust is orders of magnitude faster than Python AST parsing
2. **Batch Execution**: Tests are grouped by module and run in batches, minimizing subprocess overhead
3. **Smart Caching**: Test discovery results are cached with file modification tracking
4. **Process Pool**: Parallel test execution with minimal overhead (coming soon)

## 🧪 Supported Test Types

- ✅ Function-based tests (`def test_*`)
- ✅ Async tests (`async def test_*`)
- ✅ Class-based tests (`class Test*` with `test_*` methods)
- ✅ Nested test directories
- ✅ Test markers (`@fastest.mark.*` and `@pytest.mark.*`)
- ✅ Fixtures (basic support with built-ins: `tmp_path`, `capsys`, `monkeypatch`)
- 🚧 Parametrized tests (coming soon)

## 🎯 Roadmap

### Phase 1: MVP ✅
- [x] Fast test discovery using Rust
- [x] Basic test execution
- [x] Python bindings
- [x] CLI application

### Phase 2: Performance ✅
- [x] Batch execution (2.1x speedup)
- [x] Discovery caching (1.5x speedup)
- [x] Parallel execution with work-stealing (1.2-2x speedup)
- [x] Tree-sitter AST parser for faster parsing

### Phase 3: Compatibility ✅
- [x] Test markers and filtering (`-m` flag with expressions)
- [x] Support for both `pytest.mark.*` and `fastest.mark.*`
- [x] Fixture discovery and dependency extraction
- [x] Fixture execution framework with Python bridge
- [x] Built-in fixtures (tmp_path, capsys, monkeypatch)
- [x] Scope-based fixture caching
- [ ] Configuration file support (pytest.ini, pyproject.toml)
- [ ] JUnit XML output

### Phase 4: Advanced Features 🔮
- [ ] Watch mode for continuous testing
- [ ] Coverage integration
- [ ] IDE integrations (VS Code, PyCharm)
- [ ] Distributed testing

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- Built with [PyO3](https://pyo3.rs/) for Python bindings
- Inspired by the need for speed in large Python codebases
- Thanks to the Rust community for excellent crates 