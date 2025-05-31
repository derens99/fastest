# Test module 13
import pytest

def test_function_13_a():
    assert 13 + 1 == 14

def test_function_13_b():
    assert 13 * 2 == 26

class TestClass13:
    def test_method_13_a(self):
        assert 13 > -1
    
    def test_method_13_b(self):
        assert str(13) == "13"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_13(value):
    assert value > 0
