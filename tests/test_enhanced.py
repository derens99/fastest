#!/usr/bin/env python3
"""Enhanced test script to demonstrate new fastest features."""

import fastest
import time

def print_test_result(result):
    """Pretty print a test result."""
    status = "✅ PASSED" if result.passed else "❌ FAILED"
    print(f"\n{status} {result.test_id}")
    print(f"  Duration: {result.duration:.3f}s")
    
    if result.stdout:
        print(f"  Stdout: {result.stdout.strip()}")
    
    if result.stderr:
        print(f"  Stderr: {result.stderr.strip()}")
    
    if result.error:
        print(f"  Error: {result.error}")

def main():
    print("🚀 Testing enhanced fastest extension...")
    print(f"Version: {fastest.__version__}")
    
    # Discover tests
    tests = fastest.discover_tests(".")
    print(f"\n📊 Discovered {len(tests)} tests")
    
    # Show test details
    print("\n📋 Test inventory:")
    for i, test in enumerate(tests[:10]):  # Show first 10
        class_info = f" (in class {test.class_name})" if test.class_name else ""
        async_info = " [async]" if test.is_async else ""
        print(f"  {i+1}. {test.function_name}{async_info} at line {test.line_number}{class_info}")
        print(f"     Path: {test.path}")
    
    if len(tests) > 10:
        print(f"  ... and {len(tests) - 10} more tests")
    
    # Run a few tests to demonstrate features
    print("\n🧪 Running sample tests...")
    
    for test in tests[:3]:
        try:
            start = time.time()
            result = fastest.run_test(test)
            elapsed = time.time() - start
            
            print_test_result(result)
            print(f"  Total time (including overhead): {elapsed:.3f}s")
            
        except Exception as e:
            print(f"\n❌ Error running test {test.name}: {e}")
    
    # Create a test that will fail to demonstrate error handling
    print("\n🔍 Testing error handling with a non-existent test...")
    
    # Create a fake test item
    class FakeTest:
        def __init__(self):
            self.id = "fake::test"
            self.path = "/nonexistent/test.py"
            self.name = "test_fake"
            self.function_name = "test_fake"
            self.line_number = 1
            self.is_async = False
            self.class_name = None
    
    try:
        fake_test = FakeTest()
        result = fastest.run_test(fake_test)
        print_test_result(result)
    except Exception as e:
        print(f"❌ Expected error: {e}")
    
    print("\n✅ Test demonstration complete!")

if __name__ == "__main__":
    main() 