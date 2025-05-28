"""Basic test file without pytest dependency."""


def test_simple():
    """A simple test that should pass."""
    assert 1 + 1 == 2


def test_string():
    """Test string operations."""
    assert "hello".upper() == "HELLO"
    assert "WORLD".lower() == "world"


class TestMath:
    """Test class for math operations."""
    
    def test_addition(self):
        """Test addition."""
        assert 2 + 3 == 5
    
    def test_multiplication(self):
        """Test multiplication."""
        assert 4 * 5 == 20
    


def test_with_print():
    """Test that produces output."""
    print("Hello from test!")
    print("This is stdout")
    assert True


async def test_async():
    """An async test."""
    import asyncio
    await asyncio.sleep(0.01)
    assert True


 