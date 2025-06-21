"""
Example of parametrized tests with Fastest.

This example demonstrates how Fastest supports @pytest.mark.parametrize
for running the same test with different inputs.
"""

import pytest


# Simple parametrized test
@pytest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (4, 5, 9),
    (10, -5, 5),
    (0, 0, 0),
    (-1, -1, -2),
])
def test_addition(x, y, expected):
    """Test addition with multiple inputs."""
    assert x + y == expected


# Parametrized test with single parameter
@pytest.mark.parametrize("word", ["hello", "world", "fastest", "python"])
def test_string_length(word):
    """Test that all words have positive length."""
    assert len(word) > 0
    assert isinstance(word, str)


# Multiple parametrize decorators create cartesian product
@pytest.mark.parametrize("base", [2, 10])
@pytest.mark.parametrize("exponent", [0, 1, 2, 3])
def test_power(base, exponent):
    """Test power calculation with different bases and exponents."""
    result = base ** exponent
    if exponent == 0:
        assert result == 1
    elif exponent == 1:
        assert result == base
    else:
        assert result == base * (base ** (exponent - 1))


# Parametrized test with custom IDs
@pytest.mark.parametrize("input_str,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
    ("Python", "PYTHON"),
    ("fastest", "FASTEST"),
], ids=["lowercase", "lowercase2", "mixed_case", "tool_name"])
def test_uppercase(input_str, expected):
    """Test string uppercase conversion."""
    assert input_str.upper() == expected


# Parametrized test with tuples
@pytest.mark.parametrize("point1,point2,expected_distance", [
    ((0, 0), (3, 4), 5.0),
    ((1, 1), (1, 1), 0.0),
    ((0, 0), (1, 0), 1.0),
    ((0, 0), (0, 1), 1.0),
])
def test_euclidean_distance(point1, point2, expected_distance):
    """Test Euclidean distance calculation."""
    x1, y1 = point1
    x2, y2 = point2
    distance = ((x2 - x1) ** 2 + (y2 - y1) ** 2) ** 0.5
    assert abs(distance - expected_distance) < 0.0001


# Parametrized test with marks
@pytest.mark.parametrize("value,is_positive", [
    (5, True),
    (0, False),
    (-5, False),
    pytest.param(-10, False, marks=pytest.mark.skip(reason="Negative test")),
])
def test_is_positive(value, is_positive):
    """Test positive number check."""
    assert (value > 0) == is_positive


# Parametrized test in a class
class TestStringMethods:
    @pytest.mark.parametrize("s,substring,expected", [
        ("hello world", "world", True),
        ("hello world", "python", False),
        ("fastest", "test", True),
        ("", "", True),
    ])
    def test_contains(self, s, substring, expected):
        """Test string contains method."""
        assert (substring in s) == expected
    
    @pytest.mark.parametrize("s,old,new,expected", [
        ("hello world", "world", "python", "hello python"),
        ("fastest", "est", "EST", "fastEST"),
        ("aaa", "a", "b", "bbb"),
    ])
    def test_replace(self, s, old, new, expected):
        """Test string replace method."""
        assert s.replace(old, new) == expected


if __name__ == "__main__":
    print("Run this file with: fastest examples/test_parametrize_example.py")
    print("This will expand to multiple test cases automatically!") 