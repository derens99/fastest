def test_simple():
    assert 1 + 1 == 2

def test_another():
    assert "hello".upper() == "HELLO"

class TestClass:
    def test_method(self):
        assert [1, 2, 3][1] == 2 