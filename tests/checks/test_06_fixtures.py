"""Check 6: Built-in fixture support."""

import os


def test_tmp_path(tmp_path):
    """tmp_path should provide a writable temporary directory."""
    p = tmp_path / "test_file.txt"
    p.write_text("hello")
    assert p.read_text() == "hello"
    assert tmp_path.is_dir()


def test_tmp_path_unique(tmp_path):
    """Each test should get a unique tmp_path."""
    marker = tmp_path / "marker.txt"
    marker.write_text("unique")
    assert marker.exists()


def test_capsys(capsys):
    """capsys should capture stdout and stderr."""
    print("hello stdout")
    import sys
    print("hello stderr", file=sys.stderr)
    captured = capsys.readouterr()
    assert "hello stdout" in captured.out
    assert "hello stderr" in captured.err


def test_capsys_multiple_reads(capsys):
    """Multiple readouterr calls should work."""
    print("first")
    cap1 = capsys.readouterr()
    print("second")
    cap2 = capsys.readouterr()
    assert "first" in cap1.out
    assert "second" in cap2.out
    assert "first" not in cap2.out


def test_monkeypatch_setenv(monkeypatch):
    """monkeypatch.setenv should set environment variables."""
    monkeypatch.setenv("FASTEST_TEST_VAR", "hello")
    assert os.environ["FASTEST_TEST_VAR"] == "hello"


def test_monkeypatch_delenv(monkeypatch):
    """monkeypatch.delenv should remove environment variables."""
    os.environ["FASTEST_DEL_TEST"] = "to_delete"
    monkeypatch.delenv("FASTEST_DEL_TEST")
    assert "FASTEST_DEL_TEST" not in os.environ


def test_monkeypatch_setattr(monkeypatch):
    """monkeypatch.setattr should patch attributes."""

    class MyClass:
        value = 42

    monkeypatch.setattr(MyClass, "value", 99)
    assert MyClass.value == 99
