# Test module 1
import pytest

def test_function_1_a():
    assert 1 + 1 == 2

def test_function_1_b():
    assert 1 * 2 == 2

class TestClass1:
    def test_method_1_a(self):
        assert 1 > -1
    
    def test_method_1_b(self):
        assert str(1) == "1"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_1(value):
    assert value > 0
