# Scripts Directory

Utility scripts for building, testing, and maintaining Fastest - organized by purpose.

## 📁 Directory Structure

```
scripts/
├── benchmarks/      # Performance benchmarking
├── development/     # Development tools
├── release/         # Release management
└── utils/           # Miscellaneous utilities
```

## 📊 Benchmarking Scripts (`benchmarks/`)

### `official.py` ⭐
**Benchmark artifact generator** - compares Fastest and pytest and records the environment for release review.

**Usage:**
```bash
# Full benchmark suite
uv run python scripts/benchmarks/official.py

# Quick benchmark (for development)
uv run python scripts/benchmarks/official.py --quick --output-dir target/benchmark-artifacts/quick

# Custom output directory
uv run python scripts/benchmarks/official.py --output-dir results/
```

**Features:**
- Multiple test suite sizes (10-2000 tests)
- Realistic test patterns
- Detailed timing breakdowns
- System information capture
- Markdown and JSON artifacts for review (`benchmark_results.md` and `benchmark_results.json`)

### `charts.py` ⭐  
**Benchmark visualization** - creates charts from benchmark artifacts.

**Usage:**
```bash
# Generate all charts
uv run python scripts/benchmarks/charts.py

# Charts are generated from the latest benchmark result files
```

**Outputs:**
- Performance comparison charts
- Scaling analysis graphs
- High-resolution PNG/SVG files

### `compare.py`
**Detailed pytest comparison** - validates compatibility and performance.

**Usage:**
```bash
# Compare with local tests
uv run python scripts/benchmarks/compare.py tests/

# Compare specific features
uv run python scripts/benchmarks/compare.py pytest-compat-suite/features/fixtures/
```

## 🔧 Development Scripts (`development/`)

### `compatibility_report.py`
**Compatibility suite reporter** - runs selected `pytest-compat-suite/`
directories through Fastest and writes a machine-readable JSON report.

**Usage:**
```bash
make compat-report COMPAT_SUITES="core/basic features/fixtures"
make compat-report-all

uv run python scripts/development/compatibility_report.py \
  --fastest-binary target/debug/fastest \
  --json-output target/compatibility-report.json \
  core/basic features/fixtures
```

### `install-dev.sh`
**Local development installation** - builds and installs Fastest for testing.

**Usage:**
```bash
./scripts/development/install-dev.sh
```

### `install-dev-tools.sh`
**Development environment setup** - installs all required tools.

**Usage:**
```bash
./scripts/development/install-dev-tools.sh
```

### `setup-hooks.sh`
**Git hooks configuration** - sets up pre-commit and pre-push hooks.

**Usage:**
```bash
./scripts/development/setup-hooks.sh
```

### `pre-push-check.sh`
**Pre-push validation** - runs tests and checks before pushing.

**Usage:**
```bash
./scripts/development/pre-push-check.sh
```

## 🚀 Release Scripts (`release/`)

### `bump-version.sh`
**Version management** - updates version across all project files.

**Usage:**
```bash
# Bump to specific version
./scripts/release/bump-version.sh 1.0.0

# Show current version
./scripts/release/bump-version.sh
```

### `build-macos.sh`
**macOS release builds** - creates distribution binaries.

**Usage:**
```bash
./scripts/release/build-macos.sh [version]
```

### `update-manifest.sh`
**Version manifest** - updates version tracking files.

**Usage:**
```bash
./scripts/release/update-manifest.sh
```

## 🔧 Utility Scripts (`utils/`)

### `setup_test_repos.sh`
**Test repository setup** - clones real-world projects for testing.

**Usage:**
```bash
./scripts/utils/setup_test_repos.sh
```

## 🏃 Quick Start

### Running Benchmarks
```bash
# Benchmark artifact generation
PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo build --release
uv run python scripts/benchmarks/official.py

# Generate charts
uv run python scripts/benchmarks/charts.py

# Compare with pytest
uv run python scripts/benchmarks/compare.py
```

### Development Setup
```bash
# Install dev tools
./scripts/development/install-dev-tools.sh

# Set up git hooks
./scripts/development/setup-hooks.sh

# Install local build
./scripts/development/install-dev.sh
```

### Release Process
```bash
# Bump version
./scripts/release/bump-version.sh 1.0.0

# Build release
./scripts/release/build-macos.sh

# Update manifest
./scripts/release/update-manifest.sh
```

## 📋 Note on Test Scripts

Feature test scripts have been moved to the main test directory:
- `test_markers.py` → `tests/integration/`
- `test_parametrization.py` → `tests/integration/`
- `test_plugins.py` → `tests/integration/`

## 🔄 CI/CD Integration

```yaml
# GitHub Actions example
- name: Run Benchmarks
  run: |
    PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo build --release
    uv run python scripts/benchmarks/official.py --quick --output-dir target/benchmark-artifacts/quick
    uv run python scripts/benchmarks/charts.py
```

## 📁 Output Locations

- **Benchmark Results**: `target/benchmark-artifacts/quick/`
- **Charts**: `docs/images/` directory
- **Documentation**: `docs/performance/` directory

---

*Scripts are organized for clarity and maintainability. Each subdirectory has a specific purpose.*
