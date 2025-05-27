#!/usr/bin/env python3
"""Generate test files with varying numbers of tests for scalability testing."""
import os

def generate_test_file(num_tests, filename):
    """Generate a test file with the specified number of tests."""
    content = ['"""Auto-generated test file for performance testing."""']
    content.append('import pytest')
    content.append('')
    
    # Add some fixtures for variety
    content.append('@pytest.fixture')
    content.append('def simple_fixture():')
    content.append('    return 42')
    content.append('')
    
    # Generate test functions
    for i in range(num_tests):
        # Mix of different test types
        if i % 10 == 0:
            # Parametrized test (counts as 3 tests)
            content.append(f'@pytest.mark.parametrize("x", [1, 2, 3])')
            content.append(f'def test_param_{i}(x):')
            content.append(f'    assert x > 0')
        elif i % 5 == 0:
            # Test with fixture
            content.append(f'def test_with_fixture_{i}(simple_fixture):')
            content.append(f'    assert simple_fixture == 42')
        else:
            # Simple test
            content.append(f'def test_simple_{i}():')
            content.append(f'    assert True')
        content.append('')
    
    # Add a test class every 20 tests
    if num_tests >= 20:
        content.append('class TestClass:')
        for i in range(min(5, num_tests // 20)):
            content.append(f'    def test_method_{i}(self):')
            content.append(f'        assert True')
            content.append('')
    
    with open(filename, 'w') as f:
        f.write('\n'.join(content))
    
    # Calculate actual test count (parametrized tests count as 3)
    actual_count = 0
    for i in range(num_tests):
        if i % 10 == 0:
            actual_count += 3  # Parametrized test
        else:
            actual_count += 1  # Regular test
    if num_tests >= 20:
        actual_count += min(5, num_tests // 20)  # Class methods
    
    return actual_count

def main():
    """Generate test files of different sizes."""
    test_sizes = [10, 100, 1000, 10000]
    
    # Create directory for scale tests
    os.makedirs('scale_tests', exist_ok=True)
    
    print("Generating test files...")
    for size in test_sizes:
        filename = f'scale_tests/test_{size}_tests.py'
        actual_count = generate_test_file(size, filename)
        print(f"Generated {filename} with {actual_count} actual tests")

if __name__ == "__main__":
    main()