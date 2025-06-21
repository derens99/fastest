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
**The definitive performance benchmark** - generates official results for publication.

**Usage:**
```bash
# Full benchmark suite (for releases)
python scripts/benchmarks/official.py

# Quick benchmark (for development)
python scripts/benchmarks/official.py --quick

# Custom output directory
python scripts/benchmarks/official.py --output-dir results/
```

**Features:**
- Multiple test suite sizes (10-2000 tests)
- Realistic test patterns
- Detailed timing breakdowns
- System information capture
- Publication-ready reports

### `charts.py` ⭐  
**Professional visualization** - creates publication-ready performance charts.

**Usage:**
```bash
# Generate all charts
python scripts/benchmarks/charts.py

# Specific chart type
python scripts/benchmarks/charts.py --type speedup
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
python scripts/benchmarks/compare.py --test-dir tests/

# Compare specific features
python scripts/benchmarks/compare.py --features fixtures,markers
```

## 🔧 Development Scripts (`development/`)

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

### `setup-test-repos.sh`
**Test repository setup** - clones real-world projects for testing.

**Usage:**
```bash
./scripts/utils/setup-test-repos.sh
```

## 🏃 Quick Start

### Running Benchmarks
```bash
# Official benchmark
cargo build --release
python scripts/benchmarks/official.py

# Generate charts
python scripts/benchmarks/charts.py

# Compare with pytest
python scripts/benchmarks/compare.py
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
    cargo build --release
    python scripts/benchmarks/official.py --quick
    python scripts/benchmarks/charts.py
```

## 📁 Output Locations

- **Benchmark Results**: `benchmarks/` directory
- **Charts**: `docs/images/` directory
- **Documentation**: `docs/performance/` directory

---

*Scripts are organized for clarity and maintainability. Each subdirectory has a specific purpose.*