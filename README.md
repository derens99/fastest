# Fastest ⚡ - High-Performance Python Test Runner

[![Crates.io](https://img.shields.io/crates/v/fastest.svg)](https://crates.io/crates/fastest)
[![CI](https://github.com/derens99/fastest/actions/workflows/ci.yml/badge.svg)](https://github.com/derens99/fastest/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Fastest** is a Rust-backed Python test runner focused on pytest-style discovery and execution. It is under active compatibility work and is currently best treated as an experimental runner for validated suites, not a production-ready drop-in pytest replacement.

## ✨ Key Features

- **Rust CLI and execution engine** for pytest-style test runs
- **Discovery for functions, classes, async tests, and parametrized tests**
- **Common fixture and marker support** including `tmp_path`, `capsys`, `monkeypatch`, skip, and xfail
- **Passing project gates** for the Rust workspace and Python project test suite
- **Compatibility suites** for tracking pytest behavior by feature area
- **Experimental advanced features** for coverage, incremental runs, watch mode, and plugins

## 🚀 Quick Start

```bash
# Install via Cargo
cargo install fastest-cli

# Run tests
fastest                    # Run all tests
fastest tests/             # Run specific directory
fastest -k "login" -v      # Filter tests with verbose output
fastest -n 4               # Worker option, currently compatibility-first
```

## 📊 Current Verification

These gates pass in the current cleanup branch:

| Gate | Result |
|------|--------|
| `make verify` | Runs lint, Rust workspace tests, Python project tests, and selected compatibility suites |
| `make compat-report-all` | Generates a full compatibility baseline report under `target/compatibility-report-all.json` |
| `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo test --workspace` | Passing |
| `uv run pytest tests -q` | Passing |
| `make lint` | Passing |
| `make compat-report-all` | All discovered compatibility categories pass with expected skips/xfails |

Older fixed-speedup claims are being replaced with reproducible benchmark artifacts. See the [roadmap](docs/reference/roadmap.md) for the current evidence and next gates.

## 📦 Installation

### From Crates.io (Recommended)
```bash
cargo install fastest-cli
```

### From Source
```bash
git clone https://github.com/derens99/fastest
cd fastest
PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo build --release
export PATH="$PWD/target/release:$PATH"
```

**Requirements**: Rust 1.70+, Python 3.8+

[Installation guide →](docs/getting-started/installation.md)

## 🎮 Usage

### Basic Commands
```bash
fastest                    # Run all tests
fastest tests/             # Run specific directory  
fastest -k "test_login"    # Filter by keyword
fastest -m "not slow"      # Filter by markers
fastest -n 4               # Worker option, currently compatibility-first
fastest -x                 # Stop on first failure
fastest -v                 # Verbose output
```

### Experimental Features
```bash
fastest --coverage         # Experimental coverage framework
fastest --incremental      # Experimental changed-test framework
fastest --watch            # Experimental watch mode
fastest --prioritize       # Experimental prioritization framework
```

[Full usage guide →](docs/getting-started/quickstart.md)

## 🏗️ Architecture

Fastest uses a modular Rust workspace with 5 specialized crates:

```
fastest/
├── crates/
│   ├── fastest-core/       # Test discovery, parsing, caching
│   ├── fastest-execution/  # Execution strategies and runtime
│   ├── fastest-advanced/   # Coverage, incremental, watch mode
│   ├── fastest-cli/        # Command-line interface
│   ├── fastest-plugins/    # Plugin system and pytest compatibility
│   └── fastest-plugins-macros/
├── docs/                   # User and development documentation
├── pytest-compat-suite/    # pytest compatibility fixtures
├── scripts/                # Development, benchmark, and release tools
└── tests/                  # Fastest project tests
```

- **Python AST discovery** with parametrization expansion
- **PyO3-based execution** with a compatibility-first in-process path
- **Discovery caching** and structured test metadata
- **Experimental strategy, plugin, and advanced-feature modules**

### Core Architecture Components

#### fastest-core
- **TestItem**: Core test representation with metadata, fixtures, markers
- **DiscoveryCache**: Content-based caching with xxHash validation
- **Parser**: Tree-sitter based Python AST parsing
- **AdvancedFixtureManager**: pytest-style fixture system covered by local compatibility suites
- **Config**: Multi-source configuration loading

#### fastest-execution
- **UltraFastExecutor**: Compatibility-first execution engine
- **PythonRuntime**: PyO3-based Python integration
- **WorkStealingExecutor**: Experimental parallel execution
- **ZeroCopyExecutor**: Experimental arena allocation

#### fastest-advanced
- **SmartCoverage**: Experimental coverage integration
- **IncrementalTester**: Experimental git-based change detection
- **TestPrioritizer**: Experimental test ordering
- **DependencyTracker**: Experimental dependency analysis

[Full architecture documentation →](docs/development/architecture.md)


## 🔌 Plugin System

Fastest has plugin scaffolding and compatibility layers under active development:

| Plugin | Status | Features |
|--------|--------|----------|
| pytest-mock | Experimental smoke | Basic mocker fixture path |
| pytest-cov | Experimental smoke | Package availability plus coverage scaffold |
| pytest-timeout | Experimental smoke | Timeout marker visibility |
| pytest-asyncio | Experimental smoke | Async marker path |
| pytest-xdist | Package smoke only | Package availability; distributed execution remains future work |

[Plugin documentation →](docs/features/plugins.md)

## ⚙️ Configuration

```toml
# pyproject.toml
[tool.fastest]
test_paths = ["tests", "src"]
python_files = ["test_*.py", "*_test.py"]
num_workers = "auto"
timeout = 300
```

[Configuration guide →](docs/getting-started/quickstart.md#configuration)

## 🔨 Development

```bash
# Build
PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo build --release

# Test
export PYO3_PYTHON="$(command -v python3.12 || command -v python3)"
cargo test --workspace
uv run pytest tests -q
make lint

# Benchmark
cargo bench

# Debug
RUST_LOG=debug fastest
```

[Development guide →](docs/development/contributing.md)

## 🗺️ Roadmap

**Current Status**: Rust and Python project gates pass; the generated
compatibility report passes every discovered category with expected skips/xfails.

**✅ Stabilized**:
- Project test layout and verification gates
- Generated compatibility report
- Compatibility suites by feature category

**🚧 In Progress**:
- Honest performance benchmarking
- Broader third-party plugin smoke coverage
- End-to-end gates for advanced CLI features

**📅 Coming Soon**:
- Test matrices (Python versions, OS)
- Revalidated performance strategy work

[Full roadmap →](docs/reference/roadmap.md)

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
