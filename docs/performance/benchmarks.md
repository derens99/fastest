# Performance Benchmarks

Fastest is designed to become a high-performance pytest-style runner, but the
current project policy is evidence first: do not quote fixed speedups unless
they come from a fresh benchmark artifact generated from this checkout.

## Current Status

The compatibility-first execution path is the verified path today. The full
compatibility report currently passes every discovered compatibility category,
including plugins, but that does not prove the older threshold-based performance
claims.

Use these commands as the current source of truth:

```bash
# Accepted local correctness gate
make verify

# Full compatibility baseline
make compat-report-all

# Quick benchmark harness
make bench

# Compare Fastest and pytest on a chosen suite
make compare TEST_DIR=pytest-compat-suite/core/basic
```

Generated quick benchmark reports are written under
`target/benchmark-artifacts/quick/benchmark_results.{json,md}`. Treat those
artifacts, not this page, as the benchmark record.

## What Is Verified

- Rust and Python project gates pass through `make verify`.
- `make compat-report-all` produces `target/compatibility-report-all.json`.
- Every discovered compatibility category has a passing Fastest summary in the
  current report.
- The current integration tests document that suite sizes use the
  compatibility-first `UltraInProcess` path.
- `make bench` produces a quick benchmark artifact with hardware, Python,
  pytest, command-line, and cache-context metadata.

## What Is Not Yet Revalidated

Older documentation claimed fixed numbers such as multi-x speedups, thousands
of tests per second, and automatic `InProcess` / `HybridBurst` / `WorkStealing`
strategy selection. Those claims are not currently treated as product claims.

Before publishing performance numbers again, regenerate and archive:

- The exact Fastest binary and commit.
- Python and pytest versions.
- Hardware and operating-system details.
- Warm and cold cache behavior.
- Separate discovery, execution, and total wall-clock times.
- Raw command lines and raw output artifacts.

## Benchmark Methodology

Use the same input suite for pytest and Fastest, run each command multiple
times, and report both raw timings and summary statistics. Avoid mixing
compatibility work and benchmark claims in one result: correctness gates should
pass before speedups are advertised.

Recommended workflow:

```bash
make verify
make compat-report-all
make compare TEST_DIR=pytest-compat-suite/core/basic COMPARISON_RUNS=5
make bench
```

## Performance Roadmap

1. Keep the compatibility report green.
2. Rebuild benchmark fixtures around verified compatibility categories.
3. Revalidate strategy selection behavior with tests and raw timing artifacts.
4. Publish benchmark numbers only when the artifact and methodology are checked
   into the release evidence.

Until then, describe Fastest as performance-oriented and Rust-backed, not as a
runner with a fixed verified speedup.
