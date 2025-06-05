# Examples Directory

This directory contains example code demonstrating how to use Fastest with various test patterns and features.

## Organization

### Basic Examples
- `quickstart.py` - Simple getting started example with basic tests
- `basic_tests.py` - Collection of basic test patterns and assertions

### Feature Examples  
- `test_parametrize_example.py` - Comprehensive parametrization examples with pytest.mark.parametrize
- `conftest_example.py` - Example conftest.py file showing fixture definitions
- `demo_install.py` - Example installation and setup script

### Plugin Development
- `plugin_example.rs` - Example Rust plugin implementation

### Sample Project
- `sample_project/` - Complete example project structure
  - `tests/test_math.py` - Math function tests
  - `tests/test_strings.py` - String manipulation tests
  - `tests/test_decorated.py` - Tests with decorators and markers

## Usage

### Running Examples

To run any example file:

```bash
# Using Fastest
fastest examples/quickstart.py

# Run all examples in sample project
fastest examples/sample_project/tests/

# Run with specific options
fastest examples/test_parametrize_example.py -v
```

### Quick Start Example

The simplest way to start:

```python
# examples/quickstart.py
def test_simple():
    assert 1 + 1 == 2

def test_with_error_message():
    assert 2 + 2 == 4, "Math is broken!"
```

### Parametrization Example

Shows various parametrization patterns:

```python
# Single parameter
@pytest.mark.parametrize("word", ["hello", "world", "fastest"])
def test_length(word):
    assert len(word) > 0

# Multiple parameters  
@pytest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (4, 5, 9),
])
def test_addition(x, y, expected):
    assert x + y == expected
```

### Creating Your Own Examples

When adding new examples:
1. Keep them simple and focused on demonstrating specific features
2. Include docstrings explaining what the example shows
3. Use realistic but not overly complex test scenarios
4. Consider adding to sample_project/ for complete project examples

## Learning Path

1. Start with `quickstart.py` for the basics
2. Explore `basic_tests.py` for common test patterns
3. Study `test_parametrize_example.py` for data-driven testing
4. Check `sample_project/` for project organization
5. Look at `conftest_example.py` for fixtures
6. Review `plugin_example.rs` for extending Fastest