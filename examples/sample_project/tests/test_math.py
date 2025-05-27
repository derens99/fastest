"""Example test file with various test types"""

def test_addition():
    """Test basic addition"""
    assert 1 + 1 == 2

def test_multiplication():
    """Test basic multiplication"""
    assert 3 * 4 == 12

def test_division():
    """Test division"""
    result = 10 / 2
    assert result == 5.0

class TestCalculator:
    """Test class for calculator operations"""
    
    def test_subtract(self):
        """Test subtraction in a class"""
        assert 10 - 3 == 7
    
    def test_power(self):
        """Test power operation"""
        assert 2 ** 3 == 8
    
    def test_modulo(self):
        """Test modulo operation"""
        assert 10 % 3 == 1

# This should not be discovered as a test
def not_a_test():
    return "I'm not a test"

async def test_async_function():
    """An async test function (will be discovered but not run yet)"""
    assert True
