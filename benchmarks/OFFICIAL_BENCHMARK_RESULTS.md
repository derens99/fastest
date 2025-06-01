# Official Fastest Performance Benchmark Results

**Generated:** 2025-06-01T05:31:48.334046+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 5.8x faster than pytest
- **Maximum Total Speedup:** 13.4x faster than pytest
- **Average Discovery Speedup:** 19.0x faster test discovery
- **Maximum Discovery Speedup:** 39.7x faster test discovery
- **Test Suite Sizes Tested:** 8 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.132s | 0.263s | **2.0x** | 9.2x | 1.2x |
| 20 | 0.112s | 0.301s | **2.7x** | 11.2x | 1.8x |
| 50 | 0.113s | 0.374s | **3.3x** | 12.0x | 2.4x |
| 100 | 0.118s | 0.329s | **2.8x** | 14.2x | 1.6x |
| 200 | 0.124s | 0.407s | **3.3x** | 15.3x | 2.0x |
| 500 | 0.079s | 0.696s | **8.8x** | 22.3x | 6.2x |
| 1,000 | 0.108s | 1.135s | **10.5x** | 28.3x | 7.5x |
| 2,000 | 0.159s | 2.125s | **13.4x** | 39.7x | 9.6x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 10.8x faster average
- **Large suites (>500 tests):** 34.0x faster average

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