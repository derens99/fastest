"""Check 2: Class-based tests."""


class TestMathOperations:
    def test_add(self):
        assert 1 + 1 == 2

    def test_subtract(self):
        assert 5 - 3 == 2

    def test_multiply(self):
        assert 3 * 4 == 12

    def test_divide(self):
        assert 10 / 2 == 5.0


class TestStringMethods:
    def test_upper(self):
        assert "hello".upper() == "HELLO"

    def test_lower(self):
        assert "WORLD".lower() == "world"

    def test_strip(self):
        assert "  spaced  ".strip() == "spaced"

    def test_split(self):
        assert "a,b,c".split(",") == ["a", "b", "c"]

    def test_join(self):
        assert "-".join(["a", "b", "c"]) == "a-b-c"


class TestNestedLogic:
    def test_list_comprehension(self):
        result = [x**2 for x in range(5)]
        assert result == [0, 1, 4, 9, 16]

    def test_dict_comprehension(self):
        result = {k: v for k, v in zip("abc", [1, 2, 3])}
        assert result == {"a": 1, "b": 2, "c": 3}

    def test_class_fail(self):
        assert False, "intentional class test failure"


# Class without Test prefix should NOT be discovered
class HelperClass:
    def test_should_not_run(self):
        raise RuntimeError("this should never execute")


# Standalone function alongside classes
def test_standalone_with_classes():
    assert True
