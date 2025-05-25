"""
rusttest - A fast Python testing framework powered by Rust
"""

from rusttest._rusttest import discover_tests, run_tests, __version__

__all__ = ["discover_tests", "run_tests", "__version__", "main"]

def main():
    """CLI entry point"""
    import sys
    import argparse
    from pathlib import Path
    
    parser = argparse.ArgumentParser(description="rusttest - Fast Python testing")
    parser.add_argument("path", nargs="?", default=".", help="Path to discover tests")
    parser.add_argument("--collect-only", action="store_true", help="Only collect tests")
    parser.add_argument("-v", "--verbose", action="store_true", help="Verbose output")
    
    args = parser.parse_args()
    
    path = Path(args.path).resolve()
    print(f"\n🦀 rusttest - discovering tests in: {path}")
    print("=" * 70)
    
    try:
        if args.collect_only:
            tests = discover_tests(str(path))
            print(f"\nFound {len(tests)} tests:\n")
            for test in tests:
                print(f"  {test.id}")
                if args.verbose:
                    print(f"    File: {test.path}:{test.line_number}")
                    print(f"    Function: {test.function_name}")
                    if test.is_async:
                        print(f"    Async: Yes")
            return 0
        else:
            # Run tests
            results = run_tests(str(path))
            
            passed = sum(1 for r in results if r.passed)
            failed = sum(1 for r in results if not r.passed)
            total_duration = sum(r.duration for r in results)
            
            print(f"\nRunning {len(results)} tests...\n")
            
            for result in results:
                status = "✓ PASSED" if result.passed else "✗ FAILED"
                print(f"{status} {result.test_id} ({result.duration:.3f}s)")
                
                if not result.passed and result.error:
                    print(f"\n{result.error}\n")
                elif args.verbose and result.output:
                    print(f"Output:\n{result.output}\n")
            
            print("=" * 70)
            print(f"\nResults: {passed} passed, {failed} failed in {total_duration:.2f}s")
            
            return 0 if failed == 0 else 1
            
    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        return 2

if __name__ == "__main__":
    exit(main())