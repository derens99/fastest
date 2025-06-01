# Official Fastest Performance Benchmark Results

**Generated:** 2025-06-01T05:53:39.929008+00:00
**System:** macOS-15.1.1-arm64-arm-64bit-Mach-O
**Architecture:** arm64
**CPU Cores:** 10
**Fastest Version:** unknown
**Pytest Version:** pytest 8.3.5

## Executive Summary

- **Average Total Speedup:** 5.7x faster than pytest
- **Maximum Total Speedup:** 13.3x faster than pytest
- **Average Discovery Speedup:** 18.7x faster test discovery
- **Maximum Discovery Speedup:** 39.3x faster test discovery
- **Test Suite Sizes Tested:** 8 different sizes

## Detailed Results

| Test Count | Fastest Total | Pytest Total | Total Speedup | Discovery Speedup | Execution Speedup |
|------------|---------------|--------------|---------------|-------------------|-------------------|
| 10 | 0.133s | 0.271s | **2.0x** | 8.7x | 1.3x |
| 20 | 0.160s | 0.317s | **2.0x** | 11.7x | 1.3x |
| 50 | 0.114s | 0.390s | **3.4x** | 12.3x | 2.5x |
| 100 | 0.120s | 0.322s | **2.7x** | 13.5x | 1.6x |
| 200 | 0.129s | 0.407s | **3.1x** | 15.7x | 1.9x |
| 500 | 0.080s | 0.679s | **8.5x** | 21.0x | 6.0x |
| 1,000 | 0.108s | 1.144s | **10.6x** | 27.4x | 7.5x |
| 2,000 | 0.159s | 2.121s | **13.3x** | 39.3x | 9.6x |

## Performance Analysis

### Test Discovery Performance

Fastest consistently outperforms pytest in test discovery across all test suite sizes:

- **Small suites (≤100 tests):** 10.9x faster average
- **Large suites (>500 tests):** 33.3x faster average

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