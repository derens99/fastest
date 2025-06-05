"""
Comprehensive Test Suite - Parametrization
Tests for complex parametrization scenarios with various data types
"""

import pytest


# Basic parametrization
@pytest.mark.parametrize("value", [1, 2, 3, 4, 5])
def test_simple_parametrize(value):
    """Basic parametrized test with integers"""
    assert 1 <= value <= 5


@pytest.mark.parametrize("string", ["hello", "world", "pytest", "fastest"])
def test_parametrize_strings(string):
    """Parametrized test with strings"""
    assert len(string) >= 5


@pytest.mark.parametrize("boolean", [True, False])
def test_parametrize_booleans(boolean):
    """Parametrized test with booleans"""
    assert isinstance(boolean, bool)


# Multiple parameters
@pytest.mark.parametrize("x,y,expected", [
    (1, 2, 3),
    (2, 3, 5),
    (3, 4, 7),
    (0, 0, 0),
    (-1, 1, 0),
])
def test_multiple_parameters(x, y, expected):
    """Test with multiple parameters"""
    assert x + y == expected


@pytest.mark.parametrize("a,b,c", [
    (1, 2, 3),
    (4, 5, 6),
    (7, 8, 9),
])
def test_three_parameters(a, b, c):
    """Test with three parameters"""
    assert a < b < c


# Complex data types
@pytest.mark.parametrize("data", [
    [1, 2, 3],
    [4, 5, 6, 7],
    [],
    [100],
])
def test_parametrize_lists(data):
    """Parametrized test with lists"""
    assert isinstance(data, list)
    if data:
        assert all(isinstance(x, int) for x in data)


@pytest.mark.parametrize("mapping", [
    {"key": "value"},
    {"a": 1, "b": 2},
    {},
    {"nested": {"inner": "value"}},
])
def test_parametrize_dicts(mapping):
    """Parametrized test with dictionaries"""
    assert isinstance(mapping, dict)


@pytest.mark.parametrize("value", [None, 0, "", [], {}])
def test_parametrize_falsy_values(value):
    """Test with various falsy values"""
    assert not bool(value)


@pytest.mark.parametrize("mixed", [
    1,
    "string",
    [1, 2, 3],
    {"key": "value"},
    None,
    True,
    3.14,
])
def test_parametrize_mixed_types(mixed):
    """Test with mixed data types"""
    assert mixed is not None or mixed is None  # Always true


# Parametrize with IDs
@pytest.mark.parametrize("value", [1, 2, 3], ids=["one", "two", "three"])
def test_parametrize_with_ids(value):
    """Test with custom parameter IDs"""
    assert value in [1, 2, 3]


@pytest.mark.parametrize("case", [
    {"input": 5, "expected": 25},
    {"input": -3, "expected": 9},
    {"input": 0, "expected": 0},
], ids=["positive", "negative", "zero"])
def test_parametrize_dict_with_ids(case):
    """Test with dict parameters and custom IDs"""
    assert case["input"] ** 2 == case["expected"]


# Nested parametrization
@pytest.mark.parametrize("x", [1, 2])
@pytest.mark.parametrize("y", [10, 20])
@pytest.mark.parametrize("z", [100, 200])
def test_nested_parametrize(x, y, z):
    """Test with nested parametrization (creates 8 test cases)"""
    assert x < y < z


# Parametrize with marks
@pytest.mark.parametrize("value,expected", [
    (1, 1),
    pytest.param(0, 0, marks=pytest.mark.xfail),
    (-1, 1),
    pytest.param(2, 3, marks=pytest.mark.skip(reason="Skip this case")),
])
def test_parametrize_with_marks(value, expected):
    """Parametrized test with marks on specific cases"""
    assert value == expected


# Class-based parametrization
@pytest.mark.parametrize("value", [10, 20, 30])
class TestParametrizedClass:
    """Class where all methods are parametrized"""
    
    def test_method_1(self, value):
        """First method gets parametrized"""
        assert value > 0
    
    def test_method_2(self, value):
        """Second method also gets parametrized"""
        assert value % 10 == 0
    
    @pytest.mark.parametrize("multiplier", [1, 2])
    def test_method_with_extra_param(self, value, multiplier):
        """Method with additional parametrization"""
        result = value * multiplier
        assert result >= value


# Edge cases and special values
@pytest.mark.parametrize("special", [
    float('inf'),
    float('-inf'),
    float('nan'),
])
def test_parametrize_special_floats(special):
    """Test with special float values"""
    assert isinstance(special, float)


@pytest.mark.parametrize("unicode", [
    "Hello 世界",
    "🐍 Python",
    "Ñoño",
    "Здравствуй",
])
def test_parametrize_unicode(unicode):
    """Test with Unicode strings"""
    assert isinstance(unicode, str)
    assert len(unicode) > 0


@pytest.mark.parametrize("escape_chars", [
    "line1\nline2",
    "tab\there",
    "quote\"inside",
    "backslash\\here",
])
def test_parametrize_escape_sequences(escape_chars):
    """Test with escape sequences"""
    assert isinstance(escape_chars, str)


# Large parametrization sets
@pytest.mark.parametrize("number", range(50))
def test_large_parametrize_set(number):
    """Test with many parameter values"""
    assert 0 <= number < 50


# Parametrize with fixtures
@pytest.fixture
def base_value():
    return 10


@pytest.mark.parametrize("multiplier", [1, 2, 3])
def test_parametrize_with_fixture(base_value, multiplier):
    """Test combining parametrization with fixtures"""
    result = base_value * multiplier
    assert result == 10 * multiplier


# Complex nested structures
@pytest.mark.parametrize("complex_data", [
    {"users": [{"name": "Alice", "age": 30}], "count": 1},
    {"users": [{"name": "Bob", "age": 25}, {"name": "Charlie", "age": 35}], "count": 2},
    {"users": [], "count": 0},
])
def test_parametrize_complex_structures(complex_data):
    """Test with complex nested data structures"""
    assert len(complex_data["users"]) == complex_data["count"]
    for user in complex_data["users"]:
        assert "name" in user
        assert "age" in user


# Parametrize with callables
def generate_test_data():
    """Generator function for test data"""
    return [(i, i**2) for i in range(5)]


@pytest.mark.parametrize("input,expected", generate_test_data())
def test_parametrize_from_function(input, expected):
    """Test with parameters from function call"""
    assert input ** 2 == expected


# Indirect parametrization (requires fixture)
@pytest.fixture
def squared_value(request):
    """Fixture that squares the parameter"""
    return request.param ** 2


@pytest.mark.parametrize("squared_value", [2, 3, 4], indirect=True)
def test_indirect_parametrize(squared_value):
    """Test with indirect parametrization through fixture"""
    assert squared_value in [4, 9, 16]


# Empty and None parameters
@pytest.mark.parametrize("empty", [
    [],
    (),
    {},
    set(),
    "",
])
def test_empty_containers(empty):
    """Test with empty containers"""
    assert len(empty) == 0


@pytest.mark.parametrize("none_values", [
    None,
    [None],
    {"key": None},
    (None, None),
])
def test_none_parameters(none_values):
    """Test with None values"""
    if isinstance(none_values, (list, dict, tuple)):
        assert None in none_values or None in none_values.values()
    else:
        assert none_values is None


# Parametrize with very long values
@pytest.mark.parametrize("long_string", [
    "a" * 100,
    "b" * 200,
    "test" * 50,
])
def test_long_string_parameters(long_string):
    """Test with very long string parameters"""
    assert len(long_string) >= 100


# Combined parametrization patterns
@pytest.mark.parametrize("operation,x,y,expected", [
    ("add", 1, 2, 3),
    ("subtract", 5, 3, 2),
    ("multiply", 4, 3, 12),
    ("divide", 10, 2, 5),
])
def test_operation_parametrize(operation, x, y, expected):
    """Test different operations with parametrization"""
    if operation == "add":
        assert x + y == expected
    elif operation == "subtract":
        assert x - y == expected
    elif operation == "multiply":
        assert x * y == expected
    elif operation == "divide":
        assert x / y == expected