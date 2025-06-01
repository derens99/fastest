# Official Fastest Performance Benchmark Results

**Generated:** 2025-06-01T04:59:42.208683+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 5.6x faster than pytest
- **Maximum Total Speedup:** 13.1x faster than pytest
- **Average Discovery Speedup:** 18.5x faster test discovery
- **Maximum Discovery Speedup:** 39.0x faster test discovery
- **Test Suite Sizes Tested:** 8 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.107s | 0.230s | **2.1x** | 8.9x | 1.3x |
| 20 | 0.111s | 0.250s | **2.2x** | 11.0x | 1.3x |
| 50 | 0.112s | 0.301s | **2.7x** | 12.1x | 1.7x |
| 100 | 0.115s | 0.307s | **2.7x** | 12.3x | 1.6x |
| 200 | 0.123s | 0.396s | **3.2x** | 14.8x | 2.0x |
| 500 | 0.078s | 0.653s | **8.4x** | 21.0x | 5.9x |
| 1,000 | 0.105s | 1.121s | **10.7x** | 28.9x | 7.6x |
| 2,000 | 0.158s | 2.063s | **13.1x** | 39.0x | 9.4x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 10.7x faster average
- **Large suites (>500 tests):** 33.9x faster average

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