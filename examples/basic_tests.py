"""
Basic example tests to demonstrate fastest capabilities.
Run with: fastest examples/basic_tests.py
"""


def test_simple_assertion():
    """Basic test with assertion."""
    assert 1 + 1 == 2


def test_string_operations():
    """Test string concatenation."""
    result = "hello" + " " + "world"
    assert result == "hello world"


class TestMathOperations:
    """Example test class."""
    
    def test_addition(self):
        assert 2 + 3 == 5
    
    def test_multiplication(self):
        assert 4 * 5 == 20
    
    def test_division(self):
        assert 10 / 2 == 5.0


def test_list_operations():
    """Test list operations."""
    numbers = [1, 2, 3]
    numbers.append(4)
    assert len(numbers) == 4
    assert numbers[-1] == 4


def test_dictionary_operations():
    """Test dictionary operations."""
    data = {"name": "fastest", "type": "test runner"}
    assert data["name"] == "fastest"
    assert "type" in data
    assert data.get("missing", "default") == "default"