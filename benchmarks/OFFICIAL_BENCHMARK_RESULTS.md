# Official Fastest Performance Benchmark Results

**Generated:** 2025-06-01T04:33:22.053901+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 5.9x faster than pytest
- **Maximum Total Speedup:** 13.0x faster than pytest
- **Average Discovery Speedup:** 18.7x faster test discovery
- **Maximum Discovery Speedup:** 36.7x faster test discovery
- **Test Suite Sizes Tested:** 8 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.063s | 0.231s | **3.6x** | 9.4x | 2.3x |
| 20 | 0.070s | 0.253s | **3.6x** | 11.3x | 2.2x |
| 50 | 0.105s | 0.297s | **2.8x** | 12.4x | 1.8x |
| 100 | 0.159s | 0.311s | **2.0x** | 13.3x | 1.1x |
| 200 | 0.128s | 0.397s | **3.1x** | 15.5x | 1.9x |
| 500 | 0.077s | 0.661s | **8.5x** | 22.4x | 5.9x |
| 1,000 | 0.106s | 1.102s | **10.4x** | 28.9x | 7.4x |
| 2,000 | 0.158s | 2.061s | **13.0x** | 36.7x | 9.5x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 11.0x faster average
- **Large suites (>500 tests):** 32.8x faster average

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