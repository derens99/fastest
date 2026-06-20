# Changelog

This changelog records historical development milestones. It is not the source of truth for current compatibility or performance claims; use `make verify`, `make compat-report-all`, and current benchmark artifacts for release status.

## [0.4.0] - January 2025

### Added
- **Plugin System Scaffolding** (January 2025)
  - Type-safe hook-based plugin architecture
  - pytest-style hook compatibility for supported hooks
  - Hook calls at all test lifecycle points
  - Built-in plugins for core functionality (fixtures, markers, reporting, capture)
  - Plugin manager integrated into execution engine
  - CLI support for plugin options (--no-plugins, --plugin-dir, --disable-plugin)
  - Hook registry with execution tracing
  - Minimal working implementation that compiles and runs
  - Historical local compatibility milestone

### Integrated
- **Plugin System Integration** (January 2025)
  - Plugin manager added to UltraFastExecutor
  - Hooks called during test collection and execution
  - Collection hooks: pytest_collection_start, modifyitems, finish
  - Session hooks: pytest_sessionstart, sessionfinish
  - Test execution hooks: runtest_setup, call, teardown, logreport
  - CLI arguments for plugin control
  - Built-in plugins automatically registered
  - Debug output shows hook execution with FASTEST_DEBUG=1

### Planned
- **pytest Plugin Compatibility** (Coming Soon)
  - `pytest-mock`: Mocker fixture implementation
  - `pytest-cov`: Coverage collection and reporting
  - Python plugin loading from installed packages
  - Hierarchical conftest.py loading
  - Native Rust plugin support via dynamic loading

## [0.3.0] - January 2025

### Added
- **Marker System Milestone** (January 2025)
  - Support for `@pytest.mark.skip` with skip reasons
  - `@pytest.mark.xfail` support with xpass detection
  - `@pytest.mark.skipif` with basic condition evaluation
  - Custom marker support and filtering
  - Marker expressions in CLI (`-m` option)
  - Runtime skip/xfail support (`pytest.skip()`, `pytest.xfail()`)
  - Enhanced test result reporting with skip/xfail counts

- **Setup/Teardown Methods** (January 2025)
  - Support for common pytest setup/teardown methods
  - Module level: `setup_module()` and `teardown_module()`
  - Class level: `setup_class()` and `teardown_class()` (classmethod)
  - Method level: `setup_method()` and `teardown_method()`
  - Function level: `setup_function()` and `teardown_function()`
  - unittest-style: `setUp()` and `tearDown()`
  - Proper execution order with fixtures
  - Teardown always runs even on test failure
  - Setup failures skip appropriate tests

### Fixed
- **Parametrized Test Value Mapping** (January 2025)
  - Fixed critical bug where parametrized tests received indices instead of actual values
  - Tests now receive correct parameter values from `@pytest.mark.parametrize`
  - Complex parameter types (lists, dicts, None) are properly passed to test functions
  - Test IDs now show actual parameter values instead of indices
  - Parameter values are stored in decorators and passed through execution engine

## [0.2.0] - January 2025

### Added
- **Fixture System Milestone** (January 2025)
  - All fixture scopes implemented (function, class, module, session, package)
  - Dependency resolution with topological sorting
  - Autouse fixture support
  - Yield fixture support with proper teardown
  - Fixture parametrization
  - Request object implementation
  - Built-in fixtures: tmp_path, capsys, monkeypatch

- **Class-Based Test Support** (January 2025)
  - Discovery of test classes (TestClass pattern)
  - Method execution with proper self handling
  - Support for setUp/tearDown methods
  - Async test methods in classes
  - Class inheritance support
  - 85% pytest compatibility for class-based tests

## [0.1.1]

### Added
- **Parametrized Test Support**
  - Support for `@pytest.mark.parametrize` decorator
  - Support for `@fastest.mark.parametrize` decorator (native syntax)
  - Handles single and multiple parameters
  - Supports multiple decorators (cartesian product)
  - Complex parameter values (tuples, lists, strings, numbers)
  - Proper test ID generation with parameter values
  - Works with both regex and AST parsers
