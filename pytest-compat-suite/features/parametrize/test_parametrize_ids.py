import pytest

@pytest.mark.parametrize("x", [1, 2, 3])
def test_simple_param(x):
    assert x > 0

@pytest.mark.parametrize("x,y", [(1, 2), (3, 4), (5, 6)])
def test_tuple_param(x, y):
    assert x < y

@pytest.mark.parametrize("name", ["alice", "bob", "charlie"])
def test_string_param(name):
    assert len(name) > 0

@pytest.mark.parametrize("value", [True, False, None])
def test_mixed_param(value):
    pass

@pytest.mark.parametrize("x", [1, 2], ids=["first", "second"])
def test_with_ids(x):
    assert x > 0

@pytest.mark.parametrize("x", [
    pytest.param(1, id="one"),
    pytest.param(2, id="two"),
    pytest.param(3, id="three")
])
def test_pytest_param(x):
    assert x > 0

class TestParametrized:
    @pytest.mark.parametrize("x", [10, 20, 30])
    def test_class_param(self, x):
        assert x > 5