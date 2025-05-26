"""Test class-based test support in Fastest."""

class TestMath:
    """Test class for math operations."""
    
    def test_addition(self):
        """Test addition."""
        assert 2 + 2 == 4
    
    def test_subtraction(self):
        """Test subtraction."""
        assert 5 - 3 == 2
    
    def test_multiplication(self):
        """Test multiplication."""
        assert 3 * 4 == 12


class TestString:
    """Test class for string operations."""
    
    def test_concatenation(self):
        """Test string concatenation."""
        assert "hello" + " " + "world" == "hello world"
    
    def test_upper(self):
        """Test string upper."""
        assert "hello".upper() == "HELLO"
    
    def test_strip(self):
        """Test string strip."""
        assert "  hello  ".strip() == "hello"


class TestWithoutPrefix:
    """Class without Test prefix - should not be discovered."""
    
    def test_should_not_run(self):
        """This should not be discovered."""
        assert False


def test_standalone_function():
    """Standalone test function."""
    assert True 