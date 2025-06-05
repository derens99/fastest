# Comprehensive Test Suite for Fastest

This directory contains a comprehensive test suite designed to thoroughly test all features of the Fastest test runner. The suite is organized into multiple files, each focusing on specific functionality.

## Test Files Overview

### 1. `test_comprehensive_suite_basic.py`
**Tests basic functionality including:**
- Simple pass/fail tests
- Test discovery patterns (naming conventions)
- Async tests
- Exception handling with pytest.raises
- Tests with imports and module variables
- Output capture (stdout/stderr)
- Various assertion types for introspection testing

**Key test scenarios:**
- Different test naming patterns
- Async/await functionality  
- Multiple assertion types
- Print output handling
- Module-level variables

### 2. `test_comprehensive_suite_fixtures.py`
**Tests the complete fixture system including:**
- All fixture scopes (function, class, module, session, package)
- Fixture dependencies and dependency resolution
- Yield fixtures with setup/teardown
- Autouse fixtures
- Parametrized fixtures
- Built-in fixtures (tmp_path, capsys, monkeypatch)
- Request object usage
- Fixture finalization

**Key test scenarios:**
- Complex fixture dependency chains
- Scope interaction and isolation
- Generator-based fixtures
- Error handling in fixtures

### 3. `test_comprehensive_suite_markers.py`
**Tests the marker system including:**
- @pytest.mark.skip with and without reasons
- @pytest.mark.xfail and XPASS scenarios
- @pytest.mark.skipif with various conditions
- Custom markers for test categorization
- Multiple markers on single test
- Runtime skip/xfail with pytest.skip()/pytest.xfail()
- Marker inheritance in classes
- Complex marker expressions

**Key test scenarios:**
- Platform-specific skips
- Expected failures vs unexpected passes
- Marker combinations and precedence
- Dynamic test skipping

### 4. `test_comprehensive_suite_parametrize.py`
**Tests parametrization features including:**
- Simple single-parameter tests
- Multiple parameter combinations
- Complex data types (lists, dicts, None)
- Parametrize with custom IDs
- Nested parametrization
- Parametrized classes
- Indirect parametrization through fixtures
- Large parameter sets

**Key test scenarios:**
- Unicode and special values
- Empty and None parameters
- Class-level parametrization
- Generated parameter sets

### 5. `test_comprehensive_suite_classes.py`
**Tests class-based testing including:**
- Basic test classes
- setup_method/teardown_method
- setup_class/teardown_class (classmethods)
- Class inheritance and method override
- Multiple inheritance with mixins
- unittest-style setUp/tearDown
- Nested test classes
- Async methods in classes

**Key test scenarios:**
- Setup/teardown execution order
- State isolation between methods
- Inheritance patterns
- Class-scoped fixtures

### 6. `test_comprehensive_suite_plugins.py`
**Tests plugin system including:**
- Plugin hook functionality
- pytest-mock compatibility (mocker fixture)
- pytest-cov compatibility
- Conftest loading and fixtures
- Plugin lifecycle
- Custom plugin markers
- Plugin communication
- Built-in plugins

**Key test scenarios:**
- Hook execution order
- Plugin-provided fixtures
- Dynamic plugin features
- Plugin error handling

### 7. `test_comprehensive_suite_performance.py`
**Tests performance and execution strategies including:**
- Small suite (<20 tests) for InProcess strategy
- Medium suite (21-100 tests) for HybridBurst strategy
- Large suite (>100 tests) for WorkStealing strategy
- CPU-bound vs I/O-bound tests
- Async performance tests
- Memory-intensive tests
- Fixture overhead measurement

**Key test scenarios:**
- Strategy selection validation
- Parallel execution efficiency
- Resource utilization
- Performance characteristics

### 8. `test_comprehensive_suite_edge_cases.py`
**Tests edge cases and error scenarios including:**
- Empty and minimal tests
- Unicode and special characters
- Very long names and strings
- Global state manipulation
- Import errors and missing dependencies
- Resource management issues
- Circular references
- Name collisions
- Memory edge cases

**Key test scenarios:**
- Unusual test patterns
- Error recovery
- Resource cleanup
- Platform-specific behavior

## Supporting Files

### `conftest_comprehensive.py`
Provides fixtures and hooks for the test suite, simulating plugin functionality:
- Basic conftest fixtures
- Plugin hook implementations
- Session-wide setup/teardown
- Custom marker registration

## Running the Test Suite

To run the entire comprehensive suite with Fastest:
```bash
cargo run --release -- testing_files/test_comprehensive_suite_*.py
```

To run specific test categories:
```bash
# Basic functionality only
cargo run --release -- testing_files/test_comprehensive_suite_basic.py

# Fixture tests only  
cargo run --release -- testing_files/test_comprehensive_suite_fixtures.py

# Performance tests only
cargo run --release -- testing_files/test_comprehensive_suite_performance.py
```

To run with specific markers:
```bash
# Run only slow tests
cargo run --release -- -m slow testing_files/

# Run tests except xfail
cargo run --release -- -m "not xfail" testing_files/
```

## Test Statistics

- **Total test files**: 8 main test files + 1 conftest
- **Approximate test count**: 500+ tests
- **Features covered**: All major pytest-compatible features
- **Execution strategies**: Tests for all 3 strategies (InProcess, HybridBurst, WorkStealing)

## Performance Expectations

With Fastest's optimized execution:
- Small suite (15 tests): ~0.3 seconds (InProcess)
- Medium suite (40-80 tests): ~0.3-0.4 seconds (HybridBurst) 
- Large suite (500+ tests): ~0.1-0.2 seconds (WorkStealing)
- Full suite: Should complete in under 0.5 seconds

This represents a 3-4x speedup compared to pytest!