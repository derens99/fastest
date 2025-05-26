# Testing Guide for Fastest

This document describes the testing strategy, structure, and procedures for the Fastest project.

## Table of Contents

1. [Testing Strategy](#testing-strategy)
2. [Test Structure](#test-structure)
3. [Running Tests](#running-tests)
4. [Writing Tests](#writing-tests)
5. [CI/CD Pipeline](#cicd-pipeline)
6. [Performance Testing](#performance-testing)
7. [Coverage](#coverage)

## Testing Strategy

Fastest uses a multi-layered testing approach:

1. **Unit Tests**: Test individual components in isolation
2. **Integration Tests**: Test interactions between components
3. **End-to-End Tests**: Test the complete workflow
4. **Performance Tests**: Benchmark critical paths
5. **Compatibility Tests**: Ensure pytest compatibility

## Test Structure

```
fastest/
├── tests/                    # Python test files for testing fastest itself
│   ├── test_*.py            # Various test scenarios
│   └── conftest.py          # pytest configuration
├── crates/
│   ├── fastest-core/
│   │   └── src/
│   │       └── **/*.rs      # Unit tests in Rust modules
│   └── fastest-cli/
│       └── tests/           # CLI integration tests
├── tests/                   # Integration tests
│   └── integration_test.rs  # Rust integration tests
└── benchmarks/             # Performance benchmarks
    └── bench_*.py          # Python benchmark scripts
```

## Running Tests

### Rust Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in release mode
cargo test --release

# Run with specific features
cargo test --all-features
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration_test

# Run with verbose output
cargo test --test integration_test -- --nocapture
```

### Python Tests

First, build the release binary:

```bash
cargo build --release
```

Then run Python tests:

```bash
# Run all Python tests
./target/release/fastest tests/

# Run with coverage
./target/release/fastest tests/ --cov

# Run specific test file
./target/release/fastest tests/test_basic.py

# Run with verbose output
./target/release/fastest tests/ -v
```

### Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --all-features

# View coverage report
open tarpaulin-report.html
```

## Writing Tests

### Rust Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = "test";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

### Integration Tests

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_cli_command() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_example.py");
    
    std::fs::write(&test_file, "def test_foo(): assert True").unwrap();
    
    Command::cargo_bin("fastest")
        .unwrap()
        .arg(test_file)
        .assert()
        .success()
        .stdout(predicate::str::contains("1 tests passed"));
}
```

### Python Test Files

Create test files following pytest conventions:

```python
# test_example.py
def test_simple():
    """Test basic functionality."""
    assert 1 + 1 == 2

class TestClass:
    """Test class-based tests."""
    
    def test_method(self):
        assert True
    
    async def test_async_method(self):
        import asyncio
        await asyncio.sleep(0.001)
        assert True
```

## CI/CD Pipeline

The project uses GitHub Actions for continuous integration:

### Workflows

1. **ci.yml**: Runs on every push and PR
   - Tests on multiple OS (Linux, macOS, Windows)
   - Tests with multiple Python versions (3.8-3.12)
   - Runs clippy and rustfmt
   - Generates coverage reports
   - Runs security audits

2. **release.yml**: Runs on version tags
   - Builds binaries for all platforms
   - Creates GitHub releases
   - Publishes to crates.io
   - Updates Docker images
   - Updates Homebrew formula

### Local CI Testing

You can test CI locally using [act](https://github.com/nektos/act):

```bash
# Install act
brew install act  # macOS
# or see https://github.com/nektos/act for other platforms

# Run CI workflow
act -j test

# Run specific job
act -j clippy
```

## Performance Testing

### Running Benchmarks

```bash
# Run discovery benchmark
python benchmarks/bench_discovery.py

# Run with profiling
python benchmarks/bench_performance.py --profile
```

### Writing Benchmarks

```python
# benchmarks/bench_new_feature.py
import time
import subprocess

def benchmark_feature():
    start = time.time()
    
    # Run fastest with specific configuration
    result = subprocess.run([
        "./target/release/fastest",
        "tests/",
        "--feature-flag"
    ], capture_output=True)
    
    duration = time.time() - start
    return duration

if __name__ == "__main__":
    times = [benchmark_feature() for _ in range(10)]
    avg_time = sum(times) / len(times)
    print(f"Average time: {avg_time:.3f}s")
```

## Coverage

### Requirements

- Minimum 80% code coverage for new features
- Critical paths must have 100% coverage
- Integration tests should cover all CLI commands

### Viewing Coverage

1. **Local HTML Report**:
   ```bash
   cargo tarpaulin --out Html
   open tarpaulin-report.html
   ```

2. **CI Coverage**: Check Codecov reports linked in PR comments

3. **Python Coverage**:
   ```bash
   ./target/release/fastest tests/ --cov --cov-report html
   open htmlcov/index.html
   ```

## Test Data

Test fixtures and sample files are located in:

- `tests/fixtures/`: Reusable test data
- `examples/`: Example Python test files
- `tests/`: Actual test files

## Debugging Tests

### Rust Tests

```bash
# Run with backtrace
RUST_BACKTRACE=1 cargo test

# Run specific test with output
cargo test test_name -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test
```

### Python Tests

```bash
# Run with verbose output
./target/release/fastest tests/ -v

# Debug specific test
./target/release/fastest tests/test_file.py::test_name -v

# Check generated Python code
./target/release/fastest tests/ -v
# Then check /tmp/fastest_debug.py
```

## Troubleshooting

### Common Issues

1. **Test Discovery Issues**
   - Clear cache: `rm -rf ~/Library/Caches/fastest`
   - Run with `--no-cache` flag
   - Check Python path with `-v` flag

2. **Virtual Environment Issues**
   - Ensure virtual env is activated
   - Check Python detection in verbose output

3. **Performance Issues**
   - Run with `--workers 1` to debug sequential execution
   - Check for test isolation issues

### Getting Help

- Check CI logs for test failures
- Run with `-v` flag for detailed output
- Check `/tmp/fastest_debug.py` for generated code
- Open an issue with reproduction steps 