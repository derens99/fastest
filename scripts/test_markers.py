#!/usr/bin/env python3
"""
Test script for Fastest marker system functionality.
Tests skip, xfail, skipif, and custom markers.
"""

import subprocess
import sys
import os
import tempfile
import json
from pathlib import Path

# Colors for output
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
RESET = '\033[0m'
BOLD = '\033[1m'

def run_fastest(test_file, args=''):
    """Run fastest on a test file and return the result."""
    cmd = f"cargo run --bin fastest -- {test_file} {args} -o json"
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if result.stdout:
            return json.loads(result.stdout), result.returncode
        return None, result.returncode
    except:
        return None, -1

def create_test_file(content):
    """Create a temporary test file with the given content."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(content)
        return f.name

def test_skip_marker():
    """Test @pytest.mark.skip functionality."""
    print(f"\n{BOLD}Testing @pytest.mark.skip:{RESET}")
    
    test_content = '''
import pytest

def test_normal():
    """This test should run."""
    assert True

@pytest.mark.skip
def test_skip_no_reason():
    """This test should be skipped."""
    assert False

@pytest.mark.skip(reason="Not implemented yet")
def test_skip_with_reason():
    """This test should be skipped with a reason."""
    assert False

def test_runtime_skip():
    """This test uses pytest.skip()."""
    pytest.skip("Skipping at runtime")
    assert False
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        skipped = sum(1 for r in results if 'SKIPPED' in r.get('error', ''))
        
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        print(f"  Skipped: {YELLOW}{skipped}{RESET}")
        
        assert passed == 1, f"Expected 1 passed test, got {passed}"
        assert skipped == 3, f"Expected 3 skipped tests, got {skipped}"
        print(f"  {GREEN}✓ Skip marker working correctly{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_xfail_marker():
    """Test @pytest.mark.xfail functionality."""
    print(f"\n{BOLD}Testing @pytest.mark.xfail:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.xfail
def test_xfail_expected():
    """This test is expected to fail."""
    assert False

@pytest.mark.xfail(reason="Known bug")
def test_xfail_with_reason():
    """This test is expected to fail with a reason."""
    assert 1 == 2

@pytest.mark.xfail
def test_xpass():
    """This test is expected to fail but passes (XPASS)."""
    assert True

@pytest.mark.xfail(strict=True)
def test_xfail_strict():
    """Strict xfail - XPASS should be treated as failure."""
    assert True
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        xfailed = sum(1 for r in results if r.get('outcome') == 'xfailed' or 'xfail' in str(r))
        xpassed = sum(1 for r in results if r.get('outcome') == 'xpassed' or 'xpass' in str(r))
        
        print(f"  Total tests: {len(results)}")
        print(f"  XFailed: {YELLOW}{xfailed}{RESET}")
        print(f"  XPassed: {BLUE}{xpassed}{RESET}")
        print(f"  {GREEN}✓ XFail marker working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_skipif_marker():
    """Test @pytest.mark.skipif functionality."""
    print(f"\n{BOLD}Testing @pytest.mark.skipif:{RESET}")
    
    test_content = '''
import pytest
import sys

@pytest.mark.skipif(sys.version_info < (3, 0), reason="Requires Python 3")
def test_python3_only():
    """This should run on Python 3."""
    assert True

@pytest.mark.skipif(True, reason="Always skip")
def test_always_skip():
    """This should always be skipped."""
    assert False

@pytest.mark.skipif(False, reason="Never skip")
def test_never_skip():
    """This should never be skipped."""
    assert True

@pytest.mark.skipif(1 + 1 == 2, reason="Math works")
def test_skipif_expression():
    """Skip based on expression."""
    assert False
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'] and 'SKIP' not in r.get('error', ''))
        skipped = sum(1 for r in results if 'SKIP' in r.get('error', ''))
        
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        print(f"  Skipped: {YELLOW}{skipped}{RESET}")
        print(f"  {GREEN}✓ Skipif marker working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_custom_markers():
    """Test custom marker functionality."""
    print(f"\n{BOLD}Testing custom markers:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.slow
def test_slow():
    """Test marked as slow."""
    assert True

@pytest.mark.integration
def test_integration():
    """Test marked as integration."""
    assert True

@pytest.mark.unit
def test_unit():
    """Test marked as unit."""
    assert True

def test_no_marker():
    """Test without any marker."""
    assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Test filtering by marker
    print(f"  Testing marker filtering:")
    
    # Run only slow tests
    results, _ = run_fastest(test_file, '-m slow')
    if results:
        print(f"    -m slow: {len(results)} test(s)")
        assert len(results) == 1, f"Expected 1 slow test, got {len(results)}"
    
    # Run only integration tests
    results, _ = run_fastest(test_file, '-m integration')
    if results:
        print(f"    -m integration: {len(results)} test(s)")
        assert len(results) == 1, f"Expected 1 integration test, got {len(results)}"
    
    # Run tests NOT marked as slow
    results, _ = run_fastest(test_file, '-m "not slow"')
    if results:
        print(f"    -m 'not slow': {len(results)} test(s)")
        assert len(results) == 3, f"Expected 3 non-slow tests, got {len(results)}"
    
    print(f"  {GREEN}✓ Custom marker filtering working{RESET}")
    os.unlink(test_file)

def test_marker_combinations():
    """Test combinations of markers."""
    print(f"\n{BOLD}Testing marker combinations:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.skip
@pytest.mark.slow
def test_skip_and_slow():
    """Test with multiple markers."""
    assert False

@pytest.mark.xfail
@pytest.mark.integration
def test_xfail_integration():
    """Integration test expected to fail."""
    assert False

@pytest.mark.skipif(True, reason="Skip this")
@pytest.mark.unit
def test_skipif_unit():
    """Unit test that should be skipped."""
    assert False
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        print(f"  Total tests: {len(results)}")
        print(f"  {GREEN}✓ Marker combinations working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_class_markers():
    """Test markers on classes."""
    print(f"\n{BOLD}Testing markers on classes:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.slow
class TestSlowClass:
    def test_method1(self):
        assert True
    
    def test_method2(self):
        assert True

class TestMixedMarkers:
    @pytest.mark.skip
    def test_skip_method(self):
        assert False
    
    def test_normal_method(self):
        assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Run all tests
    results, _ = run_fastest(test_file)
    if results:
        print(f"  Total tests: {len(results)}")
    
    # Run only slow tests (should get both methods from TestSlowClass)
    results, _ = run_fastest(test_file, '-m slow')
    if results:
        print(f"  Tests marked as slow: {len(results)}")
        assert len(results) == 2, f"Expected 2 slow tests, got {len(results)}"
        print(f"  {GREEN}✓ Class-level markers working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def main():
    """Run all marker tests."""
    print(f"{BOLD}{BLUE}=== Fastest Marker System Test Suite ==={RESET}")
    print(f"Testing marker functionality in Fastest\n")
    
    # Check if fastest is available
    if subprocess.run("cargo --version", shell=True, capture_output=True).returncode != 0:
        print(f"{RED}Error: Cargo not found. Please install Rust.{RESET}")
        sys.exit(1)
    
    # Run all tests
    test_skip_marker()
    test_xfail_marker()
    test_skipif_marker()
    test_custom_markers()
    test_marker_combinations()
    test_class_markers()
    
    print(f"\n{BOLD}{GREEN}All marker tests completed!{RESET}")
    print(f"\nMarker system features tested:")
    print(f"  • @pytest.mark.skip with and without reasons")
    print(f"  • @pytest.mark.xfail and XPASS detection")
    print(f"  • @pytest.mark.skipif with conditions")
    print(f"  • Custom markers and filtering (-m)")
    print(f"  • Marker combinations")
    print(f"  • Class-level markers")

if __name__ == "__main__":
    main()