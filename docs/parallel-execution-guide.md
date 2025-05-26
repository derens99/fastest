# Parallel Test Execution Guide

Fastest supports running tests in parallel to maximize performance on multi-core systems. This guide explains how to use parallel execution effectively.

## Quick Start

```bash
# Auto-detect optimal number of workers based on CPU cores
fastest -n 0

# Use specific number of workers
fastest -n 4

# Run tests in specific directory with 8 parallel workers
fastest tests/ -n 8
```

## How It Works

Fastest uses a work-stealing thread pool to distribute tests across multiple workers:

1. **Smart Grouping**: Tests are grouped by module to minimize import overhead
2. **Work Stealing**: Idle workers automatically pick up work from busy workers
3. **Process Isolation**: Each worker runs tests in separate Python processes for safety

## Choosing Worker Count

### Auto-Detection (Recommended)
```bash
fastest -n 0
```
Fastest automatically detects the optimal number of workers based on:
- Number of CPU cores
- Available system memory
- Total number of tests

### Manual Configuration
```bash
# Conservative - half the CPU cores
fastest -n 2

# Balanced - equal to CPU cores
fastest -n 4

# Aggressive - more workers than cores (useful for I/O-heavy tests)
fastest -n 8
```

## Performance Guidelines

### When Parallel Execution Helps

✅ **Best for:**
- Large test suites (100+ tests)
- CPU-bound tests
- Tests with independent fixtures
- CI/CD environments with multiple cores

### When to Use Sequential Execution

❌ **Avoid parallel for:**
- Small test suites (<50 tests) - overhead may exceed benefits
- Tests that share global state
- Tests with specific ordering requirements
- Debugging test failures

## Real-World Examples

### Development Workflow
```bash
# Quick feedback during development - run changed tests only
fastest tests/unit/ -n 4 -k "test_new_feature"

# Full test suite before commit
fastest -n 0
```

### CI/CD Integration
```yaml
# GitHub Actions example
- name: Run tests
  run: |
    # Use all available cores in CI
    fastest -n 0 -v
    
# Or with specific worker count
- name: Run tests (4 workers)
  run: fastest -n 4
```

### Different Test Types
```bash
# Unit tests - maximum parallelism
fastest tests/unit/ -n 0

# Integration tests - moderate parallelism to avoid resource contention
fastest tests/integration/ -n 2

# End-to-end tests - sequential to avoid conflicts
fastest tests/e2e/
```

## Performance Benchmarks

Based on real-world test suites:

| Test Suite Size | Sequential | Parallel (4 workers) | Speedup |
|----------------|------------|---------------------|---------|
| 100 tests      | 1.87s      | 0.89s               | 2.1x    |
| 500 tests      | 9.35s      | 3.12s               | 3.0x    |
| 1000 tests     | 18.72s     | 5.24s               | 3.6x    |

## Troubleshooting

### Tests Failing in Parallel but Passing Sequentially

This usually indicates shared state between tests. Solutions:

1. **Identify problematic tests**:
   ```bash
   # Run tests sequentially to confirm they pass
   fastest
   
   # Run with minimal parallelism to isolate issues
   fastest -n 2 -v
   ```

2. **Fix test isolation**:
   - Use fixtures for setup/teardown
   - Avoid global variables
   - Use unique temporary files/databases

### Performance Not Improving

1. **Check CPU usage**:
   ```bash
   # Monitor during test run
   top  # or htop
   ```

2. **Reduce worker count if:**
   - CPU usage is already at 100%
   - Tests are I/O bound
   - System is memory constrained

3. **Consider test characteristics**:
   ```bash
   # Group similar tests together
   fastest tests/unit/ -n 4      # CPU-bound tests
   fastest tests/db/ -n 2        # I/O-bound tests
   ```

### Memory Issues

For memory-intensive tests:

```bash
# Reduce parallelism
fastest -n 2

# Or run specific test groups separately
fastest tests/memory_intensive/ -n 1
fastest tests/normal/ -n 4
```

## Best Practices

### 1. Start with Auto-Detection
```bash
fastest -n 0
```
Let Fastest determine the optimal configuration for your system.

### 2. Profile Before Optimizing
```bash
# Time sequential execution
time fastest

# Compare with parallel
time fastest -n 0
```

### 3. Use Markers for Better Control
```python
@fastest.mark.serial  # Coming soon: force sequential execution
def test_database_migration():
    # This test must run alone
    pass

@fastest.mark.parallel_safe
def test_pure_calculation():
    # This test is safe for parallel execution
    pass
```

### 4. CI/CD Optimization
```yaml
# Run different test types with appropriate parallelism
- run: fastest tests/unit/ -n 0 -m "not slow"
- run: fastest tests/integration/ -n 2
- run: fastest tests/e2e/ -n 1
```

## Comparison with pytest-xdist

For users familiar with pytest-xdist:

| Feature | pytest-xdist | Fastest |
|---------|--------------|---------|
| Basic parallel | `-n 4` | `-n 4` |
| Auto-detect | `-n auto` | `-n 0` |
| Load balancing | Yes | Yes (work-stealing) |
| Process isolation | Yes | Yes |
| Startup time | ~1-2s | <100ms |
| Memory usage | Higher | ~50% less |

## Advanced Configuration

### Environment Variables
```bash
# Override worker count
FASTEST_WORKERS=4 fastest

# Disable parallel execution
FASTEST_WORKERS=1 fastest
```

### Python API
```python
import fastest

# Discover tests
tests = fastest.discover_tests("tests/")

# Run with specific worker count
results = fastest.run_tests_parallel(tests, num_workers=4)

# Auto-detect workers
results = fastest.run_tests_parallel(tests, num_workers=None)
```

## Future Enhancements

Coming soon:
- Test timing history for better work distribution
- `--fail-fast` mode to stop on first failure
- Memory-aware worker scaling
- Distributed testing across machines

For the latest updates, see the [Roadmap](ROADMAP.md). 