"""Test class method resolution fix."""

class TestClassMethodFix:
    """Test class to verify class method resolution works."""
    
    def test_simple_method(self):
        """Simple test method that should work."""
        assert True
    
    def test_arithmetic_method(self):
        """Test arithmetic in class method."""
        assert 2 + 2 == 4
    
    def test_string_method(self):
        """Test string operations in class method."""
        text = "fastest"
        assert text.upper() == "FASTEST"

class TestAnotherClass:
    """Another test class."""
    
    def test_boolean_method(self):
        """Test boolean operations."""
        assert True and not False
    
    def test_list_method(self):
        """Test list operations."""
        items = [1, 2, 3]
        assert len(items) == 3

# Mix with function test
def test_function_level():
    """Function level test for comparison."""
    assert True