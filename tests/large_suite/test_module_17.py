# Test module 17
import pytest

def test_function_17_a():
    assert 17 + 1 == 18

def test_function_17_b():
    assert 17 * 2 == 34

class TestClass17:
    def test_method_17_a(self):
        assert 17 > -1
    
    def test_method_17_b(self):
        assert str(17) == "17"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_17(value):
    assert value > 0
