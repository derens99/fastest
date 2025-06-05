"""
Tests specifically for assertion introspection functionality.
Tests the ability to show detailed information about failed assertions.
"""
import pytest
from typing import List, Dict, Any, Optional


class TestBasicAssertions:
    """Test basic assertion introspection."""
    
    def test_equality_introspection(self):
        """Test introspection of equality assertions."""
        left = 42
        right = 43
        # Should show: assert 42 == 43
        assert left == right
    
    def test_inequality_introspection(self):
        """Test introspection of inequality assertions."""
        value = 10
        # Should show: assert not (10 != 10)
        assert value != 10
    
    def test_comparison_introspection(self):
        """Test introspection of comparison operators."""
        x = 5
        y = 10
        # Should show: assert 5 > 10
        assert x > y
        # Should show: assert 10 < 5
        assert y < x
        # Should show: assert 5 >= 11
        assert x >= 11
        # Should show: assert 10 <= 9
        assert y <= 9
    
    def test_membership_introspection(self):
        """Test introspection of membership operators."""
        items = [1, 2, 3]
        # Should show: assert 4 in [1, 2, 3]
        assert 4 in items
        # Should show: assert not (2 not in [1, 2, 3])
        assert 2 not in items
    
    def test_identity_introspection(self):
        """Test introspection of identity operators."""
        a = [1, 2, 3]
        b = [1, 2, 3]
        # Should show: assert [1, 2, 3] is [1, 2, 3] (different objects)
        assert a is b
        # Should show: assert not ([1, 2, 3] is not [1, 2, 3])
        c = a
        assert a is not c


class TestComplexAssertions:
    """Test introspection of complex assertions."""
    
    def test_boolean_expressions(self):
        """Test introspection of boolean expressions."""
        x = 5
        y = 10
        z = 15
        # Should show values of all variables
        assert x > 3 and y < 5 and z == 20
        # Should show which part is false
        assert x < 10 or y > 20 or z != 15
    
    def test_function_call_introspection(self):
        """Test introspection of function calls in assertions."""
        def is_even(n):
            return n % 2 == 0
        
        value = 7
        # Should show: assert is_even(7) -> assert False
        assert is_even(value)
    
    def test_method_call_introspection(self):
        """Test introspection of method calls."""
        text = "hello world"
        # Should show: assert 'hello world'.startswith('goodbye') -> assert False
        assert text.startswith("goodbye")
    
    def test_attribute_access_introspection(self):
        """Test introspection of attribute access."""
        class Point:
            def __init__(self, x, y):
                self.x = x
                self.y = y
        
        p = Point(3, 4)
        # Should show: assert p.x == 5 -> assert 3 == 5
        assert p.x == 5
    
    def test_indexing_introspection(self):
        """Test introspection of indexing operations."""
        data = {"key": "value", "number": 42}
        # Should show: assert data['number'] == 43 -> assert 42 == 43
        assert data["number"] == 43
        
        items = [10, 20, 30]
        # Should show: assert items[1] == 25 -> assert 20 == 25
        assert items[1] == 25


class TestCollectionAssertions:
    """Test introspection of collection assertions."""
    
    def test_list_comparison(self):
        """Test detailed list comparison."""
        expected = [1, 2, 3, 4, 5]
        actual = [1, 2, 9, 4, 5]
        # Should show difference at index 2
        assert expected == actual
    
    def test_dict_comparison(self):
        """Test detailed dict comparison."""
        expected = {"a": 1, "b": 2, "c": 3}
        actual = {"a": 1, "b": 99, "d": 4}
        # Should show missing keys, different values
        assert expected == actual
    
    def test_set_comparison(self):
        """Test detailed set comparison."""
        expected = {1, 2, 3, 4}
        actual = {2, 3, 4, 5}
        # Should show items only in expected, only in actual
        assert expected == actual
    
    def test_nested_structure_comparison(self):
        """Test comparison of nested structures."""
        expected = {
            "users": [
                {"id": 1, "name": "Alice", "active": True},
                {"id": 2, "name": "Bob", "active": False}
            ],
            "total": 2
        }
        actual = {
            "users": [
                {"id": 1, "name": "Alice", "active": False},
                {"id": 2, "name": "Bob", "active": False}
            ],
            "total": 2
        }
        # Should show path to difference: users[0].active
        assert expected == actual


class TestStringAssertions:
    """Test introspection of string assertions."""
    
    def test_string_equality(self):
        """Test string equality with diff."""
        expected = "The quick brown fox jumps over the lazy dog"
        actual = "The quick brown cat jumps over the lazy dog"
        # Should highlight the difference (fox vs cat)
        assert expected == actual
    
    def test_multiline_string_diff(self):
        """Test multiline string comparison."""
        expected = """First line
        Second line
        Third line
        Fourth line"""
        
        actual = """First line
        Different second line
        Third line
        Fourth line"""
        # Should show line-by-line diff
        assert expected == actual
    
    def test_string_contains(self):
        """Test string containment assertions."""
        text = "Hello, World!"
        # Should show: assert 'Python' in 'Hello, World!'
        assert "Python" in text
    
    def test_regex_match(self):
        """Test regex matching assertions."""
        import re
        text = "The year is 2024"
        pattern = r"The year is \d{2}$"
        # Should show pattern and text
        assert re.match(pattern, text)


class TestNumericAssertions:
    """Test introspection of numeric assertions."""
    
    def test_float_comparison(self):
        """Test float comparison with tolerance."""
        expected = 3.14159
        actual = 3.14160
        # Should show both values
        assert expected == actual
    
    def test_approximate_equality(self):
        """Test approximate equality assertions."""
        # Should show difference and tolerance
        assert 1.1 + 2.2 == pytest.approx(3.3)
    
    def test_numeric_ranges(self):
        """Test numeric range assertions."""
        value = 15
        # Should show: assert 10 <= 15 <= 20
        assert 10 <= value <= 12


class TestCustomAssertions:
    """Test introspection with custom assertion helpers."""
    
    def test_custom_assertion_function(self):
        """Test custom assertion helper introspection."""
        def assert_valid_email(email):
            assert "@" in email, f"{email} is not a valid email"
            assert "." in email.split("@")[1], f"{email} missing domain extension"
        
        # Should show custom error message
        assert_valid_email("invalid-email")
    
    def test_assertion_with_explanation(self):
        """Test assertions with explanatory messages."""
        x = 10
        y = 20
        # Should show both computed message and values
        assert x > y, f"Expected {x} to be greater than {y}"
    
    def test_complex_assertion_helper(self):
        """Test complex assertion helper."""
        def assert_user_data_valid(user):
            assert "name" in user, "User missing name"
            assert "age" in user, "User missing age"
            assert user["age"] >= 0, f"Invalid age: {user['age']}"
            assert len(user["name"]) > 0, "Name cannot be empty"
        
        invalid_user = {"name": "", "age": 25}
        assert_user_data_valid(invalid_user)


class TestAssertionContext:
    """Test assertion context preservation."""
    
    def test_loop_assertion_context(self):
        """Test assertions in loops show iteration context."""
        items = [1, 2, 3, 4, 5]
        for i, item in enumerate(items):
            # Should show which iteration failed
            assert item < 4, f"Failed at index {i}"
    
    def test_comprehension_assertion_context(self):
        """Test assertions in comprehensions."""
        data = [1, 2, 3, 4, 5]
        # Should indicate which element failed
        assert all(x < 4 for x in data)
    
    def test_nested_call_context(self):
        """Test assertions in nested function calls."""
        def validate(x):
            assert x > 0, f"Value {x} must be positive"
            return x
        
        def process(values):
            return [validate(v) for v in values]
        
        # Should show full call stack context
        process([1, 2, -3, 4])


class TestAssertionSideEffects:
    """Test handling of assertions with side effects."""
    
    def test_assertion_with_mutation(self):
        """Test assertions that mutate state."""
        counter = 0
        def increment():
            nonlocal counter
            counter += 1
            return counter
        
        # Should handle side effects properly
        assert increment() == 2
    
    def test_assertion_with_io(self, tmp_path):
        """Test assertions involving I/O operations."""
        test_file = tmp_path / "test.txt"
        test_file.write_text("content")
        
        # Should show file content in assertion
        assert test_file.read_text() == "different content"