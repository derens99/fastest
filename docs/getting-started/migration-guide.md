# Migration Guide: From pytest to Fastest

This guide helps you migrate your test suite from pytest to Fastest, highlighting differences and providing solutions for common scenarios.

## Table of Contents
- [Quick Start](#quick-start)
- [Command Line Equivalents](#command-line-equivalents)
- [Feature Compatibility](#feature-compatibility)
- [Common Migration Patterns](#common-migration-patterns)
- [Gradual Migration Strategy](#gradual-migration-strategy)

## Quick Start

### Basic Migration
```bash
# Before (pytest)
pytest tests/

# After (fastest)
fastest tests/

# Or with virtual environment
source .venv/bin/activate
fastest tests/
```

### Installation
```bash
# Install fastest
pip install fastest

# Or from source
git clone https://github.com/derens99/fastest.git
cd fastest
cargo build --release
```

## Command Line Equivalents

| pytest | fastest | Notes |
|--------|---------|-------|
| `pytest` | `fastest` | Run all tests |
| `pytest tests/` | `fastest tests/` | Run tests in directory |
| `pytest -k pattern` | `fastest -k pattern` | Filter by pattern |
| `pytest -m "not slow"` | `fastest -m "not slow"` | Filter by markers |
| `pytest -x` | `fastest -x` | Stop on first failure |
| `pytest -v` | `fastest -v` | Verbose output |
| `pytest -n auto` | `fastest -n 0` | Parallel execution |
| `pytest --collect-only` | `fastest discover` | Discover without running |

## Feature Compatibility

### ✅ Fully Compatible Features

**Basic Test Functions**
```python
# Works identically in both
def test_simple():
    assert 1 + 1 == 2

async def test_async():
    await asyncio.sleep(0.1)
    assert True
```

**Test Classes**
```python
# Works identically in both
class TestMath:
    def test_addition(self):
        assert 2 + 3 == 5
```

**Basic Markers**
```python
# Supported markers
@pytest.mark.skip(reason="Not ready")
def test_skip():
    pass

@pytest.mark.xfail
def test_expected_failure():
    assert False
```

**Basic Fixtures**
```python
# Supported fixtures
def test_output(capsys):
    print("Hello")
    captured = capsys.readouterr()
    assert "Hello" in captured.out

def test_temp_files(tmp_path):
    file = tmp_path / "test.txt"
    file.write_text("content")
    assert file.read_text() == "content"
```

### ⚠️ Partial Compatibility

**Custom Markers**
```python
# pytest
@pytest.mark.slow
def test_slow():
    pass

# fastest - use for filtering only, no custom behavior
@pytest.mark.slow
def test_slow():
    pass
```

### ❌ Not Yet Supported

**Config Files** (Coming in v0.2.0)
```ini
# pytest.ini - not yet supported
[tool:pytest]
testpaths = tests
python_files = test_*.py
```

**Complex Fixtures**
```python
# Session/module scoped fixtures not supported
@pytest.fixture(scope="session")
def database():
    # Not supported
    pass

# Workaround: Use setup in each test
def test_with_db():
    db = setup_database()
    try:
        # test code
    finally:
        teardown_database(db)
```

## Common Migration Patterns

### Pattern 1: Simple Test Suite
If your tests are mostly simple functions without complex fixtures:

```bash
# Just switch the command
fastest tests/ -v
```

### Pattern 2: Tests with Parametrization
Until parametrized tests are supported:

```python
# Original parametrized test
@pytest.mark.parametrize("input,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
])
def test_upper(input, expected):
    assert input.upper() == expected

# Migration approach 1: Loop in single test
def test_upper_cases():
    cases = [
        ("hello", "HELLO"),
        ("world", "WORLD"),
    ]
    for input_val, expected in cases:
        assert input_val.upper() == expected

# Migration approach 2: Separate tests (better for parallel execution)
def test_upper_hello():
    assert "hello".upper() == "HELLO"

def test_upper_world():
    assert "world".upper() == "WORLD"
```

### Pattern 3: Custom Fixtures
```python
# pytest custom fixture
@pytest.fixture
def client():
    return TestClient()

def test_api(client):
    response = client.get("/")
    assert response.status_code == 200

# fastest approach
def test_api():
    client = TestClient()
    response = client.get("/")
    assert response.status_code == 200
```

## Gradual Migration Strategy

### Phase 1: Parallel Usage (Recommended Start)
```bash
# Use fastest for quick local testing
fastest tests/ -k test_unit

# Keep pytest for CI/full test runs
pytest tests/ --cov=myproject
```

### Phase 2: Migrate Simple Modules
1. Identify modules without complex pytest features
2. Verify they work with fastest
3. Update CI for these modules

```yaml
# .github/workflows/test.yml
- name: Run unit tests (fastest)
  run: fastest tests/unit -v

- name: Run integration tests (pytest)
  run: pytest tests/integration --cov
```

### Phase 3: Refactor Complex Tests
1. Replace parametrized tests with loops or separate tests
2. Simplify fixture usage
3. Remove plugin dependencies where possible

### Phase 4: Full Migration
Once all tests are compatible:
```bash
# Replace pytest entirely
pip uninstall pytest
fastest tests/
```

## Performance Tips

1. **Use AST parser for accurate discovery**
   ```bash
   fastest --parser ast tests/
   ```

2. **Enable parallel execution**
   ```bash
   fastest -n 0  # Auto-detect cores
   ```

3. **Use the optimized executor (default)**
   ```bash
   fastest --optimizer optimized
   ```

4. **Leverage caching**
   ```bash
   # First run builds cache
   fastest tests/
   
   # Subsequent runs are faster
   fastest tests/
   ```

## Troubleshooting

### Python Not Found
```bash
# Activate virtual environment
source .venv/bin/activate
fastest tests/
```

### Import Errors
```bash
# Ensure test files are in Python path
export PYTHONPATH="${PYTHONPATH}:."
fastest tests/
```

### Missing Features
Check the [compatibility matrix](../README.md#-pytest-compatibility) for supported features and planned additions.

## Getting Help

- **GitHub Issues**: Report bugs or request features
- **Discussions**: Ask questions and share experiences
- **Contributing**: Help implement missing features!

---

Remember: You don't need to migrate everything at once. Start with simple test modules and gradually expand as Fastest gains more features. 