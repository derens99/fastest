"""Simple test file for parser comparison."""
import pytest

def test_simple_1():
    assert True

def test_simple_2():
    assert True

def test_simple_3():
    assert True

@pytest.mark.parametrize("x", [1, 2, 3])
def test_param(x):
    assert x > 0

class TestClass:
    def test_method_1(self):
        assert True
    
    def test_method_2(self):
        assert True

@pytest.fixture
def my_fixture():
    return 42

def test_with_fixture(my_fixture):
    assert my_fixture == 42