"""Check 10: Error reporting quality."""


def test_assertion_error_basic():
    """Should show clear assertion error."""
    assert 1 == 2


def test_assertion_error_message():
    """Should show custom message."""
    assert False, "this is a custom failure message"


def test_assertion_with_expression():
    """Should show the compared values."""
    x = [1, 2, 3]
    y = [1, 2, 4]
    assert x == y


def test_name_error():
    """Should report NameError clearly."""
    _ = undefined_variable  # noqa: F821


def test_attribute_error():
    """Should report AttributeError."""
    "hello".nonexistent_method()


def test_index_error():
    """Should report IndexError."""
    lst = [1, 2, 3]
    _ = lst[10]


def test_key_error():
    """Should report KeyError."""
    d = {"a": 1}
    _ = d["missing"]


def test_zero_division():
    """Should report ZeroDivisionError."""
    _ = 1 / 0


def test_value_error():
    """Should report ValueError."""
    int("not_a_number")


def test_import_error():
    """Should report ImportError."""
    import nonexistent_module_12345  # noqa: F401


def test_deep_traceback():
    """Should show full traceback."""
    def level3():
        raise RuntimeError("deep error")

    def level2():
        level3()

    def level1():
        level2()

    level1()
