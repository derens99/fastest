# Test module 9
import pytest

def test_function_9_a():
    assert 9 + 1 == 10

def test_function_9_b():
    assert 9 * 2 == 18

class TestClass9:
    def test_method_9_a(self):
        assert 9 > -1
    
    def test_method_9_b(self):
        assert str(9) == "9"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_9(value):
    assert value > 0
