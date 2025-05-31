# Test module 15
import pytest

def test_function_15_a():
    assert 15 + 1 == 16

def test_function_15_b():
    assert 15 * 2 == 30

class TestClass15:
    def test_method_15_a(self):
        assert 15 > -1
    
    def test_method_15_b(self):
        assert str(15) == "15"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_15(value):
    assert value > 0
