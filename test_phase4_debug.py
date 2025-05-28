#!/usr/bin/env python3
"""
Phase 4: Developer Experience Integration Test
Tests debugging, IDE integration, enhanced error reporting, and timeout handling
"""

import pytest
import time
import asyncio

def test_basic_pass():
    """Test that should pass for debugging integration"""
    assert 1 + 1 == 2

def test_assertion_failure():
    """Test that should fail with detailed assertion error"""
    expected = "hello world"
    actual = "hello universe"
    assert expected == actual, f"Expected '{expected}' but got '{actual}'"

def test_attribute_error():
    """Test that should fail with attribute error"""
    obj = "string"
    result = obj.non_existent_attribute
    assert result is not None

def test_type_error():
    """Test that should fail with type error"""
    result = "string" + 42
    assert result is not None

@pytest.mark.timeout(5)
def test_with_timeout():
    """Test with custom timeout marker"""
    time.sleep(0.1)  # Should complete within timeout
    assert True

def test_slow_operation():
    """Test that might timeout (for timeout testing)"""
    # This would timeout if timeout is set low
    time.sleep(0.1)
    assert True

@pytest.mark.asyncio
async def test_async_operation():
    """Async test for async support testing"""
    await asyncio.sleep(0.1)
    assert True

@pytest.mark.parametrize("value,expected", [
    (1, 2),
    (2, 4),
    (3, 6),
])
def test_parametrized_with_failure(value, expected):
    """Parametrized test with some failures for enhanced reporting"""
    result = value * 2
    assert result == expected

class TestDebugClass:
    """Test class for debugging class-based tests"""
    
    def test_method_pass(self):
        assert True
        
    def test_method_fail(self):
        """Method that fails for debugging"""
        data = {"key": "value"}
        assert data["missing_key"] == "value"

def test_fixture_error(tmp_path):
    """Test that uses fixtures and might have fixture-related errors"""
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello")
    
    # This should work
    content = test_file.read_text()
    assert content == "hello"

def test_import_error():
    """Test that would have import error"""
    # This would fail if the module doesn't exist
    # import non_existent_module
    # Use placeholder that won't actually fail compilation
    assert True

if __name__ == "__main__":
    print("Phase 4 Developer Experience Test Suite")
    print("Features tested:")
    print("✓ Debug integration (--pdb flag)")
    print("✓ Enhanced error reporting")
    print("✓ Timeout handling")
    print("✓ Async test support")
    print("✓ IDE integration capabilities")
    print("\nRun with: cargo run --bin fastest -- test_phase4_debug.py -v")
    print("Debug mode: cargo run --bin fastest -- test_phase4_debug.py --pdb")
    print("Enhanced errors: cargo run --bin fastest -- test_phase4_debug.py --enhanced-errors")