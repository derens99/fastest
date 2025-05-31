"""
Real-world test patterns for compatibility testing

This test suite contains common patterns found in real pytest codebases
to ensure fastest maintains compatibility during development.
"""

import pytest
import asyncio
import tempfile
import os
from pathlib import Path
from unittest.mock import Mock, patch


class TestBasicPatterns:
    """Basic test patterns that should always work"""
    
    def test_simple_assertion(self):
        assert 1 + 1 == 2
    
    def test_string_operations(self):
        text = "hello world"
        assert text.upper() == "HELLO WORLD"
        assert len(text) == 11
    
    def test_list_operations(self):
        items = [1, 2, 3, 4, 5]
        assert len(items) == 5
        assert sum(items) == 15
        assert max(items) == 5


class TestParametrizedPatterns:
    """Parametrized test patterns"""
    
    @pytest.mark.parametrize("input_val,expected", [
        (1, 2),
        (2, 4),
        (3, 6),
        (10, 20)
    ])
    def test_double_number(self, input_val, expected):
        assert input_val * 2 == expected
    
    @pytest.mark.parametrize("text,expected_length", [
        ("", 0),
        ("a", 1),
        ("hello", 5),
        ("hello world", 11)
    ])
    def test_string_length(self, text, expected_length):
        assert len(text) == expected_length
    
    @pytest.mark.parametrize("numbers,expected_sum", [
        ([1, 2, 3], 6),
        ([10, 20], 30),
        ([], 0),
        ([5], 5)
    ])
    def test_sum_numbers(self, numbers, expected_sum):
        assert sum(numbers) == expected_sum


class TestFixturePatterns:
    """Common fixture usage patterns"""
    
    def test_tmp_path_fixture(self, tmp_path):
        """Test tmp_path built-in fixture"""
        test_file = tmp_path / "test.txt"
        test_file.write_text("hello world")
        assert test_file.read_text() == "hello world"
        assert test_file.exists()
    
    def test_capsys_fixture(self, capsys):
        """Test capsys built-in fixture"""
        print("Hello, World!")
        captured = capsys.readouterr()
        assert "Hello, World!" in captured.out
    
    def test_monkeypatch_fixture(self, monkeypatch):
        """Test monkeypatch built-in fixture"""
        monkeypatch.setenv("TEST_VAR", "test_value")
        assert os.environ.get("TEST_VAR") == "test_value"


class TestAsyncPatterns:
    """Async test patterns"""
    
    async def test_simple_async(self):
        """Basic async test"""
        result = await self.async_function()
        assert result == "async_result"
    
    async def async_function(self):
        await asyncio.sleep(0.001)  # Minimal sleep
        return "async_result"
    
    @pytest.mark.parametrize("delay,expected", [
        (0.001, "fast"),
        (0.002, "medium"),
    ])
    async def test_parametrized_async(self, delay, expected):
        """Parametrized async test"""
        result = await self.timed_function(delay)
        assert expected in result
    
    async def timed_function(self, delay):
        await asyncio.sleep(delay)
        if delay < 0.002:
            return "fast_result"
        else:
            return "medium_result"


class TestMarkerPatterns:
    """Test marker patterns"""
    
    @pytest.mark.skip(reason="Testing skip marker")
    def test_skipped(self):
        assert False, "This should never run"
    
    @pytest.mark.xfail(reason="Expected to fail")
    def test_expected_failure(self):
        assert False, "This is expected to fail"
    
    @pytest.mark.slow
    def test_custom_marker(self):
        """Test with custom marker"""
        assert True


class TestErrorPatterns:
    """Error handling patterns"""
    
    def test_exception_raised(self):
        """Test that exception is raised"""
        with pytest.raises(ValueError):
            raise ValueError("Expected error")
    
    def test_exception_message(self):
        """Test exception with specific message"""
        with pytest.raises(ValueError, match="specific message"):
            raise ValueError("specific message here")
    
    def test_no_exception(self):
        """Test that no exception is raised"""
        try:
            result = 1 + 1
            assert result == 2
        except Exception:
            pytest.fail("Unexpected exception raised")


class TestComplexPatterns:
    """More complex real-world patterns"""
    
    def test_multiple_assertions(self):
        """Multiple assertions in one test"""
        data = {"name": "test", "value": 42, "active": True}
        
        assert "name" in data
        assert data["name"] == "test"
        assert data["value"] > 0
        assert data["active"] is True
        assert len(data) == 3
    
    def test_nested_data_structures(self):
        """Working with nested data"""
        users = [
            {"id": 1, "name": "Alice", "roles": ["admin", "user"]},
            {"id": 2, "name": "Bob", "roles": ["user"]},
            {"id": 3, "name": "Charlie", "roles": ["guest"]}
        ]
        
        assert len(users) == 3
        assert users[0]["name"] == "Alice"
        assert "admin" in users[0]["roles"]
        assert len(users[1]["roles"]) == 1
        
        # Find user by condition
        admin_users = [u for u in users if "admin" in u["roles"]]
        assert len(admin_users) == 1
        assert admin_users[0]["name"] == "Alice"
    
    def test_file_operations(self, tmp_path):
        """File system operations"""
        # Create directory structure
        subdir = tmp_path / "subdir"
        subdir.mkdir()
        
        # Create files
        file1 = tmp_path / "file1.txt"
        file2 = subdir / "file2.txt"
        
        file1.write_text("content1")
        file2.write_text("content2")
        
        # Test operations
        assert file1.exists()
        assert file2.exists()
        assert file1.read_text() == "content1"
        assert file2.read_text() == "content2"
        
        # List files
        all_files = list(tmp_path.rglob("*.txt"))
        assert len(all_files) == 2


# Module-level functions (not in class)
def test_module_level_function():
    """Test at module level"""
    assert True


@pytest.mark.parametrize("x,y", [(1, 2), (3, 4)])
def test_module_level_parametrized(x, y):
    """Parametrized test at module level"""
    assert x < y


async def test_module_level_async():
    """Async test at module level"""
    await asyncio.sleep(0.001)
    assert True


# Fixtures for this module
@pytest.fixture
def sample_data():
    """Custom fixture returning sample data"""
    return {
        "users": ["alice", "bob", "charlie"],
        "numbers": [1, 2, 3, 4, 5],
        "config": {"debug": True, "version": "1.0"}
    }


@pytest.fixture
def mock_service():
    """Mock service fixture"""
    service = Mock()
    service.get_data.return_value = {"status": "ok", "data": [1, 2, 3]}
    service.is_active.return_value = True
    return service


class TestCustomFixtures:
    """Tests using custom fixtures"""
    
    def test_sample_data_fixture(self, sample_data):
        """Test using custom sample_data fixture"""
        assert len(sample_data["users"]) == 3
        assert "alice" in sample_data["users"]
        assert sample_data["config"]["debug"] is True
    
    def test_mock_service_fixture(self, mock_service):
        """Test using mock service fixture"""
        data = mock_service.get_data()
        assert data["status"] == "ok"
        assert len(data["data"]) == 3
        assert mock_service.is_active()
        
        # Verify mock was called
        mock_service.get_data.assert_called_once()
        mock_service.is_active.assert_called_once()


class TestEdgeCases:
    """Edge cases and potential compatibility issues"""
    
    def test_empty_test(self):
        """Completely empty test"""
        pass
    
    def test_only_assertion(self):
        """Test with only assertion"""
        assert True
    
    def test_multiple_classes_same_method_name(self):
        """Method name that might conflict"""
        assert True


class TestSecondClass:
    """Second class with same method name as TestEdgeCases"""
    
    def test_multiple_classes_same_method_name(self):
        """Same method name, different class"""
        assert True


# Test with same name as class method but at module level
def test_multiple_classes_same_method_name():
    """Function with same name as class methods"""
    assert True