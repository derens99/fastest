# Official Fastest Performance Benchmark Results

**Generated:** 2025-06-01T05:21:40.601247+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 5.9x faster than pytest
- **Maximum Total Speedup:** 13.2x faster than pytest
- **Average Discovery Speedup:** 19.0x faster test discovery
- **Maximum Discovery Speedup:** 39.5x faster test discovery
- **Test Suite Sizes Tested:** 8 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.110s | 0.265s | **2.4x** | 8.9x | 1.6x |
| 20 | 0.115s | 0.312s | **2.7x** | 11.8x | 1.8x |
| 50 | 0.121s | 0.377s | **3.1x** | 11.6x | 2.2x |
| 100 | 0.119s | 0.317s | **2.7x** | 13.4x | 1.6x |
| 200 | 0.124s | 0.402s | **3.3x** | 16.1x | 2.0x |
| 500 | 0.080s | 0.678s | **8.5x** | 21.8x | 5.9x |
| 1,000 | 0.106s | 1.151s | **10.9x** | 28.7x | 7.8x |
| 2,000 | 0.160s | 2.121s | **13.2x** | 39.5x | 9.5x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 10.8x faster average
- **Large suites (>500 tests):** 34.1x faster average

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