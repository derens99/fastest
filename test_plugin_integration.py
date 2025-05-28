#!/usr/bin/env python3
"""
Integration test for Phase 2 plugin system
Tests basic pytest plugin compatibility and hook execution
"""

import pytest
import os
import tempfile
from pathlib import Path

def test_basic_functionality():
    """Test that basic pytest functionality works"""
    assert 1 + 1 == 2

@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized(value):
    """Test parametrized test works"""
    assert value > 0

def test_fixture_basic(tmp_path):
    """Test that built-in fixtures work"""
    assert tmp_path.exists()
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello")
    assert test_file.read_text() == "hello"

class TestClass:
    """Test class-based tests"""
    
    def test_method(self):
        assert True
        
    @pytest.mark.skip(reason="Testing skip marker")
    def test_skipped(self):
        assert False

# Test conftest.py integration
if __name__ == "__main__":
    # Create a simple conftest.py for testing
    conftest_content = '''
import pytest

@pytest.fixture
def custom_fixture():
    return "custom_value"

def pytest_configure(config):
    config.addinivalue_line("markers", "custom: mark test as custom")
'''
    
    with tempfile.TemporaryDirectory() as tmp_dir:
        conftest_path = Path(tmp_dir) / "conftest.py"
        conftest_path.write_text(conftest_content)
        
        test_file = Path(tmp_dir) / "test_conftest.py"
        test_file.write_text('''
import pytest

@pytest.mark.custom
def test_with_conftest_fixture(custom_fixture):
    assert custom_fixture == "custom_value"
''')
        
        print(f"Created test files in {tmp_dir}")
        print("Run: python -m pytest -v to test with conftest.py integration")