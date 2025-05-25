# Benchmarks

This directory contains various performance benchmarks comparing `fastest` with `pytest`.

## Benchmark Scripts

### benchmark.py
Main performance benchmark comparing pytest vs fastest for both test discovery and execution. Creates a test suite of 100 tests and measures:
- Test discovery performance (5 runs average)
- Test execution performance
- Overall speedup comparison

### benchmark_v2.py
Demonstrates the performance improvements in fastest v2 with batch execution support. Compares:
- Old method: Individual subprocess per test
- New method: Batch execution
- Pytest execution

### benchmark_detailed.py
Provides detailed performance analysis including:
- Subprocess overhead measurement
- Direct import vs subprocess execution comparison
- Performance insights and optimization opportunities

### benchmark_cache.py
Measures the performance impact of the discovery cache feature:
- Cold cache vs warm cache performance
- Comparison with and without cache
- Large project benchmarking (1000 tests)

## Performance Reports

### performance_report.md
Results from the main benchmark showing typical performance characteristics.

### performance_report_v2.md
Results from the v2 benchmark showing batch execution improvements.

## Running Benchmarks

To run any benchmark:

```bash
python benchmarks/benchmark.py
python benchmarks/benchmark_v2.py
# etc.
```

Make sure `fastest` is installed first:
```bash
maturin develop
``` 