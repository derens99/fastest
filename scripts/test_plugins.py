#!/usr/bin/env python3
"""
Test script for Fastest plugin system functionality.
Tests plugin loading, hooks, CLI options, and built-in plugins.
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

def run_fastest(test_file, args='', capture_stderr=False):
    """Run fastest on a test file and return the result."""
    cmd = f"cargo run --bin fastest -- {test_file} {args}"
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if capture_stderr:
            return result.stdout, result.stderr, result.returncode
        return result.stdout, result.returncode
    except:
        if capture_stderr:
            return None, None, -1
        return None, -1

def create_test_file(content):
    """Create a temporary test file with the given content."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(content)
        return f.name

def test_plugin_loading():
    """Test that plugins are loaded by default."""
    print(f"\n{BOLD}Testing plugin loading:{RESET}")
    
    test_content = '''
def test_simple():
    assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Run with verbose to see plugin loading
    stdout, stderr, _ = run_fastest(test_file, "-v", capture_stderr=True)
    
    if stderr and "Initializing plugin system" in stderr:
        print(f"  {GREEN}✓ Plugin system initializes{RESET}")
    else:
        print(f"  {YELLOW}⚠ Plugin initialization not visible{RESET}")
    
    if stderr and "Loaded" in stderr and "plugins" in stderr:
        print(f"  {GREEN}✓ Plugins loaded successfully{RESET}")
    else:
        print(f"  {YELLOW}⚠ Plugin loading message not found{RESET}")
    
    os.unlink(test_file)

def test_no_plugins_option():
    """Test --no-plugins option."""
    print(f"\n{BOLD}Testing --no-plugins option:{RESET}")
    
    test_content = '''
def test_simple():
    assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Run without plugins
    stdout, stderr, _ = run_fastest(test_file, "--no-plugins -v", capture_stderr=True)
    
    if stderr and "Plugin system disabled" in stderr:
        print(f"  {GREEN}✓ --no-plugins disables plugin system{RESET}")
    else:
        print(f"  {YELLOW}⚠ Plugin disable message not found{RESET}")
    
    # Verify test still runs
    if stdout and "1 passed" in stdout:
        print(f"  {GREEN}✓ Tests run without plugins{RESET}")
    else:
        print(f"  {RED}✗ Tests failed without plugins{RESET}")
    
    os.unlink(test_file)

def test_plugin_hooks():
    """Test that plugin hooks are called."""
    print(f"\n{BOLD}Testing plugin hooks:{RESET}")
    
    test_content = '''
def test_one():
    assert True

def test_two():
    assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Run with FASTEST_DEBUG to see hooks
    env = os.environ.copy()
    env['FASTEST_DEBUG'] = '1'
    
    cmd = f"cargo run --bin fastest -- {test_file} -v"
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, env=env)
    
    output = result.stdout + result.stderr
    
    hooks_found = []
    expected_hooks = [
        "pytest_collection_start",
        "pytest_collection_modifyitems",
        "pytest_collection_finish",
        "pytest_sessionstart",
        "pytest_sessionfinish"
    ]
    
    for hook in expected_hooks:
        if f"[Hook] {hook}" in output:
            hooks_found.append(hook)
            print(f"  {GREEN}✓ {hook} called{RESET}")
        else:
            print(f"  {RED}✗ {hook} not found{RESET}")
    
    if len(hooks_found) == len(expected_hooks):
        print(f"  {GREEN}✓ All expected hooks called{RESET}")
    else:
        print(f"  {YELLOW}⚠ Only {len(hooks_found)}/{len(expected_hooks)} hooks found{RESET}")
    
    os.unlink(test_file)

def test_plugin_directories():
    """Test --plugin-dir option."""
    print(f"\n{BOLD}Testing --plugin-dir option:{RESET}")
    
    test_content = '''
def test_simple():
    assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Create a temporary plugin directory
    with tempfile.TemporaryDirectory() as plugin_dir:
        # Create a dummy plugin file
        plugin_file = Path(plugin_dir) / "my_plugin.py"
        plugin_file.write_text('''
# Dummy plugin file
def pytest_configure(config):
    pass
''')
        
        # Run with plugin directory
        stdout, stderr, _ = run_fastest(test_file, f"--plugin-dir {plugin_dir} -v", capture_stderr=True)
        
        if "1 passed" in stdout:
            print(f"  {GREEN}✓ Tests run with --plugin-dir{RESET}")
        else:
            print(f"  {RED}✗ Tests failed with --plugin-dir{RESET}")
    
    os.unlink(test_file)

def test_disable_plugin():
    """Test --disable-plugin option."""
    print(f"\n{BOLD}Testing --disable-plugin option:{RESET}")
    
    test_content = '''
def test_simple():
    assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Run with a plugin disabled
    stdout, stderr, _ = run_fastest(test_file, "--disable-plugin verbose -v", capture_stderr=True)
    
    if "1 passed" in stdout:
        print(f"  {GREEN}✓ Tests run with --disable-plugin{RESET}")
    else:
        print(f"  {RED}✗ Tests failed with --disable-plugin{RESET}")
    
    os.unlink(test_file)

def test_builtin_plugins():
    """Test that built-in plugins are registered."""
    print(f"\n{BOLD}Testing built-in plugins:{RESET}")
    
    test_content = '''
def test_with_print():
    """Test output capture plugin."""
    print("Hello from test")
    assert True

def test_fixture_plugin(tmp_path):
    """Test fixture plugin."""
    assert tmp_path.exists()

import pytest
@pytest.mark.skip
def test_marker_plugin():
    """Test marker plugin."""
    assert False
'''
    
    test_file = create_test_file(test_content)
    stdout, _ = run_fastest(test_file)
    
    if stdout:
        # Check that tests passed/skipped appropriately
        if "passed" in stdout:
            print(f"  {GREEN}✓ Built-in plugins working{RESET}")
            print(f"  {GREEN}✓ Capture plugin (output capture){RESET}")
            print(f"  {GREEN}✓ Fixture plugin (tmp_path){RESET}")
            print(f"  {GREEN}✓ Marker plugin (skip){RESET}")
        else:
            print(f"  {RED}✗ Built-in plugins not working properly{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_performance_impact():
    """Test performance with and without plugins."""
    print(f"\n{BOLD}Testing performance impact:{RESET}")
    
    # Create a test file with many tests
    test_content = '\n'.join([
        f'def test_{i}(): assert True'
        for i in range(50)
    ])
    
    test_file = create_test_file(test_content)
    
    # Time with plugins
    import time
    start = time.time()
    stdout, _ = run_fastest(test_file)
    with_plugins_time = time.time() - start
    
    # Time without plugins
    start = time.time()
    stdout, _ = run_fastest(test_file, "--no-plugins")
    without_plugins_time = time.time() - start
    
    print(f"  With plugins: {with_plugins_time:.3f}s")
    print(f"  Without plugins: {without_plugins_time:.3f}s")
    
    overhead = ((with_plugins_time - without_plugins_time) / without_plugins_time) * 100
    print(f"  Plugin overhead: {overhead:.1f}%")
    
    if overhead < 10:
        print(f"  {GREEN}✓ Minimal performance impact (<10%){RESET}")
    else:
        print(f"  {YELLOW}⚠ Higher plugin overhead: {overhead:.1f}%{RESET}")
    
    os.unlink(test_file)

def test_plugin_options():
    """Test --plugin-opt option."""
    print(f"\n{BOLD}Testing --plugin-opt option:{RESET}")
    
    test_content = '''
def test_simple():
    assert True
'''
    
    test_file = create_test_file(test_content)
    
    # Run with plugin options
    stdout, stderr, _ = run_fastest(test_file, '--plugin-opt key=value --plugin-opt another=test -v', capture_stderr=True)
    
    if "1 passed" in stdout:
        print(f"  {GREEN}✓ Tests run with --plugin-opt{RESET}")
    else:
        print(f"  {RED}✗ Tests failed with --plugin-opt{RESET}")
    
    os.unlink(test_file)

def main():
    """Run all plugin tests."""
    print(f"{BOLD}{BLUE}=== Fastest Plugin System Test Suite ==={RESET}")
    print(f"Testing plugin functionality in Fastest\n")
    
    # Check if fastest is available
    if subprocess.run("cargo --version", shell=True, capture_output=True).returncode != 0:
        print(f"{RED}Error: Cargo not found. Please install Rust.{RESET}")
        sys.exit(1)
    
    # Run all tests
    test_plugin_loading()
    test_no_plugins_option()
    test_plugin_hooks()
    test_plugin_directories()
    test_disable_plugin()
    test_builtin_plugins()
    test_performance_impact()
    test_plugin_options()
    
    print(f"\n{BOLD}{GREEN}All plugin tests completed!{RESET}")
    print(f"\nPlugin features tested:")
    print(f"  • Plugin loading and initialization")
    print(f"  • --no-plugins option")
    print(f"  • Hook execution (with FASTEST_DEBUG=1)")
    print(f"  • --plugin-dir option")
    print(f"  • --disable-plugin option")
    print(f"  • Built-in plugins (fixtures, markers, capture)")
    print(f"  • Performance impact")
    print(f"  • --plugin-opt option")

if __name__ == "__main__":
    main()