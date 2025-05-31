# Test module 8
import pytest

def test_function_8_a():
    assert 8 + 1 == 9

def test_function_8_b():
    assert 8 * 2 == 16

class TestClass8:
    def test_method_8_a(self):
        assert 8 > -1
    
    def test_method_8_b(self):
        assert str(8) == "8"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_8(value):
    assert value > 0
