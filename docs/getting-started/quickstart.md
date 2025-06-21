# Quickstart Guide

Get up and running with Fastest in 5 minutes!

## 🎯 Prerequisites

- Python 3.8 or higher
- 5 minutes of your time

## ⚡ 1-Minute Install

```bash
# macOS/Linux
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh

# Windows (PowerShell)
iwr -useb https://raw.githubusercontent.com/yourusername/fastest/main/install.ps1 | iex
```

## 🏃 First Test Run

### Run Existing Tests

```bash
# Run all tests in current directory
fastest

# Run tests in specific directory
fastest tests/

# Run a specific test file
fastest test_math.py

# Run tests matching a pattern
fastest -k "user" tests/
```

### Create a Simple Test

```python
# test_example.py
def test_addition():
    assert 1 + 1 == 2

def test_string():
    assert "hello".upper() == "HELLO"

class TestMath:
    def test_multiplication(self):
        assert 2 * 3 == 6
```

Run it:
```bash
fastest test_example.py
```

## 🚀 Key Features Demo

### Parallel Execution (Default)

```bash
# Fastest automatically uses optimal parallelization
fastest tests/

# Control worker count
fastest -n 4 tests/  # Use 4 workers
fastest -n auto tests/  # Auto-detect (default)
```

### Test Filtering

```bash
# By name pattern
fastest -k "login or auth" tests/

# By marker
fastest -m "slow" tests/

# Exclude pattern
fastest -k "not integration" tests/
```

### Output Formats

```bash
# Default (pretty)
fastest tests/

# Verbose output
fastest -v tests/

# Quiet (minimal)
fastest -q tests/

# JSON output
fastest -o json tests/ > results.json
```

### Stop on First Failure

```bash
fastest -x tests/
```

## 🎉 Real-World Example

Create a more realistic test file:

```python
# test_user.py
import pytest

class User:
    def __init__(self, name, email):
        self.name = name
        self.email = email
    
    def is_valid(self):
        return "@" in self.email

@pytest.fixture
def user():
    return User("Alice", "alice@example.com")

def test_user_creation(user):
    assert user.name == "Alice"
    assert user.email == "alice@example.com"

def test_user_validation(user):
    assert user.is_valid() is True

@pytest.mark.parametrize("email,expected", [
    ("valid@email.com", True),
    ("invalid-email", False),
    ("another@test.org", True),
])
def test_email_validation(email, expected):
    user = User("Test", email)
    assert user.is_valid() == expected
```

Run with:
```bash
fastest test_user.py -v
```

## 📊 Performance Comparison

See the speed difference:

```bash
# Time with pytest
time pytest tests/

# Time with fastest
time fastest tests/
```

Typical results:
- **pytest**: 2.5 seconds for 100 tests
- **fastest**: 0.4 seconds for 100 tests
- **Speedup**: 6x faster! 🚀

## 🎆 What's Next?

1. **[Migration Guide](migration-guide.md)** - Migrate from pytest
2. **[Fixtures](../features/fixtures.md)** - Learn about fixtures
3. **[Markers](../features/markers.md)** - Use test markers
4. **[Plugins](../features/plugins.md)** - Extend functionality
5. **[Performance Guide](../performance/optimization.md)** - Optimize test runs

## 👜 Quick Tips

- Fastest is a drop-in replacement for pytest
- Most pytest plugins work out of the box
- Use `fastest --help` to see all options
- File an issue if something doesn't work as expected

---

**Ready to make your tests blazing fast? You're all set! 🎉**
