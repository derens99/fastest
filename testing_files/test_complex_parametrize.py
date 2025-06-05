import pytest

# Test with complex parameter types
@pytest.mark.parametrize("data,expected", [
    ([1, 2, 3], 6),
    ([4, 5, 6], 15),
    ([], 0),
])
def test_list_sum(data, expected):
    assert sum(data) == expected

# Test with dict parameters
@pytest.mark.parametrize("config", [
    {"name": "test1", "value": 100},
    {"name": "test2", "value": 200},
])
def test_dict_param(config):
    assert config["value"] > 0
    assert "name" in config

# Test with None values
@pytest.mark.parametrize("x,y", [
    (1, None),
    (None, 2),
    (3, 4),
])
def test_none_values(x, y):
    if x is None:
        assert y is not None
    if y is None:
        assert x is not None
    if x is not None and y is not None:
        assert x + y > 0

# Test with string parameters containing special characters
@pytest.mark.parametrize("text,expected", [
    ("hello world", 11),
    ("test-case", 9),
    ("foo/bar", 7),
    ("a b c", 5),
])
def test_special_chars(text, expected):
    assert len(text) == expected

# Test indirect parametrization (fixture-based)
@pytest.mark.parametrize("value", [10, 20, 30], indirect=True)
def test_indirect(value):
    # Note: indirect parametrization requires fixture support
    # For now, this will use the value directly
    assert value > 0