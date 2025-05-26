"""Test parametrized functionality with mock pytest."""

# Mock pytest to avoid dependency
class Mark:
    @staticmethod
    def parametrize(params, values, **kwargs):
        def decorator(func):
            return func
        return decorator
    
    @staticmethod
    def skip(reason=None):
        def decorator(func):
            return func
        return decorator
    
    @staticmethod
    def xfail(reason=None):
        def decorator(func):
            return func
        return decorator

class Pytest:
    mark = Mark()

pytest = Pytest()


# Now the actual tests
@pytest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (4, 5, 9),
    (10, -5, 5),
])
def test_addition(x, y, expected):
    """Test addition with multiple inputs."""
    assert x + y == expected


@pytest.mark.parametrize("word", ["hello", "world", "fastest"])
def test_length(word):
    """Test string length."""
    assert len(word) > 0


@pytest.mark.parametrize("input,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
    ("Python", "PYTHON"),
])
def test_upper(input, expected):
    """Test string upper method."""
    assert input.upper() == expected


# Multiple parametrize decorators (creates cartesian product)
@pytest.mark.parametrize("x", [1, 2])
@pytest.mark.parametrize("y", [10, 20])
def test_multiply(x, y):
    """Test multiplication with cartesian product."""
    assert x * y == x * y  # Always true, just testing discovery


# Test function that doesn't need parameters
def test_simple():
    """Simple test without parameters."""
    assert True 