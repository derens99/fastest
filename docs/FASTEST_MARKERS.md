# Fastest Marker Support 🏷️

Fastest supports both `pytest.mark.*` and its own `fastest.mark.*` decorators for marking tests. This allows you to use Fastest as a drop-in replacement for pytest while also taking advantage of Fastest-specific features.

## Quick Start

```python
import fastest

@fastest.mark.skip(reason="Not implemented yet")
def test_feature():
    pass

@fastest.mark.slow
def test_heavy_computation():
    # This test takes a while
    pass
```

## Supported Marker Formats

Fastest recognizes markers in both formats:
- `@pytest.mark.skip` - PyTest style (for compatibility)
- `@fastest.mark.skip` - Fastest native style

Both work identically and can be mixed in the same test file:

```python
import pytest
import fastest

@pytest.mark.skip
def test_pytest_style():
    pass

@fastest.mark.skip
def test_fastest_style():
    pass
```

## Built-in Markers

### skip
Skip a test unconditionally:
```python
@fastest.mark.skip
def test_not_ready():
    pass

@fastest.mark.skip(reason="Waiting for API implementation")
def test_api_call():
    pass
```

### xfail
Mark a test as expected to fail:
```python
@fastest.mark.xfail
def test_known_bug():
    assert False  # This failure is expected
```

### slow
Mark tests that take longer to run:
```python
@fastest.mark.slow
def test_integration():
    # Long-running test
    pass
```

### Custom Markers
You can create any custom marker:
```python
@fastest.mark.unit
def test_unit():
    pass

@fastest.mark.integration
def test_integration():
    pass

@fastest.mark.smoke
def test_critical_path():
    pass
```

## Filtering Tests by Markers

Use the `-m` flag to filter tests based on markers:

```bash
# Run only tests marked as 'slow'
fastest -m slow

# Run all tests except those marked as 'slow'
fastest -m "not slow"

# Run tests marked as 'unit' OR 'smoke'
fastest -m "unit or smoke"

# Run tests marked as 'integration' AND 'slow'
fastest -m "integration and slow"

# Complex expressions
fastest -m "not skip and (unit or integration)"
```

## Multiple Markers

Tests can have multiple markers:

```python
@fastest.mark.slow
@fastest.mark.integration
def test_database_integration():
    pass
```

## Using the Fastest Module

Install the fastest module in your project:

```python
# fastest.py (included with Fastest)
import fastest

# Use markers
@fastest.mark.skip
def test_skipped():
    pass

# Use shortcuts
@fastest.skip  # Shortcut for @fastest.mark.skip
def test_also_skipped():
    pass
```

## Parser Support

Both the regex and AST parsers support marker detection:

```bash
# Works with both parsers
fastest --parser regex -m slow
fastest --parser ast -m slow
```

## Implementation Status

### ✅ Implemented
- Marker detection (both pytest.mark and fastest.mark)
- Marker filtering with `-m` flag
- Complex marker expressions (and/or/not)
- Skip marker execution behavior
- Xfail marker execution behavior
- Custom marker support

### 🚧 Coming Soon
- `@fastest.mark.parametrize` - Run test with multiple parameter sets
- `@fastest.mark.skipif` - Conditional skipping
- `@fastest.mark.timeout` - Test timeout support
- Marker inheritance in test classes

## Examples

### Example Test File

```python
import fastest
import pytest

class TestMath:
    @fastest.mark.unit
    def test_addition(self):
        assert 1 + 1 == 2
    
    @fastest.mark.unit
    @fastest.mark.smoke
    def test_multiplication(self):
        assert 2 * 3 == 6

@pytest.mark.slow
@fastest.mark.integration
async def test_async_database():
    # Both pytest and fastest markers work
    await db.connect()
    assert await db.query("SELECT 1") == 1

@fastest.mark.skip(reason="Feature not implemented")
def test_future_feature():
    raise NotImplementedError
```

### Running Examples

```bash
# Run all unit tests
fastest -m unit

# Run smoke tests that aren't slow
fastest -m "smoke and not slow"

# Skip integration tests
fastest -m "not integration"

# Run only async tests (when implemented)
fastest -m asyncio
```

## Best Practices

1. **Use descriptive markers**: Create markers that clearly indicate the test's purpose
2. **Document custom markers**: Keep a list of your project's custom markers
3. **Combine markers thoughtfully**: Use multiple markers to create flexible test suites
4. **Prefer fastest.mark**: When writing new tests, use `fastest.mark` for better integration

## Migration from PyTest

If migrating from pytest, you can:
1. Keep all `@pytest.mark.*` decorators - they'll work as-is
2. Gradually migrate to `@fastest.mark.*` for new tests
3. Mix both styles during the transition

```python
# Both work identically
@pytest.mark.skip
def test_old_style():
    pass

@fastest.mark.skip  
def test_new_style():
    pass
``` 