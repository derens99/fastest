"""
fastest - A fast Python testing framework powered by Rust
"""

from fastest._fastest import discover_tests, run_tests, __version__

__all__ = ["discover_tests", "run_tests", "__version__", "main"]

def main():
    """Main entry point for the fastest CLI."""
    import argparse
    import sys
    import time
    
    start = time.time()
    
    parser = argparse.ArgumentParser(description="fastest - Fast Python testing")
    parser.add_argument("path", nargs="?", default=".", help="Path to discover tests from")
    parser.add_argument("-v", "--verbose", action="store_true", help="Verbose output")
    
    args = parser.parse_args()
    
    path = args.path
    
    print(f"\n🦀 fastest - discovering tests in: {path}")
    tests = discover_tests(path)
    print(f"Found {len(tests)} tests in {time.time() - start:.2f}s")
    
    if args.verbose:
        for test in tests[:10]:  # Show first 10
            print(f"  - {test['name']} ({test['path']}:{test['line_number']})")
        if len(tests) > 10:
            print(f"  ... and {len(tests) - 10} more")
    
    # Run tests
    print("\n🚀 Running tests...")
    results = run_tests(tests)
    
    # Report results
    passed = sum(1 for r in results if r["passed"])
    failed = len(results) - passed
    
    print(f"\n{'='*60}")
    if failed == 0:
        print(f"✅ All {passed} tests passed in {time.time() - start:.2f}s")
    else:
        print(f"❌ {failed} tests failed, {passed} passed in {time.time() - start:.2f}s")
        print("\nFailures:")
        for r in results:
            if not r["passed"]:
                print(f"  ❌ {r['test_id']}")
                if r.get("error"):
                    print(f"     {r['error']}")
    
    sys.exit(0 if failed == 0 else 1)

if __name__ == "__main__":
    main()