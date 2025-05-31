# Fastest Benchmarks

This directory contains performance benchmarks comparing `fastest` with `pytest` and other test runners.

## Quick Start

```bash
# Main comprehensive benchmark (recommended)
python benchmarks/main_benchmark.py

# Quick demo benchmark
python benchmarks/quick_benchmark_demo.py

# Simple benchmark
python benchmarks/simple_benchmark.py
```

## Benchmark Scripts

### Core Benchmarks

- **`main_benchmark.py`** - Comprehensive benchmark suite comparing fastest vs pytest across multiple test suite sizes. This is the primary benchmark that validates performance claims.

- **`simple_benchmark.py`** - Basic performance comparison focused on test discovery and execution speed.

- **`quick_benchmark_demo.py`** - Lightweight benchmark for quick demonstrations and CI.

### Specialized Benchmarks

- **`bench_discovery.py`** - Focused test discovery performance analysis.

- **`benchmark_cache.py`** - Tests caching mechanisms and incremental test runs.

- **`benchmark_parsers.py`** - Compares different parsing strategies and their performance impact.

- **`benchmark_scale.py`** - Tests performance scaling from small to large test suites.

- **`validate_performance_claims.py`** - Validates specific performance claims made in documentation.

### Utility Scripts

- **`generate_scale_tests.py`** - Generates test suites of various sizes for benchmarking.

## Benchmark Results

Current benchmark results are documented in:

- **`BENCHMARK_RESULTS.md`** - Latest comprehensive performance comparison
- **`latest_benchmark_results.json`** - Raw JSON results from latest run

## Performance Summary

Based on latest benchmarks:

### Test Discovery
- **10-20 tests**: 11-16x faster than pytest
- **50-100 tests**: 12-29x faster than pytest  
- **500+ tests**: 19-141x faster than pytest

### Test Execution
Performance varies by execution strategy:
- **InProcess** (≤20 tests): 2-3x faster than pytest
- **WarmWorkers** (21-100 tests): Similar to pytest
- **FullParallel** (>100 tests): 1.5-3x faster than pytest

## Running Benchmarks

### Prerequisites

1. Build fastest in release mode:
   ```bash
   cargo build --release
   ```

2. Ensure pytest is installed:
   ```bash
   pip install pytest
   ```

### Interpreting Results

- **Discovery times** measure how long it takes to find and parse tests
- **Execution times** measure actual test running including setup/teardown
- **Speedup factors** show how many times faster fastest is compared to pytest

### Benchmark Environment

For consistent results:
- Run on a dedicated machine or during low activity
- Use release builds (`cargo build --release`)
- Run multiple iterations (benchmarks do this automatically)
- Avoid running other intensive processes during benchmarking

## Contributing

When adding new benchmarks:

1. Follow the naming convention: `benchmark_<feature>.py`
2. Include comprehensive error handling
3. Support multiple runs for statistical significance
4. Document what the benchmark measures
5. Update this README with the new benchmark description