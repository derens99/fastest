"""Test parametrized test functionality."""

import pytest

@pytest.mark.parametrize("value,expected", [
    (1, 2),
    (2, 4), 
    (3, 6),
    (4, 8),
])
def test_multiply_by_two(value, expected):
    """Test multiplication by 2."""
    assert value * 2 == expected


@pytest.mark.parametrize("input_str", [
    "hello",
    "world", 
    "pytest",
    "fastest",
])
def test_string_length(input_str):
    """Test string operations."""
    assert len(input_str) > 0
    assert isinstance(input_str, str)


@pytest.mark.parametrize("x,y,expected", [
    (1, 1, 2),
    (2, 3, 5),
    (5, 5, 10),
    (-1, 1, 0),
])
def test_addition(x, y, expected):
    """Test addition with multiple parameters."""
    assert x + y == expected


class TestParametrizedClass:
    """Test parametrized methods in a class."""
    
    @pytest.mark.parametrize("number", [1, 2, 3, 4, 5])
    def test_positive_numbers(self, number):
        """Test with positive numbers."""
        assert number > 0
    
    @pytest.mark.parametrize("value,power,expected", [
        (2, 2, 4),
        (3, 2, 9),
        (2, 3, 8),
        (5, 2, 25),
    ])
    def test_power(self, value, power, expected):
        """Test power calculations."""
        assert value ** power == expected