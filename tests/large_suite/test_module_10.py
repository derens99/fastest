# Test module 10
import pytest

def test_function_10_a():
    assert 10 + 1 == 11

def test_function_10_b():
    assert 10 * 2 == 20

class TestClass10:
    def test_method_10_a(self):
        assert 10 > -1
    
    def test_method_10_b(self):
        assert str(10) == "10"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_10(value):
    assert value > 0
