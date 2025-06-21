# Performance Optimization Guide

Maximize the speed of your test suite with these optimization techniques.

## 🎯 Understanding Execution Strategies

Fastest automatically selects the optimal strategy, but understanding them helps you optimize:

### InProcess Strategy (≤20 tests)
- Runs tests directly in Rust process via PyO3
- Minimal overhead, fastest for small suites
- Best for: Unit tests, quick feedback loops

### HybridBurst Strategy (21-100 tests)  
- Intelligent batching with thread pool
- Balances parallelism and overhead
- Best for: Integration tests, medium suites

### WorkStealing Strategy (>100 tests)
- Lock-free parallel execution
- Maximum CPU utilization
- Best for: Large suites, CI/CD pipelines

## ⚡ Optimization Techniques

### 1. Parallel Execution (Default)

```bash
# Auto-detect optimal workers (recommended)
fastest tests/

# Explicit worker count
fastest -n 8 tests/  # 8 parallel workers

# Single-threaded (debugging)
fastest -n 1 tests/
```

### 2. Test Discovery Optimization

```bash
# Enable discovery cache (default)
fastest tests/

# Force cache refresh
fastest --no-cache tests/

# Use specific parser for simple tests
fastest --parser regex tests/  # Faster for simple patterns
```

### 3. Memory Allocator

Use mimalloc for 8-15% performance boost:

```bash
# Linux/macOS
LD_PRELOAD=/usr/lib/libmimalloc.so fastest tests/

# Or compile with mimalloc feature
cargo install fastest-cli --features mimalloc
```

### 4. Environment Variables

```bash
# Enable all optimizations
export FASTEST_OPTIMIZE=true

# Increase thread pool size
export FASTEST_THREADS=16

# Enable SIMD optimizations
export FASTEST_SIMD=avx2  # or avx512 if supported
```

## 📊 Test Suite Optimization

### 1. Test Organization

**Good:** Group related tests
```python
# test_user.py - All user-related tests together
class TestUserCreation:
    def test_valid_user(self): ...
    def test_invalid_email(self): ...

class TestUserAuth:
    def test_login(self): ...
    def test_logout(self): ...
```

**Avoid:** Scattered test files with few tests each

### 2. Fixture Optimization

**Use session/module fixtures for expensive setup:**
```python
@pytest.fixture(scope="session")
def database():
    # Expensive setup only happens once
    return setup_test_db()

@pytest.fixture(scope="module") 
def api_client():
    # Reused for all tests in module
    return create_client()
```

### 3. Parallel-Safe Tests

**Make tests independent:**
```python
# Good: Each test isolated
def test_user_creation(tmp_path):
    db_file = tmp_path / "test.db"
    # Use unique resources

# Avoid: Shared state
shared_counter = 0
def test_increment():
    global shared_counter
    shared_counter += 1  # Race condition!
```

## 🚀 CI/CD Optimization

### GitHub Actions Example

```yaml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      # Cache Fastest binary
      - name: Cache Fastest
        uses: actions/cache@v3
        with:
          path: ~/.local/bin/fastest
          key: fastest-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}
      
      # Install if not cached
      - name: Install Fastest
        run: |
          if ! command -v fastest &> /dev/null; then
            curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh
          fi
      
      # Run tests with optimizations
      - name: Run Tests
        run: |
          export FASTEST_OPTIMIZE=true
          fastest -n auto tests/
```

### Docker Optimization

```dockerfile
# Multi-stage build for smaller image
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features mimalloc

FROM python:3.11-slim
COPY --from=builder /app/target/release/fastest /usr/local/bin/
RUN fastest --version
```

## 📋 Performance Checklist

### Before Optimization
- [ ] Measure baseline performance
- [ ] Identify slowest tests
- [ ] Check for test interdependencies

### Quick Wins
- [ ] Enable parallel execution (default)
- [ ] Use discovery cache (default)
- [ ] Group related tests in same file
- [ ] Use appropriate fixture scopes

### Advanced Optimization  
- [ ] Install mimalloc
- [ ] Set FASTEST_OPTIMIZE=true
- [ ] Profile with `--profile` flag
- [ ] Optimize slowest tests first

## 📈 Monitoring Performance

### Generate Performance Report

```bash
# Run with profiling
fastest --profile tests/ > profile.json

# Analyze results
fastest analyze-profile profile.json
```

### Track Performance Over Time

```bash
# Save benchmark results
fastest benchmark --save results.json

# Compare with previous
fastest benchmark --compare results.json
```

## 🔧 Troubleshooting Slow Tests

### 1. Identify Slow Tests

```bash
# Show test durations
fastest -v tests/ | grep "passed in"

# List slowest tests
fastest --tb=no tests/ | sort -k3 -nr | head -20
```

### 2. Common Bottlenecks

- **Database Setup**: Use transactions and rollback
- **Network Calls**: Mock external services
- **File I/O**: Use tmpfs or memory filesystem
- **Sleep/Wait**: Replace with event-driven logic

### 3. Debug Strategy Selection

```bash
# See which strategy is used
FASTEST_LOG=debug fastest tests/ 2>&1 | grep "strategy"
```

## 🎯 Best Practices Summary

1. **Let Fastest choose**: Default settings are optimized
2. **Keep tests independent**: Enables parallelism
3. **Use appropriate fixtures**: Session > Module > Function
4. **Group related tests**: Better cache locality
5. **Mock external dependencies**: Eliminate I/O wait
6. **Profile before optimizing**: Measure, don't guess

---

*Remember: Fastest is already optimized out of the box. These techniques help you get the absolute maximum performance.*
