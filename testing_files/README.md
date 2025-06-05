# Testing Files Directory

This directory contains test files used for internal testing and validation of Fastest itself.

## Organization

### Core Functionality Tests
- `test_basic.py` - Basic test functionality validation
- `test_advanced.py` - Advanced test patterns
- `simple_test.py` - Minimal test file for quick testing
- `test_small_suite.py` - Small test suite for performance testing

### Feature-Specific Tests

#### Fixtures
- `test_fixtures.py` - Basic fixture functionality
- `test_advanced_fixtures.py` - Advanced fixture patterns
- `test_comprehensive_fixtures.py` - Complete fixture system validation
- `conftest.py` - Root conftest for fixture definitions
- `test_conftest_loading.py` - Conftest loading behavior

#### Parametrization
- `test_parametrize.py` - Basic parametrization tests
- `test_complex_parametrize.py` - Complex parametrization patterns
- `test_parametrize_ids.py` - Parametrization with custom IDs
- `test_indirect_parametrize.py` - Indirect parametrization

#### Class-Based Tests
- `test_simple_class.py` - Simple class-based tests
- `test_class_verification.py` - Class test discovery validation
- `test_class_method_fix.py` - Class method handling
- `test_class_based_comprehensive.py` - Comprehensive class features
- `test_comprehensive_class_features.py` - All class test patterns
- `test_class_teardown_order.py` - Class teardown ordering

#### Markers
- `test_markers.py` - Comprehensive marker tests
- `test_markers_simple.py` - Simple marker examples

#### Setup/Teardown
- `test_setup_teardown.py` - Basic setup/teardown
- `test_setup_teardown_order.py` - Execution order validation
- `test_setup_teardown_errors.py` - Error handling
- `test_setup_teardown_fixtures.py` - Integration with fixtures

#### Plugin System
- `test_plugin_loading.py` - Plugin loading mechanisms
- `test_plugin_hooks.py` - Hook system validation
- `test_plugin_integration.py` - Integration tests
- `test_plugin_integration_advanced.py` - Advanced plugin patterns
- `test_plugin_cli.py` - CLI plugin options

#### pytest Plugin Compatibility
- `test_pytest_mock.py` - pytest-mock compatibility
- `test_pytest_cov.py` - pytest-cov compatibility
- `test_pytest_timeout.py` - pytest-timeout compatibility
- `test_pytest_xdist.py` - pytest-xdist compatibility
- `test_pytest_asyncio.py` - pytest-asyncio compatibility

### Performance & Benchmarking
- `test_simd_benchmark.py` - SIMD optimization benchmarks
- `test_simd_parallel.py` - Parallel SIMD tests
- `test_json_bound_performance.py` - JSON performance tests
- `test_mimalloc_performance.py` - Memory allocator performance
- `test_mimalloc_stress.py` - Memory stress tests

### Error Handling & Reporting
- `test_failures.py` - Test failure scenarios
- `test_error_reporting.py` - Error message formatting
- `test_assertion_introspection.py` - Enhanced assertion messages

### Comprehensive Test Suite
The `test_comprehensive_suite_*.py` files form a complete validation suite:
- `test_comprehensive_suite_basic.py` - Core functionality
- `test_comprehensive_suite_classes.py` - Class-based tests
- `test_comprehensive_suite_fixtures.py` - Fixture system
- `test_comprehensive_suite_markers.py` - Marker system
- `test_comprehensive_suite_parametrize.py` - Parametrization
- `test_comprehensive_suite_plugins.py` - Plugin system
- `test_comprehensive_suite_edge_cases.py` - Edge cases
- `test_comprehensive_suite_performance.py` - Performance patterns

See also:
- [Comprehensive Test Results](../docs/COMPREHENSIVE_TEST_RESULTS.md) - Detailed test results and compatibility report
- [Comprehensive Suite Documentation](../docs/COMPREHENSIVE_SUITE_DETAILS.md) - Implementation details and test patterns

### Other Tests
- `test_discovery_example.py` - Test discovery patterns
- `test_jit_simple.py` - JIT compilation tests
- `test_simple_working.py` - Basic working test

## Purpose

These files serve several purposes:

1. **Validation** - Ensure Fastest correctly handles all test patterns
2. **Regression Testing** - Catch bugs when making changes
3. **Performance Testing** - Measure execution speed
4. **Compatibility Testing** - Verify pytest compatibility
5. **Feature Development** - Test new features during development

## Usage

These files are primarily for internal testing:

```bash
# Run a specific test file
cargo run -- testing_files/test_basic.py

# Run comprehensive suite
cargo run -- testing_files/test_comprehensive_suite_*.py

# Test specific feature
cargo run -- testing_files/test_parametrize.py -v
```

## Adding New Test Files

When adding new test files:
1. Name them descriptively (test_<feature>.py)
2. Group related tests in comprehensive suite files
3. Include both passing and failing test cases
4. Document edge cases and expected behavior
5. Keep performance test files separate