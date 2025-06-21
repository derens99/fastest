"""
Tests for enhanced error reporting features.
Tests assertion introspection, better tracebacks, and error messages.
"""
import pytest
import sys
import os
from pathlib import Path


class TestAssertionIntrospection:
    """Test assertion introspection features."""
    
    def test_simple_assertion_message(self):
        """Test that simple assertions show values."""
        x = 5
        y = 10
        # Should show: assert 5 == 10
        assert x == y, "Values should be equal"
    
    def test_assertion_with_expressions(self):
        """Test assertions with complex expressions."""
        data = {"key": "value", "count": 42}
        # Should show the actual dictionary contents
        assert data["count"] == 43
    
    def test_assertion_with_operators(self):
        """Test assertions with different operators."""
        x = [1, 2, 3]
        y = [1, 2, 4]
        # Should show list contents and differences
        assert x == y
    
    def test_assertion_in_comprehension(self):
        """Test assertion in list comprehension."""
        items = [1, 2, 3, 4, 5]
        # Should show which item failed
        assert all(x < 4 for x in items)
    
    def test_assertion_with_long_strings(self):
        """Test assertion with long string differences."""
        expected = "This is a very long string that has some content"
        actual = "This is a very long string that has different content"
        # Should show a diff
        assert expected == actual
    
    def test_assertion_with_multiline_strings(self):
        """Test assertion with multiline strings."""
        expected = """Line 1
        Line 2
        Line 3"""
        actual = """Line 1
        Line B
        Line 3"""
        # Should show line-by-line diff
        assert expected == actual
    
    def test_assertion_with_custom_objects(self):
        """Test assertion with custom objects."""
        class Person:
            def __init__(self, name, age):
                self.name = name
                self.age = age
            
            def __repr__(self):
                return f"Person(name={self.name!r}, age={self.age})"
        
        p1 = Person("Alice", 30)
        p2 = Person("Bob", 30)
        # Should show repr() of objects
        assert p1 == p2


class TestEnhancedTracebacks:
    """Test enhanced traceback formatting."""
    
    def test_traceback_with_locals(self):
        """Test traceback shows local variables."""
        def inner_function(x, y):
            result = x * y
            temp = result + 10
            assert temp == 100  # This will fail
        
        inner_function(5, 10)
    
    def test_traceback_with_source_context(self):
        """Test traceback shows surrounding code."""
        def complex_function():
            data = [1, 2, 3]
            for i, item in enumerate(data):
                if i == 2:
                    # Should show this context
                    assert item == 4
        
        complex_function()
    
    def test_traceback_filtering(self):
        """Test traceback filters internal frames."""
        # Internal test framework frames should be hidden
        assert False
    
    def test_traceback_with_chained_exceptions(self):
        """Test traceback with exception chains."""
        try:
            raise ValueError("First error")
        except ValueError:
            raise RuntimeError("Second error") from None


class TestErrorMessages:
    """Test improved error messages."""
    
    def test_type_error_messages(self):
        """Test helpful messages for type errors."""
        # Should suggest correct type
        assert isinstance("string", int)
    
    def test_attribute_error_messages(self):
        """Test helpful messages for attribute errors."""
        obj = {"key": "value"}
        # Should suggest using [] instead of .
        assert obj.key == "value"
    
    def test_key_error_messages(self):
        """Test helpful messages for key errors."""
        data = {"name": "Alice", "age": 30}
        # Should show available keys
        assert data["height"] == 170
    
    def test_import_error_messages(self):
        """Test helpful messages for import errors."""
        # Should suggest package installation
        import nonexistent_module


class TestDiffDisplay:
    """Test diff display for various types."""
    
    def test_list_diff(self):
        """Test diff display for lists."""
        expected = [1, 2, 3, 4, 5]
        actual = [1, 2, 9, 4, 5]
        assert expected == actual
    
    def test_dict_diff(self):
        """Test diff display for dictionaries."""
        expected = {"a": 1, "b": 2, "c": 3}
        actual = {"a": 1, "b": 99, "d": 4}
        assert expected == actual
    
    def test_set_diff(self):
        """Test diff display for sets."""
        expected = {1, 2, 3, 4}
        actual = {2, 3, 4, 5}
        assert expected == actual
    
    def test_tuple_diff(self):
        """Test diff display for tuples."""
        expected = (1, "a", True)
        actual = (1, "b", True)
        assert expected == actual
    
    def test_nested_structure_diff(self):
        """Test diff for nested structures."""
        expected = {
            "users": [
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": 25}
            ],
            "count": 2
        }
        actual = {
            "users": [
                {"name": "Alice", "age": 31},
                {"name": "Bob", "age": 25}
            ],
            "count": 2
        }
        assert expected == actual


class TestColoredOutput:
    """Test colored output for better readability."""
    
    def test_colored_assertions(self):
        """Test assertions are colored."""
        # Red for failures, green for passes
        assert True
        assert False  # Should be in red
    
    def test_colored_diffs(self):
        """Test diffs are colored."""
        # Added lines in green, removed in red
        assert "hello" == "world"
    
    def test_colored_tracebacks(self):
        """Test tracebacks are colored."""
        # File paths, line numbers, etc. in different colors
        raise ValueError("Colored error")
    
    def test_colored_markers(self):
        """Test markers are colored in output."""
        # SKIP in yellow, XFAIL in orange, etc.
        pytest.skip("Colored skip message")


class TestContextualErrors:
    """Test errors with helpful context."""
    
    def test_fixture_not_found_error(self):
        """Test helpful error when fixture not found."""
        @pytest.fixture
        def my_fixture():
            return 42
        
        # Should suggest available fixtures
        def test_uses_wrong_fixture(wrong_fixture):
            pass
    
    def test_marker_not_found_error(self):
        """Test helpful error when marker not found."""
        # Should suggest available markers
        @pytest.mark.nonexistent_marker
        def test_with_bad_marker():
            pass
    
    def test_parametrize_error(self):
        """Test helpful error in parametrize."""
        # Should show which parameter failed
        @pytest.mark.parametrize("x,y", [(1, 2), (3,)])  # Missing value
        def test_parametrize(x, y):
            assert x + y > 0


class TestVerboseErrors:
    """Test verbose error reporting modes."""
    
    def test_verbose_level_1(self):
        """Test -v shows more details."""
        assert 1 == 2
    
    def test_verbose_level_2(self):
        """Test -vv shows even more details."""
        data = list(range(100))
        assert data[50] == 51
    
    def test_quiet_mode(self):
        """Test -q shows minimal details."""
        assert False


class TestErrorSummary:
    """Test error summary at end of test run."""
    
    def test_failure_summary(self):
        """Test summary of all failures."""
        assert False, "First failure"
    
    def test_error_summary(self):
        """Test summary of all errors."""
        raise RuntimeError("Test error")
    
    def test_skip_summary(self):
        """Test summary includes skips."""
        pytest.skip("Skip reason")
    
    def test_xfail_summary(self):
        """Test summary includes xfails."""
        pytest.xfail("Expected failure")