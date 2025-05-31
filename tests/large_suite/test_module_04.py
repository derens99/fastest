# Test module 4
import pytest

def test_function_4_a():
    assert 4 + 1 == 5

def test_function_4_b():
    assert 4 * 2 == 8

class TestClass4:
    def test_method_4_a(self):
        assert 4 > -1
    
    def test_method_4_b(self):
        assert str(4) == "4"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_4(value):
    assert value > 0
