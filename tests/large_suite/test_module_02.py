# Test module 2
import pytest

def test_function_2_a():
    assert 2 + 1 == 3

def test_function_2_b():
    assert 2 * 2 == 4

class TestClass2:
    def test_method_2_a(self):
        assert 2 > -1
    
    def test_method_2_b(self):
        assert str(2) == "2"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_2(value):
    assert value > 0
