"""
Example of using Fastest's native parametrize decorator.

This shows that you can use @fastest.mark.parametrize instead of 
@pytest.mark.parametrize for a more native experience.
"""

# Mock fastest module for demonstration
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

class Fastest:
    mark = Mark()

fastest = Fastest()


# Using fastest's native parametrize decorator
@fastest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (4, 5, 9),
    (10, -5, 5),
])
def test_addition(x, y, expected):
    """Test addition with multiple inputs using fastest decorator."""
    assert x + y == expected


@fastest.mark.parametrize("word", ["hello", "world", "fastest"])
def test_length(word):
    """Test string length with fastest decorator."""
    assert len(word) > 0


# You can mix fastest and pytest decorators (for transition period)
@fastest.mark.parametrize("base", [2, 10])
@fastest.mark.parametrize("exp", [0, 1, 2])
def test_power(base, exp):
    """Test power function with fastest decorators."""
    result = base ** exp
    if exp == 0:
        assert result == 1
    else:
        assert result == base ** exp


# Fastest-specific markers work too
@fastest.mark.skip(reason="Not implemented yet")
def test_future_feature():
    """This test will be skipped."""
    assert False


@fastest.mark.xfail
@fastest.mark.parametrize("x", [1, 0, -1])
def test_reciprocal(x):
    """Test that might fail for x=0."""
    assert 1 / x > 0  # Will fail for x=0 and x=-1


if __name__ == "__main__":
    print("This example shows Fastest's native @fastest.mark.parametrize decorator")
    print("Run with: fastest examples/test_fastest_parametrize.py") 