# Fastest Benchmark Artifacts

This directory is for benchmark inputs, saved outputs, and ad hoc benchmark artifacts. The maintained benchmark runners live in `scripts/benchmarks/`.

## Current Tools

Run benchmark tools from the repository root:

```bash
# Benchmark artifact generator for release review
uv run python scripts/benchmarks/official.py

# Faster development run
uv run python scripts/benchmarks/official.py --quick --output-dir target/benchmark-artifacts/quick

# Compare Fastest against pytest on a selected suite
uv run python scripts/benchmarks/compare.py pytest-compat-suite/core/basic --fastest-binary ./target/release/fastest

# Generate charts from benchmark outputs
uv run python scripts/benchmarks/charts.py
```

## Local Artifact Script

- `unified_comprehensive_benchmark.py` is a standalone benchmark harness kept here as an artifact-oriented runner. Prefer `scripts/benchmarks/official.py` for maintained benchmark artifacts.

## Expected Output Locations

- Raw comparison data: `comparison_results/`
- Quick benchmark artifacts: `target/benchmark-artifacts/quick/`
- Generated charts: `docs/images/`
- Benchmark methodology documentation: `docs/performance/benchmarks.md`

## Prerequisites

```bash
PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo build --release
uv sync --extra dev
```

Keep new benchmark runners under `scripts/benchmarks/`. Keep generated data and one-off benchmark artifacts here or in an ignored output directory.
