# Test module 18
import pytest

def test_function_18_a():
    assert 18 + 1 == 19

def test_function_18_b():
    assert 18 * 2 == 36

class TestClass18:
    def test_method_18_a(self):
        assert 18 > -1
    
    def test_method_18_b(self):
        assert str(18) == "18"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_18(value):
    assert value > 0
