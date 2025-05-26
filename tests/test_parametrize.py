"""Test file to verify parametrized test support."""

import pytest


# Simple parametrized test
@pytest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (4, 5, 9),
    (10, -5, 5),
])
def test_addition(x, y, expected):
    """Test addition with multiple inputs."""
    assert x + y == expected


# Parametrized with single parameter
@pytest.mark.parametrize("word", ["hello", "world", "fastest"])
def test_length(word):
    """Test string length."""
    assert len(word) > 0


# Multiple parametrize decorators (creates cartesian product)
@pytest.mark.parametrize("x", [1, 2])
@pytest.mark.parametrize("y", [10, 20])
def test_multiply(x, y):
    """Test multiplication with cartesian product."""
    assert x * y == x * y  # Always true, just testing discovery


# Parametrized test in a class
class TestStringMethods:
    @pytest.mark.parametrize("input,expected", [
        ("hello", "HELLO"),
        ("world", "WORLD"),
        ("Python", "PYTHON"),
    ])
    def test_upper(self, input, expected):
        """Test string upper method."""
        assert input.upper() == expected

    @pytest.mark.parametrize("s,char,count", [
        ("hello", "l", 2),
        ("world", "o", 1),
        ("fastest", "t", 2),
    ])
    def test_count(self, s, char, count):
        """Test string count method."""
        assert s.count(char) == count


# Parametrized with ids
@pytest.mark.parametrize("test_input,expected", [
    (1, 1),
    (2, 4),
    (3, 9),
    (4, 16),
], ids=["one", "two", "three", "four"])
def test_square(test_input, expected):
    """Test square function with custom ids."""
    assert test_input ** 2 == expected


# Parametrized with marks
@pytest.mark.parametrize("value,expected", [
    (5, True),
    pytest.param(0, False, marks=pytest.mark.xfail),
    (-5, False),
])
def test_is_positive(value, expected):
    """Test positive number check."""
    assert (value > 0) == expected


# Complex parametrization with tuples
@pytest.mark.parametrize("coords", [
    ((0, 0), (3, 4), 5.0),
    ((1, 1), (4, 5), 5.0),
    ((0, 0), (0, 5), 5.0),
])
def test_distance(coords):
    """Test distance calculation."""
    (x1, y1), (x2, y2), expected = coords
    distance = ((x2 - x1) ** 2 + (y2 - y1) ** 2) ** 0.5
    assert distance == expected


# Async parametrized test
@pytest.mark.parametrize("delay", [0.001, 0.002, 0.003])
async def test_async_delay(delay):
    """Test async function with parametrization."""
    import asyncio
    start = asyncio.get_event_loop().time()
    await asyncio.sleep(delay)
    elapsed = asyncio.get_event_loop().time() - start
    assert elapsed >= delay


# Test that should fail with some parameters
@pytest.mark.parametrize("dividend,divisor,expected", [
    (10, 2, 5),
    (20, 4, 5),
    (15, 3, 5),
    (10, 0, None),  # This should raise ZeroDivisionError
])
def test_division(dividend, divisor, expected):
    """Test division with edge cases."""
    if divisor == 0:
        with pytest.raises(ZeroDivisionError):
            dividend / divisor
    else:
        assert dividend / divisor == expected 