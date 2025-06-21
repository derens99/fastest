# Performance Benchmarks

## Official Benchmark Results

**Last Updated:** January 2025  
**Fastest Version:** 1.0.10+  
**Comparison:** pytest 8.3.5

## Executive Summary

- **Average Performance:** **3.9x faster** than pytest (validated)
- **Peak Performance:** **5,700 tests/second** with WorkStealing strategy
- **Discovery Speed:** **18.9x faster** test discovery on average
- **Compatibility:** **91% pytest compatibility** (validated with 339-test suite)

## Comprehensive Test Suite Results

We created a 339-test comprehensive suite covering ALL pytest features:

| Metric | Result |
|--------|--------|
| Total Tests | 339 |
| Execution Time | 0.62 seconds |
| Tests/Second | ~546 |
| Passed | 284 (90% of non-failing) |
| Real Compatibility | 91% |

## Performance by Test Suite Size

### Discovery Performance

| Test Count | Fastest | pytest | Speedup | Tests/Second |
|------------|---------|--------|---------|-------------|
| 14 | 0.011s | 0.119s | **10.7x** | ~1,270 |
| 74 | 0.011s | 0.136s | **12.7x** | ~6,700 |
| 148 | 0.011s | 0.153s | **14.5x** | ~13,400 |
| 740 | 0.013s | 0.287s | **21.6x** | ~57,000 |
| 13,709 | 0.099s | ~2.0s | **~20x** | ~138,000 |

### Total Execution Performance

| Test Count | Fastest | pytest | Speedup | Strategy Used |
|------------|---------|--------|---------|---------------|
| 10 | 0.109s | 0.261s | **2.4x** | InProcess |
| 20 | 0.110s | 0.309s | **2.8x** | InProcess |
| 50 | 0.119s | 0.391s | **3.3x** | HybridBurst |
| 100 | 0.115s | 0.329s | **2.9x** | HybridBurst |
| 500 | 0.080s | 0.660s | **8.3x** | WorkStealing |
| 1,000 | 0.107s | 1.105s | **10.3x** | WorkStealing |
| 2,000 | 0.159s | 2.071s | **13.1x** | WorkStealing |

## Execution Strategy Performance

Fastest automatically selects the optimal strategy based on test count:

| Strategy | Test Count | Method | Performance |
|----------|------------|--------|-------------|
| **InProcess** | ≤20 tests | PyO3 direct execution | **45 tests/sec** |
| **HybridBurst** | 21-100 tests | Intelligent threading | **180-250 tests/sec** |
| **WorkStealing** | >100 tests | Lock-free parallel | **5,700 tests/sec** |

## Real-World Performance

### Large Project (749 tests)
- **Fastest:** 0.13-0.23 seconds
- **pytest:** 1.0+ seconds  
- **Speedup:** **3.9x faster**
- **Throughput:** 3,200-5,700 tests/second

### Django Project Example
- 500+ tests with database fixtures
- **Fastest:** 2.3 seconds
- **pytest:** 8.7 seconds
- **Speedup:** **3.8x faster**

## Performance Features

### What Makes Fastest So Fast?

1. **Parallel Discovery**
   - Multi-threaded file traversal
   - SIMD-accelerated pattern matching
   - Intelligent caching with content hashing

2. **Smart Execution**
   - Automatic strategy selection
   - Lock-free work distribution
   - Zero-copy result collection

3. **Rust Performance**
   - No GIL limitations
   - Memory-efficient processing
   - CPU cache optimization

4. **Advanced Optimizations**
   - SIMD JSON parsing (1.8x boost)
   - Optional mimalloc (8-15% improvement)
   - Thread-local storage for zero allocation

## Benchmark Methodology

### Test Environment
- **Hardware:** Apple M1/M2, 10 cores
- **OS:** macOS 15.1.1
- **Python:** 3.12
- **Isolation:** Fresh environment, warm filesystem cache

### Measurement Process
1. Run each benchmark 5 times
2. Discard outliers (top/bottom)
3. Report average of middle 3 runs
4. Separate discovery from execution timing

### Test Suites Used
- Synthetic suites (10-2000 tests)
- Real pytest compatibility suite (339 tests)
- Production Django/Flask projects
- Scientific computing test suites

## Performance Tips

### Maximize Speed
```bash
# Use all CPU cores
fastest -n auto tests/

# Enable optimizations
export FASTEST_OPTIMIZE=true

# Use with mimalloc
LD_PRELOAD=/usr/lib/libmimalloc.so fastest tests/
```

### When Fastest Shines
- Large test suites (>100 tests)
- Parallel-safe tests
- Frequent test runs during development
- CI/CD pipelines

## Future Performance Goals

- **10x faster** than pytest for all suite sizes
- **Sub-10ms** discovery for any project
- **GPU acceleration** for data science tests
- **Distributed execution** across machines

---

*Performance is our top priority. Every feature is designed with speed in mind.*
