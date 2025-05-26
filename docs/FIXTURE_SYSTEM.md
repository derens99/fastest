# Fastest Fixture System

## Overview

Fastest now supports a basic fixture system compatible with both `pytest.fixture` and `fastest.fixture` decorators. This allows for test setup/teardown and dependency injection.

## Current Implementation Status

### ✅ Completed
- **Fixture Discovery**: Parser can identify fixture definitions with `@pytest.fixture` and `@fastest.fixture`
- **Dependency Extraction**: Test functions' fixture dependencies are parsed from their parameters
- **Fixture Metadata**: Scope, autouse, and parameter information is captured
- **CLI Support**: Verbose mode shows fixture dependencies for each test
- **Python API**: `fastest.fixture` decorator available in `fastest.py` module

### 🚧 In Progress
- **Fixture Execution**: Actual fixture resolution and injection during test runs
- **Scope Management**: Proper handling of function/class/module/session scopes
- **Built-in Fixtures**: Common fixtures like `tmp_path`, `capsys`, etc.

## Usage

### Defining Fixtures

```python
import fastest
import pytest

# Using fastest decorator
@fastest.fixture
def my_data():
    return {"key": "value"}

# Using pytest decorator (for compatibility)
@pytest.fixture
def pytest_data():
    return {"source": "pytest"}

# Fixture with scope
@fastest.fixture(scope="module")
def shared_resource():
    # Setup
    resource = expensive_setup()
    yield resource
    # Teardown
    resource.cleanup()

# Fixture with dependencies
@fastest.fixture
def combined_fixture(my_data, shared_resource):
    return {
        "data": my_data,
        "resource": shared_resource
    }
```

### Using Fixtures in Tests

```python
def test_with_fixture(my_data):
    assert my_data["key"] == "value"

def test_with_multiple(my_data, pytest_data):
    assert my_data["key"] == "value"
    assert pytest_data["source"] == "pytest"

class TestClass:
    def test_method(self, shared_resource):
        # Fixture is injected as parameter
        assert shared_resource is not None
```

## Implementation Details

### Parser Support

Both regex and AST parsers support fixture discovery:

1. **Regex Parser**: 
   - Collects decorators before function definitions
   - Identifies fixture decorators by pattern matching
   - Extracts parameters from function signatures

2. **AST Parser**: 
   - Parses decorator nodes directly
   - More accurate for complex decorator expressions

### Data Structures

```rust
pub struct FixtureDefinition {
    pub name: String,
    pub line_number: usize,
    pub is_async: bool,
    pub scope: String,        // "function", "class", "module", "session"
    pub autouse: bool,
    pub params: Vec<String>,
    pub decorators: Vec<String>,
}

pub struct TestItem {
    // ... other fields ...
    pub fixture_deps: Vec<String>,  // Required fixtures
}
```

### Fixture Manager

The `FixtureManager` handles:
- Fixture registration
- Dependency resolution (topological sort)
- Scope-based caching
- Instance lifecycle management

## Viewing Fixture Dependencies

Use verbose mode to see fixture dependencies:

```bash
# Show all test details including fixtures
fastest -v discover

# Example output:
● test_module::test_with_fixtures
  Path: ./test_module.py
  Line: 42
  Fixtures: database, user, session
```

## Limitations

Current implementation limitations:
1. Fixtures are discovered but not yet executed during test runs
2. No support for parametrized fixtures yet
3. Built-in fixtures (tmp_path, etc.) not implemented
4. Fixture teardown not yet handled

## Next Steps

1. **Integrate with Executors**: Modify single/batch/parallel executors to resolve and inject fixtures
2. **Python Bridge**: Create Python code to execute fixture functions and return values
3. **Scope Management**: Implement proper caching based on fixture scope
4. **Built-in Fixtures**: Add commonly used pytest fixtures

## Example Test Output

```
$ fastest -v discover | grep -B1 "Fixtures:"
● test_fixtures_demo::test_with_pytest_fixture
  Fixtures: pytest_data
● test_fixtures_demo::test_with_multiple_fixtures  
  Fixtures: pytest_data, fastest_data
● test_fixtures_demo::test_with_fixture_dependencies
  Fixtures: combined_data
``` 