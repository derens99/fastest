# Fastest ⚡ - High-Performance Python Test Runner

[![Crates.io](https://img.shields.io/crates/v/fastest.svg)](https://crates.io/crates/fastest)
[![CI](https://github.com/derens99/fastest/actions/workflows/ci.yml/badge.svg)](https://github.com/derens99/fastest/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Fastest** is a blazing-fast Python test runner written in Rust, achieving **3.9x speedup** over pytest through parallel execution and intelligent work-stealing algorithms. With ~91% pytest compatibility, it's ready for real-world projects!

## ✨ Key Features

- **⚡ 3.9x faster than pytest** - Verified on real test suites (3,200-5,700 tests/second)
- **🔄 ~91% pytest compatible** - Works with most existing test suites  
- **🚀 Intelligent execution** - Automatically selects optimal strategy based on test count
- **🔌 Plugin system** - Compatible with pytest plugins (mock, coverage)
- **📊 Smart features** - Coverage, incremental testing, file watching

## 🚀 Quick Start

```bash
# Install via Cargo
cargo install fastest-cli

# Run tests
fastest                    # Run all tests
fastest tests/             # Run specific directory
fastest -k "login" -v      # Filter tests with verbose output
fastest -n 4               # Use 4 parallel workers
```

## 📊 Performance

| Test Count | Strategy | Speed | vs pytest |
|------------|----------|-------|-----------|
| ≤20 | InProcess | 45 tests/sec | 2x faster |
| 21-100 | HybridBurst | 180-250 tests/sec | 3x faster |
| >100 | WorkStealing | **5,700 tests/sec** | **3.9x faster** |

[See detailed benchmarks →](docs/PERFORMANCE.md)

## 📦 Installation

### From Crates.io (Recommended)
```bash
cargo install fastest-cli
```

### From Source
```bash
git clone https://github.com/derens99/fastest
cd fastest
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

**Requirements**: Rust 1.70+, Python 3.7+

[Installation guide →](docs/INSTALLATION.md)

## 🎮 Usage

### Basic Commands
```bash
fastest                    # Run all tests
fastest tests/             # Run specific directory  
fastest -k "test_login"    # Filter by keyword
fastest -m "not slow"      # Filter by markers
fastest -n 4               # Use 4 parallel workers
fastest -x                 # Stop on first failure
fastest -v                 # Verbose output
```

### Advanced Features
```bash
fastest --coverage         # Enable coverage collection
fastest --incremental      # Only run changed tests
fastest --watch            # Auto-run on file changes
fastest --prioritize       # Smart test ordering
```

[Full usage guide →](docs/QUICKSTART.md)

## 🏗️ Architecture

Fastest uses a modular Rust workspace with 5 specialized crates:

```
fastest/
├── fastest-core/       # Test discovery, parsing, caching  
├── fastest-execution/  # Execution strategies and runtime
├── fastest-advanced/   # Coverage, incremental, watch mode
├── fastest-cli/        # Command-line interface
└── fastest-plugins/    # Plugin system and pytest compatibility
```

- **Intelligent execution strategies** based on test count
- **SIMD-accelerated parsing** and JSON processing
- **Lock-free parallelism** with work-stealing queues
- **Zero-copy IPC** for process communication

### Core Architecture Components

#### fastest-core
- **TestItem**: Core test representation with metadata, fixtures, markers
- **DiscoveryCache**: Content-based caching with xxHash validation
- **Parser**: Tree-sitter based Python AST parsing
- **AdvancedFixtureManager**: Full pytest-compatible fixture system
- **Config**: Multi-source configuration loading

#### fastest-execution  
- **UltraFastExecutor**: Intelligent strategy selection engine
- **PythonRuntime**: PyO3-based Python integration
- **WorkStealingExecutor**: Lock-free parallel execution
- **ZeroCopyExecutor**: Arena allocation for zero-copy performance

#### fastest-advanced
- **SmartCoverage**: Integrated coverage collection
- **IncrementalTester**: Git-based change detection
- **TestPrioritizer**: ML-based test ordering
- **DependencyTracker**: Graph-based dependency analysis

[Full architecture documentation →](docs/development/architecture.md)


## 🔌 Plugin System

Fastest supports pytest plugins for seamless migration:

| Plugin | Status | Features |
|--------|--------|----------|
| pytest-mock | ✅ Ready | Full mocker fixture |
| pytest-cov | ✅ Ready | Coverage reporting |
| pytest-xdist | 🚧 Planned | Distributed testing |
| pytest-asyncio | 🚧 Planned | Async test support |

[Plugin documentation →](docs/PLUGIN_SYSTEM.md)

## ⚙️ Configuration

```toml
# pyproject.toml
[tool.fastest]
test_paths = ["tests", "src"]
python_files = ["test_*.py", "*_test.py"]
num_workers = "auto"
timeout = 300
```

[Configuration guide →](docs/QUICKSTART.md#configuration)

## 🔨 Development

```bash
# Build
cargo build --release

# Test
cargo test

# Benchmark
cargo bench

# Debug
RUST_LOG=debug fastest
```

[Development guide →](docs/DEVELOPMENT.md)

## 🗺️ Roadmap

**Current Status**: ~91% pytest compatible, 3.9x faster than pytest

**✅ Completed**:
- Class-based tests, fixtures, parametrization
- Setup/teardown, markers, plugin system

**🚧 In Progress**:
- Enhanced error reporting
- More pytest plugins (xdist, asyncio)

**📅 Coming Soon**:
- 95%+ pytest compatibility
- Test matrices (Python versions, OS)
- AI-powered test selection

[Full roadmap →](docs/ROADMAP.md)

## 🤝 Contributing

We welcome contributions! Priority areas:
- Enhanced error reporting and assertion introspection
- Additional pytest plugin compatibility
- Performance optimizations
- Documentation improvements

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- The pytest team for the excellent testing framework
- The Rust community for amazing performance tools
- All contributors and early adopters

---

**Fastest** - Making Python testing faster through intelligent engineering and Rust performance.