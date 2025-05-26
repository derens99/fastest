"""Simple parametrized test for debugging."""

# Mock pytest
class Mark:
    @staticmethod
    def parametrize(params, values, **kwargs):
        def decorator(func):
            return func
        return decorator

class Pytest:
    mark = Mark()

pytest = Pytest()


@pytest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (4, 5, 9),
])
def test_addition(x, y, expected):
    """Test addition with multiple inputs."""
    assert x + y == expected


@pytest.mark.parametrize("word", ["hello", "world"])
def test_length(word):
    """Test string length."""
    assert len(word) > 0 