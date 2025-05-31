# Scripts Directory

This directory contains utility scripts for building, testing, and maintaining the Fastest project.

## 🚀 Release & Build Scripts

### `build-macos-release.sh`
Builds macOS release binaries for distribution.

**Usage:**
```bash
./scripts/build-macos-release.sh [version]
```

### `bump-version.sh`
Updates version numbers across all project files (Cargo.toml, README, etc.).

**Usage:**
```bash
./scripts/bump-version.sh 0.3.0
```

### `install-dev.sh`
Installs Fastest from local build for development testing.

**Usage:**
```bash
./scripts/install-dev.sh
```

## 📊 Benchmarking & Performance

### `official_benchmark.py` ⭐
**The definitive performance benchmark** that generates official results for publication.

Creates comprehensive performance comparisons between Fastest and pytest across multiple test suite sizes, measuring both discovery and execution performance.

**Usage:**
```bash
# Full benchmark suite (recommended for official results)
python scripts/official_benchmark.py

# Quick benchmark for development
python scripts/official_benchmark.py --quick

# Custom output directory
python scripts/official_benchmark.py --output-dir custom_results/
```

**Outputs:**
- `benchmarks/official_results.json` - Machine-readable results
- `benchmarks/OFFICIAL_BENCHMARK_RESULTS.md` - Human-readable report  
- `docs/OFFICIAL_BENCHMARK_RESULTS.md` - Published documentation

**Features:**
- Tests multiple suite sizes (10, 20, 50, 100, 200, 500, 1000, 2000 tests)
- Realistic test patterns (simple, fixtures, parametrized, classes)
- Separate discovery and execution timing
- Memory usage measurement
- System information capture
- Automatic speedup calculations
- Publication-ready markdown reports

### `compare_with_pytest.py`
Detailed comparison tool for development and validation.

**Usage:**
```bash
python scripts/compare_with_pytest.py --test-dir tests/
```

## 🔧 Development Utilities

### `setup_test_repos.sh`
Sets up real-world test repositories for validation and testing.

**Usage:**
```bash
./scripts/setup_test_repos.sh
```

## 📋 Running Benchmarks

### For Official Results (CI/Release)
```bash
# Ensure release build exists
cargo build --release

# Run official benchmark
python scripts/official_benchmark.py
```

### For Development Testing
```bash
# Quick benchmark during development
python scripts/official_benchmark.py --quick

# Detailed comparison
python scripts/compare_with_pytest.py
```

## 📁 Output Locations

- **Official Results**: `benchmarks/` and `docs/` (for publication)
- **Development Results**: `comparison_results/` (gitignored)
- **Performance Data**: `performance_data/` (gitignored)

## 🔄 Integration with CI/CD

The `official_benchmark.py` script is designed to be run in CI/CD pipelines:

```yaml
# Example GitHub Actions step
- name: Run Official Benchmark
  run: |
    cargo build --release
    python scripts/official_benchmark.py --quick
    git add docs/OFFICIAL_BENCHMARK_RESULTS.md
    git commit -m "Update benchmark results"
```

## 📖 Documentation

Benchmark results are automatically published to:
- Repository documentation in `docs/`
- README.md performance section (manual update)
- GitHub releases (attach JSON results)

For more details on benchmarking methodology, see [benchmarks/README.md](../benchmarks/README.md).