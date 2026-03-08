# Fastest - High-Performance Python Test Runner

[![CI](https://github.com/derens99/fastest/actions/workflows/ci.yml/badge.svg)](https://github.com/derens99/fastest/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Fastest** is a Python test runner written in Rust that discovers and executes pytest-style tests with parallel execution. It uses a hybrid execution engine that automatically selects between in-process (PyO3) and subprocess pool strategies based on suite size.

> **Beta Release** - Fastest v2 is a complete rewrite. Core discovery and execution work well, but some features are still being refined. Bug reports welcome!

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

# Re-run only last failed tests
fastest --lf

# Run failed tests first, then the rest
fastest --ff
```

---

## Pytest Compatibility

Fastest is designed as a **drop-in replacement for pytest** on most projects. The tables below show a comprehensive feature-by-feature comparison so you know exactly what works today, what's partial, and what's not yet supported.

### Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Fully supported |
| 🟡 | Partially supported — works for common cases but may lack edge cases |
| ❌ | Not yet supported |
| 🚀 | Fastest-only feature — not available in pytest |

---

### Test Discovery

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| `test_*.py` / `*_test.py` files | ✅ | ✅ | Configurable via `python_files` |
| `test_*` functions | ✅ | ✅ | Configurable via `python_functions` |
| `Test*` classes | ✅ | ✅ | Configurable via `python_classes` |
| `test_*` methods in classes | ✅ | ✅ | |
| `async def test_*` functions | ✅ | ✅ | Detected and executed |
| Nested test classes (`Test* > Test*`) | ✅ | ✅ | Full `::` separated ID chain |
| Parametrize expansion | ✅ | ✅ | Cross-product of multiple decorators |
| `pytest.param()` with custom IDs | ✅ | ✅ | `pytest.param(val, id="name")` |
| `--collect-only` / `discover` | ✅ | ✅ | List tests without running |
| `conftest.py` discovery | ✅ | ✅ | Per-directory, deeper overrides shallower |
| `collect_ignore` / `collect_ignore_glob` | ✅ | ❌ | |
| Doctest collection | ✅ | ❌ | |
| Package-level `__init__.py` collection | ✅ | ❌ | |

### Markers

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| `@pytest.mark.skip` | ✅ | ✅ | With optional `reason=` |
| `@pytest.mark.skipif(condition)` | ✅ | ✅ | Condition expression evaluated |
| `@pytest.mark.xfail` | ✅ | ✅ | With optional `reason=` |
| `@pytest.mark.parametrize` | ✅ | ✅ | Single, multi-param, `ids=`, `pytest.param()` |
| `@pytest.mark.timeout` | ✅ | 🟡 | Parsed and stored, default 10s timeout enforced |
| `@pytest.mark.usefixtures` | ✅ | ❌ | |
| `@pytest.mark.filterwarnings` | ✅ | ❌ | |
| Custom markers | ✅ | ✅ | Any `@pytest.mark.X` works with `-m` filtering |
| `-m` marker expression filtering | ✅ | ✅ | `and`, `or`, `not`, `()` supported |
| `--strict-markers` | ✅ | ❌ | |

### Fixtures

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| `@pytest.fixture` from conftest.py | ✅ | ✅ | Per-directory discovery |
| Fixture dependency resolution | ✅ | ✅ | Topological sort with cycle detection |
| `scope="function"` | ✅ | ✅ | Default scope |
| `scope="class"` | ✅ | 🟡 | Parsed, runtime caching partial |
| `scope="module"` | ✅ | 🟡 | Parsed, runtime caching partial |
| `scope="package"` | ✅ | 🟡 | Parsed, runtime caching partial |
| `scope="session"` | ✅ | 🟡 | Parsed, runtime caching partial |
| `autouse=True` | ✅ | ✅ | Auto-injected into all tests in scope |
| `params=[...]` fixture parametrize | ✅ | ✅ | Fixture-level parametrization |
| `yield` fixtures (teardown) | ✅ | 🟡 | Basic support |
| Fixture finalizers (`request.addfinalizer`) | ✅ | ✅ | With `_run_finalizers()` cleanup |

#### Built-in Fixtures

| Fixture | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| `tmp_path` | ✅ | ✅ | Temporary directory per test |
| `tmp_path_factory` | ✅ | ✅ | Factory for multiple temp dirs |
| `capsys` | ✅ | ✅ | Capture stdout/stderr |
| `capfd` | ✅ | 🟡 | Stub — aliases to capsys behavior |
| `caplog` | ✅ | ✅ | Full: handler, records, text, messages, set_level, clear |
| `monkeypatch` | ✅ | ✅ | setattr, delattr, setenv, delenv, chdir, syspath_prepend |
| `request` | ✅ | ✅ | addfinalizer, node info |
| `pytestconfig` | ✅ | 🟡 | Basic object, `getini()` returns defaults |
| `cache` | ✅ | 🟡 | In-memory per-session, not persisted to disk |
| `recwarn` | ✅ | ❌ | |
| `doctest_namespace` | ✅ | ❌ | |

### CLI Flags & Options

| Flag | pytest | Fastest | Notes |
|------|--------|---------|-------|
| `-k EXPR` | ✅ | ✅ | Keyword expression filter |
| `-m EXPR` | ✅ | ✅ | Marker expression filter |
| `-x` / `--exitfirst` | ✅ | ✅ | Stop on first failure |
| `--maxfail=N` | ✅ | ✅ | Stop after N failures |
| `-v` / `--verbose` | ✅ | ✅ | Verbose output with timings |
| `-q` / `--quiet` | ✅ | ✅ | Minimal output |
| `-s` | ✅ | ✅ | Disable output capture |
| `--tb=short\|long\|no` | ✅ | ✅ | Traceback format |
| `--durations=N` | ✅ | ✅ | Show N slowest tests |
| `--color=yes\|no\|auto` | ✅ | ✅ | Color output control |
| `--lf` / `--last-failed` | ✅ | ✅ | Re-run only failed tests |
| `--ff` / `--failed-first` | ✅ | ✅ | Failed tests first, then rest |
| `--collect-only` | ✅ | ✅ | Discover without running |
| `--junit-xml=PATH` | ✅ | ✅ | JUnit XML report |
| `-j` / `--workers` | ❌ | ✅ | 🚀 Parallel workers (pytest needs `xdist`) |
| `--watch` | ❌ | ✅ | 🚀 Built-in file watcher |
| `--incremental` | ❌ | ✅ | 🚀 Git-based change detection |
| `--no-progress` | ❌ | ✅ | 🚀 Disable progress bar |
| `--output json\|count` | ❌ | ✅ | 🚀 Native JSON & count formats |
| `-n` / `--numprocesses` (xdist) | ✅* | ❌ | *Requires `pytest-xdist` plugin |
| `--rootdir` | ✅ | ❌ | |
| `--override-ini` | ✅ | ❌ | |
| `-p` (plugin control) | ✅ | ❌ | |
| `--co` (collect-only short) | ✅ | ❌ | Use `discover` subcommand instead |

### Configuration

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| `pyproject.toml` (`[tool.pytest.ini_options]`) | ✅ | ✅ | Primary config source |
| `pytest.ini` | ✅ | ✅ | |
| `setup.cfg` (`[tool:pytest]`) | ✅ | ✅ | |
| `tox.ini` (`[pytest]`) | ✅ | ✅ | |
| `testpaths` | ✅ | ✅ | Directories to search |
| `python_files` | ✅ | ✅ | File patterns |
| `python_classes` | ✅ | ✅ | Class patterns |
| `python_functions` | ✅ | ✅ | Function patterns |
| `addopts` | ✅ | ✅ | Config-level CLI defaults |
| `markers` | ✅ | ✅ | Custom marker definitions |
| `minversion` | ✅ | 🟡 | Parsed but not enforced |
| `cache_dir` | ✅ | 🟡 | Parsed, uses `.fastest_cache/` |
| `filterwarnings` | ✅ | ❌ | |
| `norecursedirs` | ✅ | ❌ | Hardcoded skip list instead |
| `confcutdir` | ✅ | ❌ | |
| `[tool.fastest]` config section | ❌ | ✅ | 🚀 workers, incremental, verbose |

### Output & Reporting

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| Colored terminal output | ✅ | ✅ | ANSI colors, auto-detect terminal |
| Pass/fail/skip/xfail/xpass/error states | ✅ | ✅ | All 6 outcome states |
| Failure tracebacks | ✅ | ✅ | short, long, no modes |
| Slowest tests (`--durations`) | ✅ | ✅ | |
| JUnit XML | ✅ | ✅ | Full `<testsuites>` with stdout/stderr |
| Native JSON output | ❌ | ✅ | 🚀 `--output json` |
| Count-only summary | ❌ | ✅ | 🚀 `--output count` |
| Progress bar | ❌ | ✅ | 🚀 Spinner-based with live counter |
| Warnings summary | ✅ | ❌ | |
| Short test summary (`-r`) | ✅ | ❌ | |
| Header / session info | ✅ | ❌ | |

### Execution

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| Sequential execution | ✅ | ✅ | |
| Parallel execution | ✅* | ✅ | *pytest requires `xdist` plugin; fastest has built-in |
| In-process execution (PyO3) | ❌ | ✅ | 🚀 ≤20 tests: zero-overhead in-process |
| Subprocess isolation | ✅ | ✅ | >20 tests: work-stealing pool |
| `setup_module` / `teardown_module` | ✅ | ❌ | |
| `setup_class` / `teardown_class` | ✅ | ❌ | |
| `setup_function` / `teardown_function` | ✅ | ❌ | |
| Output capture (stdout/stderr) | ✅ | ✅ | |
| Per-test timeout | ✅* | ✅ | *pytest requires `pytest-timeout` plugin |

### Plugins & Hooks

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| Plugin system | ✅ | ✅ | Trait-based (4 built-in plugins) |
| Third-party plugin ecosystem | ✅ | ❌ | Pytest's plugin ecosystem is unmatched |
| `conftest.py` hooks | ✅ | ❌ | Only fixture extraction supported |
| `pytest_collection_modifyitems` | ✅ | 🟡 | Internal hook, not user-facing |
| `--no-plugins` | ❌ | ✅ | 🚀 Disable all plugins |

### Advanced Features

| Feature | pytest | Fastest | Notes |
|---------|--------|---------|-------|
| Watch mode (built-in) | ❌ | ✅ | 🚀 `--watch` with 300ms debounce |
| Incremental testing (git-based) | ❌ | ✅ | 🚀 `--incremental` detects changed files |
| Last-failed cache | ✅ | ✅ | Persistent `.fastest_cache/lastfailed` |
| Failed-first ordering | ✅ | ✅ | `--ff` |
| Hybrid execution engine | ❌ | ✅ | 🚀 Auto-selects PyO3 vs subprocess |
| Parallel file parsing | ❌ | ✅ | 🚀 rayon-based discovery |
| `pytest.raises` | ✅ | ❌ | Use standard Python `with self.assertRaises()` |
| `pytest.warns` | ✅ | ❌ | |
| `pytest.approx` | ✅ | ❌ | |
| `pytest.importorskip` | ✅ | ❌ | |
| Doctest support | ✅ | ❌ | |
| `unittest.TestCase` compat | ✅ | ❌ | |

---

### Compatibility Summary

| Category | Coverage | Detail |
|----------|----------|--------|
| **Test Discovery** | **92%** | All common patterns; missing doctest, `collect_ignore` |
| **Markers** | **85%** | All core markers; missing `usefixtures`, `filterwarnings` |
| **Fixtures** | **80%** | 9 built-ins, conftest, autouse, params; scoped caching partial |
| **CLI Flags** | **82%** | All common flags plus extras; missing `-p`, `--rootdir` |
| **Configuration** | **88%** | All 4 config formats + key options; missing `norecursedirs` |
| **Output** | **85%** | 4 formats + progress; missing warnings summary |
| **Execution** | **75%** | Parallel, capture, timeout; missing setup/teardown hooks |
| **Overall** | **~83%** | Drop-in for most projects, plus unique Rust-powered features |

> **Bottom line:** If your project uses standard `test_*` functions, classes, fixtures from `conftest.py`, `parametrize`, markers (`skip`, `skipif`, `xfail`), and common CLI flags — Fastest will work out of the box and run significantly faster. Projects relying on the pytest plugin ecosystem, `setup_class`/`teardown_class`, or `pytest.raises` context manager will need adjustments.

---

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
- Nested test classes with full `::` separated ID chain
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
- **Parametrize expansion**: `@pytest.mark.parametrize` with cross-product and `pytest.param()` support
- **Fixture system**: Dependency resolution, conftest.py discovery, autouse, scope-aware caching
- **9 built-in fixtures**: `tmp_path`, `capsys`, `capfd`, `caplog`, `monkeypatch`, `request`, `cache`, `pytestconfig`, `tmp_path_factory`
- **Plugin system**: Trait-based with 4 built-in plugins (fixture, marker, reporting, capture)
- **Incremental testing**: git-based change detection for running only affected tests
- **Watch mode**: File system monitoring with debounced re-execution
- **Last-failed / failed-first**: Persistent cache with `--lf` and `--ff` flags
- **Multiple output formats**: Pretty (colored), JSON, count, JUnit XML

## Configuration

Fastest reads pytest-compatible configuration from `pyproject.toml`:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py", "*_test.py"]
python_classes = ["Test*"]
python_functions = ["test_*"]
addopts = "-v --tb=short"

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
