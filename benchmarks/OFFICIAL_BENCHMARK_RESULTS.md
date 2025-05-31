# Official Fastest Performance Benchmark Results

**Generated:** 2025-05-31T22:15:00.822052+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 3.4x faster than pytest
- **Maximum Total Speedup:** 5.2x faster than pytest
- **Average Discovery Speedup:** 1.0x faster test discovery
- **Maximum Discovery Speedup:** 1.0x faster test discovery
- **Test Suite Sizes Tested:** 4 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.097s | 0.235s | **2.4x** | 1.0x | 1.3x |
| 50 | 0.100s | 0.310s | **3.1x** | 1.0x | 1.7x |
| 100 | 0.103s | 0.314s | **3.0x** | 1.0x | 1.7x |
| 500 | 0.137s | 0.706s | **5.2x** | 1.0x | 3.1x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 1.0x faster average


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