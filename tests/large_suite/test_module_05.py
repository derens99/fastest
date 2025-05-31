# Test module 5
import pytest

def test_function_5_a():
    assert 5 + 1 == 6

def test_function_5_b():
    assert 5 * 2 == 10

class TestClass5:
    def test_method_5_a(self):
        assert 5 > -1
    
    def test_method_5_b(self):
        assert str(5) == "5"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_5(value):
    assert value > 0
