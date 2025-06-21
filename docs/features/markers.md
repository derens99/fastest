# Test Markers in Fastest

Fastest supports both `pytest.mark.*` and `fastest.mark.*` decorators for organizing and filtering your tests.

## Quick Start

```python
import fastest
import pytest  # pytest marks also work!

@fastest.mark.slow
def test_heavy_computation():
    # This test takes a while
    pass

@pytest.mark.skip(reason="Not implemented yet")
def test_future_feature():
    pass

@fastest.mark.smoke
@fastest.mark.unit
def test_critical_functionality():
    # Multiple markers are supported
    pass
```

## Running Tests with Markers

Use the `-m` flag to filter tests based on markers:

```bash
# Run only slow tests
fastest -m "slow"

# Skip slow tests
fastest -m "not slow"

# Run unit OR integration tests
fastest -m "unit or integration"

# Run smoke tests that aren't marked as skip
fastest -m "smoke and not skip"

# Complex expressions
fastest -m "(unit or integration) and not slow"
```

## Available Markers

### Built-in Markers

#### `@fastest.mark.skip` / `@pytest.mark.skip`
Skip test execution entirely.

```python
@fastest.mark.skip(reason="Database not available in CI")
def test_database_integration():
    pass
```

#### `@fastest.mark.slow` / `@pytest.mark.slow`
Mark tests that take a long time to run.

```python
@fastest.mark.slow
def test_large_file_processing():
    # Process 1GB file
    pass
```

#### `@fastest.mark.xfail` / `@pytest.mark.xfail`
Mark tests that are expected to fail.

```python
@pytest.mark.xfail(reason="Known bug in third-party library")
def test_external_api():
    assert external_api.call() == "expected"
```

### Custom Markers

You can create any custom markers:

```python
@fastest.mark.smoke
def test_login():
    """Critical path test"""
    pass

@fastest.mark.integration
def test_api_endpoint():
    """Requires external services"""
    pass

@fastest.mark.unit
def test_pure_function():
    """No external dependencies"""
    pass
```

## Marker Expressions

The `-m` flag supports boolean expressions:

- `and` - Both conditions must be true
- `or` - Either condition must be true
- `not` - Negates the condition
- `()` - Groups expressions

### Examples

```bash
# Run all tests except slow ones
fastest -m "not slow"

# Run smoke tests on Linux only
fastest -m "smoke and linux"

# Run either unit tests or fast integration tests
fastest -m "unit or (integration and not slow)"

# Skip WIP (work in progress) and experimental tests
fastest -m "not (wip or experimental)"
```

## Best Practices

### 1. Use Semantic Marker Names
```python
# Good
@fastest.mark.requires_database
@fastest.mark.external_api
@fastest.mark.smoke

# Less clear
@fastest.mark.test1
@fastest.mark.group_a
```

### 2. Document Your Markers
Add marker descriptions in your project documentation:

```python
# markers.py or conftest.py
"""
Project Test Markers:

@fastest.mark.slow: Tests that take > 1 second
@fastest.mark.smoke: Core functionality tests
@fastest.mark.integration: Tests requiring external services
@fastest.mark.unit: Pure unit tests with no I/O
"""
```

### 3. Combine Markers Logically
```python
@fastest.mark.slow
@fastest.mark.integration
@fastest.mark.requires_database
def test_full_database_migration():
    """This test is slow, requires integration, and needs a database"""
    pass
```

### 4. Use Markers in CI/CD
```yaml
# .github/workflows/test.yml
- name: Run unit tests
  run: fastest -m "unit"

- name: Run smoke tests
  run: fastest -m "smoke and not slow"

- name: Run all tests except experimental
  run: fastest -m "not experimental"
```

## Marker Inheritance

Class-level markers are inherited by all methods:

```python
@fastest.mark.integration
class TestAPIEndpoints:
    # All methods inherit the 'integration' marker
    
    def test_get_user(self):
        pass
    
    @fastest.mark.slow
    def test_bulk_import(self):
        # This test has both 'integration' and 'slow' markers
        pass
```

## Performance Tip

Using markers to skip unnecessary tests can significantly speed up your test runs:

```bash
# During development - skip slow integration tests
fastest -m "not (slow or integration)"

# Before commit - run everything except experimental
fastest -m "not experimental"

# In CI - run different marker sets in parallel jobs
```

## Compatibility Note

Fastest supports both `pytest.mark.*` and `fastest.mark.*` for maximum compatibility. You can mix and match in the same file:

```python
import pytest
import fastest

@pytest.mark.skip  # Works!
@fastest.mark.slow  # Also works!
def test_mixed_markers():
    pass
```

This makes migrating from pytest seamless - your existing markers will continue to work! 