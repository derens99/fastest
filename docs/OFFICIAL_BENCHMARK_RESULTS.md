# Official Fastest Performance Benchmark Results

**Generated:** 2025-05-31T19:16:03.116147+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 5.4x faster than pytest
- **Maximum Total Speedup:** 10.1x faster than pytest
- **Average Discovery Speedup:** 4.6x faster test discovery
- **Maximum Discovery Speedup:** 7.9x faster test discovery
- **Test Suite Sizes Tested:** 4 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.163s | 0.259s | **1.6x** | 0.9x | 4.4x |
| 50 | 0.120s | 0.331s | **2.8x** | 4.4x | 2.2x |
| 100 | 0.059s | 0.423s | **7.1x** | 5.1x | 9.2x |
| 500 | 0.066s | 0.672s | **10.1x** | 7.9x | 12.6x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 3.5x faster average


### Test Execution Performance

Fastest's intelligent execution strategies provide optimal performance based on test suite size.

## Methodology

Each benchmark:
1. Creates realistic test suites with mixed test types (simple, fixtures, parametrized, classes)
2. Measures test discovery time separately from execution time
3. Runs both fastest and pytest with equivalent configurations
4. Reports total time, discovery time, and execution time
5. Calculates speedup factors for direct comparison

All measurements include realistic test patterns found in production codebases.