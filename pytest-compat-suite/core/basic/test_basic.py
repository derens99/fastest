"""Basic test file for comparing fastest vs pytest"""

def test_simple_pass():
    """Simple passing test"""
    assert 1 + 1 == 2

def test_string_operations():
    """Test string operations"""
    text = "hello world"
    assert len(text) == 11
    assert text.upper() == "HELLO WORLD"
    assert "world" in text

def test_math_operations():
    """Test basic math"""
    assert 2 * 3 == 6
    assert 10 / 2 == 5
    assert 2 ** 3 == 8

def test_list_operations():
    """Test list operations"""
    numbers = [1, 2, 3, 4, 5]
    assert len(numbers) == 5
    assert numbers[0] == 1
    assert numbers[-1] == 5
    assert sum(numbers) == 15

def test_dict_operations():
    """Test dictionary operations"""
    data = {"name": "test", "value": 42}
    assert data["name"] == "test"
    assert data["value"] == 42
    assert len(data) == 2