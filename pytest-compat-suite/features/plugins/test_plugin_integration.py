"""Test file to verify plugin integration works."""

def test_basic():
    """A simple test to verify plugin hooks are called."""
    assert 1 + 1 == 2

def test_with_print():
    """Test that exercises output capture."""
    print("Hello from test!")
    assert True

class TestPluginIntegration:
    """Test class to verify class-based test support with plugins."""
    
    def test_in_class(self):
        """Test method in a class."""
        assert 2 * 2 == 4
    
    def test_class_with_output(self):
        """Test that prints in a class."""
        print("Output from class test")
        assert "plugin" in "plugin system works"