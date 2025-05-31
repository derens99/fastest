# Fastest vs pytest Performance Validation

**Generated**: 2025-05-30 00:08:39

## Executive Summary


- **Overall Performance**: 0.48x faster than pytest
- **Success Rate**: 0/4 scenarios faster
- **Strategy Validation**: All execution strategies tested

## Execution Strategy Performance


### InProcess Strategy
- **Mean Speedup**: 0.62x
- **Range**: 0.4x - 0.8x


### WarmWorkers Strategy
- **Mean Speedup**: 0.49x
- **Range**: 0.4x - 0.6x


### FullParallel Strategy
- **Mean Speedup**: 0.98x
- **Range**: 0.6x - 1.5x


## Detailed Results

### Real-World Scenarios

| Scenario | Test Count | pytest Time | fastest Time | Speedup |
|----------|------------|-------------|--------------|---------|
| unit_tests | 45 | 0.161s | 0.304s | 0.53x |
| integration | 25 | 0.159s | 0.355s | 0.45x |
| api_tests | 30 | 0.130s | 0.355s | 0.37x |
| large_suite | 180 | 0.211s | 0.372s | 0.57x |


### Strategy Breakdown

| Strategy | Test Count | Complexity | pytest Time | fastest Time | Speedup | Status |
|----------|------------|------------|-------------|--------------|---------|--------|
| InProcess | 8 | simple | 0.135s | 0.160s | 0.84x | ❌ |
| InProcess | 15 | fixtures | 0.139s | 0.348s | 0.40x | ❌ |
| WarmWorkers | 35 | simple | 0.145s | 0.302s | 0.48x | ❌ |
| WarmWorkers | 50 | fixtures | 0.200s | 0.357s | 0.56x | ❌ |
| WarmWorkers | 75 | parametrized | 0.150s | 0.351s | 0.43x | ❌ |
| FullParallel | 150 | simple | 0.254s | 0.319s | 0.80x | ❌ |
| FullParallel | 200 | classes | 0.221s | 0.360s | 0.61x | ❌ |
| FullParallel | 300 | fixtures | 0.651s | 0.426s | 1.53x | ✅ |
