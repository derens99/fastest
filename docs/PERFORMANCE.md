# Fastest Performance Guide

## Overview

Fastest is designed to be the fastest Python test runner by leveraging Rust's performance and several optimization techniques. This guide explains the performance features and how to use them effectively.

## Performance Features

### 1. **Parallel Test Discovery**

Fastest uses parallel file traversal and parsing:
- **AST Parser**: Uses tree-sitter for fast, accurate parsing (default)
- **Regex Parser**: Lightweight alternative for simple test files
- **Discovery Cache**: Caches test discovery results based on file modification time

```bash
# Use AST parser (default, most accurate)
fastest tests/

# Use regex parser (faster for simple tests)
fastest --parser regex tests/

# Disable cache (useful for debugging)
fastest --no-cache tests/
```

### 2. **Optimized Test Execution**

Multiple execution strategies are available:

```bash
# Standard parallel execution (default)
fastest tests/

# Optimized executor with batching
fastest --optimizer optimized tests/

# Aggressive optimization (experimental)
fastest --optimizer aggressive tests/

# Control worker count
fastest -n 8 tests/  # Use 8 workers
fastest -n 1 tests/  # Sequential execution
```

### 3. **Incremental Testing**

Only run tests affected by recent changes:

```bash
# Run only affected tests (experimental)
fastest --incremental tests/
```

### 4. **Discovery Cache**

Test discovery results are cached to speed up subsequent runs:

- Cache location: `~/.cache/fastest/discovery_cache.json`
- Automatically invalidated when files change
- Disable with `--no-cache`

## Benchmarking

Compare performance with pytest:

```bash
python benchmarks/bench_discovery.py
```

## Performance Tips

### 1. **Optimal Worker Count**

- Default: 2x CPU cores (good for I/O-bound tests)
- CPU-bound tests: Use `-n` equal to CPU cores
- Many small tests: Use more workers
- Few large tests: Use fewer workers

### 2. **Test Organization**

- Group related tests in the same file
- Keep test files reasonably sized (100-500 tests)
- Use descriptive test names for better batching

### 3. **Fixture Optimization**

- Use function-scoped fixtures when possible
- Avoid expensive module/session fixtures
- Consider fixture caching for expensive operations

### 4. **Parallel-Safe Tests**

Ensure tests are parallel-safe:
- Don't share mutable state
- Use unique temporary files/directories
- Avoid hardcoded ports or resources

## Performance Metrics

Typical performance improvements over pytest:

| Scenario | Improvement | Notes |
|----------|-------------|-------|
| Discovery (small) | 5-10x | < 100 tests |
| Discovery (medium) | 10-50x | 100-1000 tests |
| Discovery (large) | 50-100x | > 1000 tests |
| Execution (parallel) | 2-5x | Depends on test characteristics |
| With cache | 100-1000x | Second+ runs |

## Troubleshooting

### Tests Running Slowly

1. Check worker count: `fastest -v tests/`
2. Look for slow test setup/teardown
3. Profile with `--verbose` flag
4. Check for blocking I/O or sleep calls

### High Memory Usage

1. Reduce batch size with fewer workers
2. Check for memory leaks in tests
3. Use `--optimizer standard` for lower memory usage

### Inconsistent Results

1. Ensure tests are parallel-safe
2. Check for test order dependencies
3. Use `-n 1` to run sequentially for debugging

## Advanced Configuration

### Custom Batch Sizes

The optimized executor uses batches of 50 tests by default. This can be tuned based on your test characteristics:

- Small, fast tests: Larger batches (100-200)
- Large, slow tests: Smaller batches (10-25)
- Mixed: Default (50)

### Persistent Workers (Experimental)

Enable persistent Python worker processes:

```bash
fastest --persistent-workers tests/
```

This avoids Python startup overhead but uses more memory.

## Future Optimizations

Planned performance improvements:

1. **Distributed Testing**: Run tests across multiple machines
2. **Smart Scheduling**: ML-based test scheduling
3. **Result Caching**: Cache test results based on code changes
4. **Native Extensions**: Rust implementations of common assertions
5. **Profile-Guided Optimization**: Automatic tuning based on test history 