# Fastest Benchmarks

This directory contains performance benchmarks comparing `fastest` with `pytest`.

## Quick Start

```bash
# Build fastest in release mode first
cargo build --release

# Run the unified benchmark suite
python benchmarks/unified_comprehensive_benchmark.py

# Run benchmarks via the runner script
python benchmarks/run_benchmarks.py

# Generate graphs from results
python benchmarks/generate_graphs.py
```

## Benchmark Scripts

- **`unified_comprehensive_benchmark.py`** — Comprehensive benchmark suite comparing fastest vs pytest across multiple test suite sizes. This is the primary benchmark.

- **`run_benchmarks.py`** — Runner script for executing benchmarks with standard options.

- **`generate_graphs.py`** — Generates comparison graphs from benchmark results in `results/`.

## Results

Benchmark output is stored in `results/`:
- `results.json` — Raw benchmark data
- `*.png` — Generated comparison graphs

## Prerequisites

1. Build fastest in release mode:
   ```bash
   cargo build --release
   ```

2. Ensure pytest is installed:
   ```bash
   pip install pytest
   ```

## Tips for Consistent Results

- Use release builds (`cargo build --release`)
- Run on a quiet machine with minimal background activity
- Run multiple iterations (the benchmarks do this automatically)
- Compare results across multiple runs before drawing conclusions
