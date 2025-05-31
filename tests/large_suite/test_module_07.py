# Test module 7
import pytest

def test_function_7_a():
    assert 7 + 1 == 8

def test_function_7_b():
    assert 7 * 2 == 14

class TestClass7:
    def test_method_7_a(self):
        assert 7 > -1
    
    def test_method_7_b(self):
        assert str(7) == "7"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_7(value):
    assert value > 0
