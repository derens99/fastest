# Test module 11
import pytest

def test_function_11_a():
    assert 11 + 1 == 12

def test_function_11_b():
    assert 11 * 2 == 22

class TestClass11:
    def test_method_11_a(self):
        assert 11 > -1
    
    def test_method_11_b(self):
        assert str(11) == "11"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_11(value):
    assert value > 0
