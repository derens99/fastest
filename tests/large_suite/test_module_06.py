# Test module 6
import pytest

def test_function_6_a():
    assert 6 + 1 == 7

def test_function_6_b():
    assert 6 * 2 == 12

class TestClass6:
    def test_method_6_a(self):
        assert 6 > -1
    
    def test_method_6_b(self):
        assert str(6) == "6"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_6(value):
    assert value > 0
