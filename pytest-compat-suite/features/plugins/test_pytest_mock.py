"""
Tests for pytest-mock compatibility layer.
Tests the mocker fixture and mock functionality.
"""
import pytest
import os
import sys
from unittest.mock import Mock, MagicMock, patch, call


class TestMockerFixture:
    """Test the mocker fixture functionality."""
    
    def test_mocker_patch(self, mocker):
        """Test basic mocker.patch functionality."""
        # Mock os.path.exists
        mock_exists = mocker.patch('os.path.exists')
        mock_exists.return_value = True
        
        assert os.path.exists('/fake/path') is True
        mock_exists.assert_called_once_with('/fake/path')
    
    def test_mocker_patch_object(self, mocker):
        """Test mocker.patch.object functionality."""
        obj = Mock()
        obj.method = lambda: "original"
        
        mocker.patch.object(obj, 'method', return_value="mocked")
        assert obj.method() == "mocked"
    
    def test_mocker_patch_multiple(self, mocker):
        """Test patching multiple targets."""
        mocker.patch('os.path.exists', return_value=True)
        mocker.patch('os.path.isfile', return_value=False)
        mocker.patch('os.path.isdir', return_value=True)
        
        assert os.path.exists('/path') is True
        assert os.path.isfile('/path') is False
        assert os.path.isdir('/path') is True
    
    def test_mocker_spy(self, mocker):
        """Test mocker.spy functionality."""
        def original_func(x):
            return x * 2
        
        spy = mocker.spy(sys.modules[__name__], 'original_func')
        result = original_func(5)
        
        assert result == 10
        spy.assert_called_once_with(5)
    
    def test_mocker_stub(self, mocker):
        """Test mocker.stub functionality."""
        stub = mocker.stub(name='my_stub')
        stub.some_method.return_value = 42
        
        assert stub.some_method() == 42
        stub.some_method.assert_called_once()
    
    def test_mocker_mock(self, mocker):
        """Test mocker.Mock creation."""
        mock = mocker.Mock(return_value=123)
        assert mock() == 123
        mock.assert_called_once()
    
    def test_mocker_magic_mock(self, mocker):
        """Test mocker.MagicMock creation."""
        magic = mocker.MagicMock()
        magic.__str__.return_value = "mocked string"
        assert str(magic) == "mocked string"
    
    def test_mocker_reset_mock(self, mocker):
        """Test resetting mocks."""
        mock = mocker.Mock()
        mock(1, 2, 3)
        mock.some_attr = "value"
        
        assert mock.called
        mocker.resetall()
        assert not mock.called


class TestMockerAdvanced:
    """Test advanced mocker functionality."""
    
    def test_mocker_autospec(self, mocker):
        """Test autospec functionality."""
        import json
        mock_json = mocker.patch('json.dumps', autospec=True)
        mock_json.return_value = '{"mocked": true}'
        
        result = json.dumps({"key": "value"})
        assert result == '{"mocked": true}'
        mock_json.assert_called_once()
    
    def test_mocker_property_mock(self, mocker):
        """Test PropertyMock functionality."""
        class MyClass:
            @property
            def my_property(self):
                return "original"
        
        obj = MyClass()
        prop_mock = mocker.PropertyMock(return_value="mocked")
        mocker.patch.object(type(obj), 'my_property', prop_mock)
        
        assert obj.my_property == "mocked"
        prop_mock.assert_called_once()
    
    def test_mocker_side_effect(self, mocker):
        """Test side_effect functionality."""
        mock = mocker.Mock(side_effect=[1, 2, 3])
        
        assert mock() == 1
        assert mock() == 2
        assert mock() == 3
        
        with pytest.raises(StopIteration):
            mock()
    
    def test_mocker_exception_side_effect(self, mocker):
        """Test raising exceptions as side effects."""
        mock = mocker.Mock(side_effect=ValueError("mocked error"))
        
        with pytest.raises(ValueError, match="mocked error"):
            mock()
    
    def test_mocker_call_args(self, mocker):
        """Test inspecting call arguments."""
        mock = mocker.Mock()
        mock(1, 2, key="value")
        mock("another", call=True)
        
        assert mock.call_args_list == [
            call(1, 2, key="value"),
            call("another", call=True)
        ]
        assert mock.call_count == 2


class TestMockerContext:
    """Test mocker with context managers."""
    
    def test_mocker_context_manager(self, mocker):
        """Test mocking context managers."""
        mock_cm = mocker.MagicMock()
        mock_cm.__enter__.return_value = "entered"
        mock_cm.__exit__.return_value = None
        
        with mock_cm as value:
            assert value == "entered"
        
        mock_cm.__enter__.assert_called_once()
        mock_cm.__exit__.assert_called_once()
    
    def test_mocker_open(self, mocker):
        """Test mocking file operations."""
        mock_open = mocker.mock_open(read_data="file content")
        mocker.patch('builtins.open', mock_open)
        
        with open('/fake/file.txt', 'r') as f:
            content = f.read()
        
        assert content == "file content"
        mock_open.assert_called_once_with('/fake/file.txt', 'r')


class TestMockerAssertions:
    """Test mock assertion methods."""
    
    def test_assert_called_methods(self, mocker):
        """Test various assertion methods."""
        mock = mocker.Mock()
        
        # Not called yet
        mock.assert_not_called()
        
        # Call once
        mock(42)
        mock.assert_called()
        mock.assert_called_once()
        mock.assert_called_with(42)
        mock.assert_called_once_with(42)
        
        # Call again
        mock(99, key="value")
        mock.assert_called_with(99, key="value")
        assert mock.call_count == 2
    
    def test_assert_any_call(self, mocker):
        """Test assert_any_call functionality."""
        mock = mocker.Mock()
        mock(1)
        mock(2)
        mock(3)
        
        mock.assert_any_call(2)
        # Should not raise
    
    def test_assert_has_calls(self, mocker):
        """Test assert_has_calls functionality."""
        mock = mocker.Mock()
        mock(1)
        mock(2)
        mock(3)
        
        mock.assert_has_calls([call(1), call(2), call(3)])
        mock.assert_has_calls([call(1), call(3)], any_order=True)


class TestMockerFixtureScopes:
    """Test mocker fixture in different scopes."""
    
    def test_mocker_function_scope(self, mocker):
        """Test that mocker is function-scoped by default."""
        mock = mocker.patch('os.getcwd', return_value='/mocked')
        assert os.getcwd() == '/mocked'
    
    def test_mocker_class_scope(self, mocker):
        """Test mocker within class scope."""
        # Each test method gets fresh mocker
        assert hasattr(mocker, 'patch')
    
    def test_mocker_with_other_fixtures(self, mocker, tmp_path):
        """Test mocker works with other fixtures."""
        mocker.patch('os.path.exists', return_value=True)
        assert os.path.exists(tmp_path) is True


class TestMockerCompatibility:
    """Test pytest-mock compatibility features."""
    
    def test_mocker_stopall(self, mocker):
        """Test stopall functionality."""
        mock1 = mocker.patch('os.path.exists')
        mock2 = mocker.patch('os.path.isfile')
        
        mocker.stopall()
        # Patches should be stopped
    
    def test_mocker_pytest_mock_version(self, mocker):
        """Test that pytest-mock version attributes exist."""
        # These would be set by the compatibility layer
        assert hasattr(mocker, 'patch')
        assert hasattr(mocker, 'Mock')
        assert hasattr(mocker, 'MagicMock')
        assert hasattr(mocker, 'PropertyMock')
        assert hasattr(mocker, 'mock_open')
        assert hasattr(mocker, 'spy')
        assert hasattr(mocker, 'stub')


# Helper function for spy tests
def original_func(x):
    return x * 2