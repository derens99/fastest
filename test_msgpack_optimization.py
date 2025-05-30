"""Test the MessagePack optimization"""

def test_simple_addition():
    """Simple test to verify execution works"""
    assert 2 + 2 == 4

def test_with_print():
    """Test that output capture still works"""
    print("Hello from MessagePack!")
    assert True

def test_with_error():
    """Test that errors are properly reported"""
    x = 10
    y = 0
    # This should fail
    assert x == y, f"Expected {x} to equal {y}"

class TestClass:
    def test_method(self):
        """Test class-based tests work"""
        assert "msgpack" != "json"