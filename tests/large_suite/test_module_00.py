# Test module 0
import pytest

def test_function_0_a():
    assert 0 + 1 == 1

def test_function_0_b():
    assert 0 * 2 == 0

class TestClass0:
    def test_method_0_a(self):
        assert 0 > -1
    
    def test_method_0_b(self):
        assert str(0) == "0"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_0(value):
    assert value > 0
