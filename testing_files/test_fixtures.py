"""Test fixture functionality in Fastest."""

def test_tmp_path_fixture(tmp_path):
    """Test the tmp_path fixture."""
    # tmp_path should be a pathlib.Path
    assert tmp_path.exists()
    assert tmp_path.is_dir()
    
    # Should be able to create files in it
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello, World!")
    assert test_file.read_text() == "Hello, World!"


def test_capsys_fixture(capsys):
    """Test the capsys fixture."""
    print("Hello to stdout")
    import sys
    print("Hello to stderr", file=sys.stderr)
    
    captured = capsys.readouterr()
    assert "Hello to stdout" in captured.out
    assert "Hello to stderr" in captured.err


def test_monkeypatch_fixture(monkeypatch):
    """Test the monkeypatch fixture."""
    import os
    
    # Test setting an environment variable
    monkeypatch.setenv("TEST_VAR", "test_value")
    assert os.environ.get("TEST_VAR") == "test_value"
    
    # Test deleting an environment variable
    monkeypatch.delenv("TEST_VAR", raising=False)
    assert "TEST_VAR" not in os.environ


def test_no_fixtures():
    """Test that works without any fixtures."""
    assert 2 + 2 == 4


class TestFixtureClass:
    """Test fixtures work with classes."""
    
    def test_with_tmp_path(self, tmp_path):
        """Test tmp_path in a class."""
        test_file = tmp_path / "class_test.txt"
        test_file.write_text("class test")
        assert test_file.exists()
    
    def test_multiple_fixtures(self, tmp_path, capsys):
        """Test multiple fixtures in one test."""
        print("Testing multiple fixtures")
        
        test_file = tmp_path / "multi.txt"
        test_file.write_text("multiple")
        
        captured = capsys.readouterr()
        assert "Testing multiple fixtures" in captured.out
        assert test_file.exists()