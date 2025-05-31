# Test module 3
import pytest

def test_function_3_a():
    assert 3 + 1 == 4

def test_function_3_b():
    assert 3 * 2 == 6

class TestClass3:
    def test_method_3_a(self):
        assert 3 > -1
    
    def test_method_3_b(self):
        assert str(3) == "3"

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_3(value):
    assert value > 0
