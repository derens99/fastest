"""Test enhanced assertion introspection capabilities"""

import pytest
from typing import List, Dict, Set


def test_simple_equality_assertion():
    """Test basic equality assertion with introspection"""
    x = 5
    y = 10
    assert x == y  # Should show: 5 == 10 is False


def test_complex_comparison():
    """Test complex comparison with multiple operators"""
    a = 5
    b = 10
    c = 15
    assert a < b < c < 20  # Chained comparison


def test_string_diff():
    """Test string difference visualization"""
    expected = """Hello World
This is a test
With multiple lines
And some content"""
    
    actual = """Hello World
This is a test
With different lines
And some content"""
    
    assert actual == expected  # Should show diff


def test_list_diff():
    """Test list difference visualization"""
    expected = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    actual = [1, 2, 3, 5, 6, 7, 8, 9, 11]
    assert actual == expected  # Should show missing 4, 10 and extra 11


def test_dict_diff():
    """Test dictionary difference visualization"""
    expected = {
        'name': 'John',
        'age': 30,
        'city': 'New York',
        'hobbies': ['reading', 'coding']
    }
    
    actual = {
        'name': 'John',
        'age': 31,
        'city': 'Boston',
        'extra': 'field'
    }
    
    assert actual == expected  # Should show all differences


def test_set_diff():
    """Test set difference visualization"""
    expected = {1, 2, 3, 4, 5}
    actual = {1, 3, 5, 7, 9}
    assert actual == expected  # Should show missing and extra elements


def test_in_operator():
    """Test 'in' operator assertion"""
    items = ['apple', 'banana', 'orange']
    item = 'grape'
    assert item in items  # Should show item and list contents


def test_not_assertion():
    """Test 'not' assertion"""
    value = [1, 2, 3]
    assert not value  # Should show that list is truthy


def test_boolean_and():
    """Test boolean AND operation"""
    x = 5
    y = 0
    z = 10
    assert x and y and z  # Should show which value is falsy


def test_boolean_or():
    """Test boolean OR operation"""
    a = None
    b = False
    c = []
    assert a or b or c  # Should show all values are falsy


def test_function_call_assertion():
    """Test assertion on function call result"""
    def is_even(n):
        return n % 2 == 0
    
    number = 7
    assert is_even(number)  # Should show function call and result


def test_custom_assertion_message():
    """Test custom assertion message handling"""
    x = 10
    y = 20
    assert x > y, f"Expected {x} to be greater than {y}"  # Custom message


def test_complex_expression():
    """Test complex expression evaluation"""
    data = {'users': [{'name': 'Alice', 'age': 25}, {'name': 'Bob', 'age': 30}]}
    assert data['users'][0]['age'] > 30  # Should show evaluated value


def test_local_variables():
    """Test local variable display"""
    def calculate(a, b):
        temp = a * 2
        result = temp + b
        expected = 50
        assert result == expected  # Should show temp, result, expected
    
    calculate(10, 15)


def test_isinstance_assertion():
    """Test isinstance assertion"""
    value = "hello"
    assert isinstance(value, int)  # Should show type mismatch


def test_length_assertion():
    """Test length comparison"""
    text = "Hello"
    assert len(text) == 10  # Should show actual length


def test_nested_structures():
    """Test nested data structure comparison"""
    expected = {
        'data': {
            'items': [
                {'id': 1, 'value': 'a'},
                {'id': 2, 'value': 'b'},
                {'id': 3, 'value': 'c'}
            ],
            'meta': {'count': 3}
        }
    }
    
    actual = {
        'data': {
            'items': [
                {'id': 1, 'value': 'a'},
                {'id': 2, 'value': 'x'},  # Different value
                {'id': 3, 'value': 'c'}
            ],
            'meta': {'count': 3}
        }
    }
    
    assert actual == expected  # Should show nested difference


def test_exception_in_assertion():
    """Test handling of exceptions during assertion evaluation"""
    def may_fail():
        raise ValueError("Intentional error")
    
    # This should handle the exception gracefully
    assert may_fail() == "success"


class TestClassAssertions:
    """Test assertions in class methods"""
    
    def setup_method(self):
        self.data = [1, 2, 3, 4, 5]
    
    def test_class_instance_assertion(self):
        """Test assertion with instance variables"""
        expected_sum = 20
        actual_sum = sum(self.data)
        assert actual_sum == expected_sum  # Should show self.data
    
    def test_method_with_params(self, value=10):
        """Test assertion in method with parameters"""
        result = value * 2
        assert result == 25  # Should show parameter value


def test_multiline_assertion():
    """Test multiline assertion handling"""
    long_list = list(range(100))
    assert (
        len(long_list) > 200 and
        sum(long_list) < 1000 and
        max(long_list) == 50
    )  # Should show each condition


@pytest.mark.parametrize("input,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
    ("test", "TEST"),
])
def test_parametrized_assertion(input, expected):
    """Test assertion in parametrized test"""
    result = input.upper()
    assert result == expected.lower()  # Will fail, should show values


def test_unicode_in_assertion():
    """Test Unicode handling in assertions"""
    emoji = "🚀"
    text = "rocket"
    assert emoji == text  # Should handle Unicode properly


def test_large_collection_truncation():
    """Test truncation of large collections"""
    large_list = list(range(1000))
    expected = list(range(500))
    assert large_list == expected  # Should truncate output


def test_custom_object_assertion():
    """Test assertion with custom objects"""
    class Point:
        def __init__(self, x, y):
            self.x = x
            self.y = y
        
        def __repr__(self):
            return f"Point({self.x}, {self.y})"
        
        def __eq__(self, other):
            return self.x == other.x and self.y == other.y
    
    p1 = Point(10, 20)
    p2 = Point(10, 30)
    assert p1 == p2  # Should show custom repr