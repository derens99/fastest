# Official Fastest Performance Benchmark Results

**Generated:** 2025-06-01T19:26:02.552145+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 5.8x faster than pytest
- **Maximum Total Speedup:** 13.1x faster than pytest
- **Average Discovery Speedup:** 18.9x faster test discovery
- **Maximum Discovery Speedup:** 40.7x faster test discovery
- **Test Suite Sizes Tested:** 8 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.109s | 0.261s | **2.4x** | 8.8x | 1.5x |
| 20 | 0.110s | 0.309s | **2.8x** | 11.5x | 1.9x |
| 50 | 0.119s | 0.391s | **3.3x** | 11.5x | 2.4x |
| 100 | 0.115s | 0.329s | **2.9x** | 13.2x | 1.8x |
| 200 | 0.123s | 0.397s | **3.2x** | 15.8x | 2.0x |
| 500 | 0.080s | 0.660s | **8.3x** | 21.5x | 5.8x |
| 1,000 | 0.107s | 1.105s | **10.3x** | 28.1x | 7.3x |
| 2,000 | 0.159s | 2.071s | **13.1x** | 40.7x | 9.4x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 10.6x faster average
- **Large suites (>500 tests):** 34.4x faster average

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