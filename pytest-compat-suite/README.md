# pytest Compatibility Test Suite

This directory contains comprehensive test suites for validating pytest compatibility of the Fastest test runner. These tests ensure that Fastest correctly handles all pytest features and patterns.

## 📂 Organization

### `core/` - Core pytest functionality
- **`basic/`** - Basic test patterns and simple functions
- **`classes/`** - Class-based test organization  
- **`async/`** - Async test support (currently empty, for future use)

### `features/` - Advanced pytest features
- **`assertion-introspection/`** - Assertion failure inputs for traceback and introspection work
- **`fixtures/`** - Fixture system including scopes, dependencies, and conftest
- **`lifecycle/`** - Setup/teardown and lifecycle edge cases, including failure-path teardown
- **`markers/`** - Test markers (@pytest.mark.skip, xfail, custom markers)
- **`parametrization/`** - Legacy/mock parametrization fixtures used by Fastest discovery tests
- **`parametrize/`** - Parametrization including indirect and complex patterns
- **`setup-teardown/`** - Setup/teardown methods and session management
- **`plugins/`** - Plugin system and pytest plugin compatibility

### `comprehensive/` - Full validation suites
Complete test suites that validate multiple features working together, including:
- Comprehensive fixture validation
- Complete class feature testing
- Full assertion introspection
- Advanced test patterns

### `edge-cases/` - Edge cases and error handling
- Unicode handling in various contexts
- Error reporting and failure scenarios
- Unusual test patterns and edge conditions
- Invalid syntax fixtures under `edge-cases/invalid-syntax/fixtures/` are stored with a `.fixture` suffix so normal suite runs do not collect them.

### `examples/` - Example test files
Simple example tests demonstrating basic usage patterns

### `third-party/` - Installed package smoke tests
- **`plugins/`** - Narrow smoke coverage for installed third-party pytest plugin
  packages and the Fastest shim subset that currently supports them

## Purpose

These files serve several purposes:

1. **Validation** - Ensure Fastest correctly handles all test patterns
2. **Regression Testing** - Catch bugs when making changes
3. **Performance Testing** - Measure execution speed
4. **Compatibility Testing** - Verify pytest compatibility
5. **Feature Development** - Test new features during development

## Usage

These test suites validate pytest compatibility:

```bash
# Run basic tests
cargo run -- pytest-compat-suite/core/basic/

# Run specific feature tests
cargo run -- pytest-compat-suite/features/fixtures/
cargo run -- pytest-compat-suite/features/parametrize/

# Run comprehensive validation suite
cargo run -- pytest-compat-suite/comprehensive/

# Test edge cases
cargo run -- pytest-compat-suite/edge-cases/

# Test the third-party plugin smoke gate
make plugin-smoke
```

## Adding New Test Files

When adding new test files:
1. Place them in the appropriate subdirectory based on the feature being tested
2. Name them descriptively (test_<feature>.py)
3. Include both passing and failing test cases
4. Document edge cases and expected behavior
5. Update this README with the new test file

Intentional failing inputs belong here, not under `tests/`. The `tests/`
directory is reserved for project tests that should pass under normal pytest.

## See Also

- [Testing Guide](../docs/development/testing.md) - How to run project and compatibility tests
- [Architecture](../docs/development/architecture.md) - How discovery and execution are organized
