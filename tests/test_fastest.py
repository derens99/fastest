#!/usr/bin/env python3
"""Test script to verify the fastest extension is working."""

import fastest

# Test discovering tests
print("Testing fastest extension...")
print(f"Version: {fastest.__version__}")

# Test discovery
tests = fastest.discover_tests(".")
print(f"\nDiscovered {len(tests)} tests:")

for test in tests[:5]:  # Show first 5 tests
    print(f"  - {test.name} (line {test.line_number}) in {test.path}")
    if test.class_name:
        print(f"    Class: {test.class_name}")

# If we found tests, try running one
if tests:
    print(f"\nRunning test: {tests[0].name}")
    try:
        result = fastest.run_test(tests[0])
        print(f"  Result: {'PASSED' if result.passed else 'FAILED'}")
        print(f"  Duration: {result.duration:.3f}s")
        if result.output:
            print(f"  Output: {result.output}")
        if result.error:
            print(f"  Error: {result.error}")
    except Exception as e:
        print(f"  Error running test: {e}") 