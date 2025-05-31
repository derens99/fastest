# Test module 14
import pytest

def test_function_14_a():
    assert 14 + 1 == 15

def test_function_14_b():
    assert 14 * 2 == 28

class TestClass14:
    def test_method_14_a(self):
        assert 14 > -1
    
    def test_method_14_b(self):
        assert str(14) == "14"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_14(value):
    assert value > 0
