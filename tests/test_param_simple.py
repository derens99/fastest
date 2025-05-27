"""Simple parametrized test file without pytest dependency."""

import pytest

# We'll simulate parametrize with decorators that fastest can parse
@pytest.mark.parametrize("x,y,expected", [(2, 3, 5), (4, 5, 9), (10, -5, 5)])
def test_addition(x, y, expected):
    """Test addition with multiple inputs."""
    assert x + y == expected


@pytest.mark.parametrize("word", ["hello", "world", "fastest"])
def test_length(word):
    """Test string length."""
    assert len(word) > 0


@pytest.mark.parametrize("input,expected", [("hello", "HELLO"), ("world", "WORLD")])
def test_upper(input, expected):
    """Test string upper method."""
    assert input.upper() == expected


# Test function that doesn't need parameters
def test_simple():
    """Simple test without parameters."""
    assert True 