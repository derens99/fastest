"""Test execution strategy selection and performance."""
import os
import subprocess
import time
import tempfile
from pathlib import Path

# Test suites of different sizes
SMALL_SUITE = """
def test_one():
    assert 1 + 1 == 2

def test_two():
    assert 2 + 2 == 4

def test_three():
    assert 3 + 3 == 6

def test_four():
    assert 4 + 4 == 8

def test_five():
    assert 5 + 5 == 10
"""

MEDIUM_SUITE_TEMPLATE = """
def test_{n}():
    assert {n} + {n} == {n} * 2
"""

LARGE_SUITE_TEMPLATE = """
def test_{n}():
    import time
    # Simulate some work
    result = sum(range(100))
    assert result == 4950
"""

def create_test_suite(tmpdir, num_tests, template=None):
    """Create a test suite with the specified number of tests."""
    test_file = tmpdir / "test_suite.py"
    
    if num_tests <= 5:
        test_file.write_text(SMALL_SUITE)
    else:
        if template is None:
            template = MEDIUM_SUITE_TEMPLATE
        
        tests = []
        for i in range(1, num_tests + 1):
            tests.append(template.format(n=i))
        
        test_file.write_text("\n".join(tests))
    
    return test_file

def run_fastest_and_capture_strategy(test_path):
    """Run Fastest and capture which strategy was used."""
    cmd = ["./target/release/fastest", str(test_path), "-v"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    output = result.stderr + result.stdout
    
    # Detect strategy from output
    if "in-process" in output.lower() or "inprocess" in output.lower():
        return "InProcess"
    elif "warm" in output.lower() and "worker" in output.lower():
        return "WarmWorkers"
    elif "parallel" in output.lower() or "full" in output.lower():
        return "FullParallel"
    else:
        # Try to infer from test count
        if "ultra‑fast executor:" in output:
            lines = output.split('\n')
            for line in lines:
                if "tests using" in line:
                    if "in-process" in line:
                        return "InProcess"
                    elif "warm worker" in line:
                        return "WarmWorkers"
                    elif "full parallel" in line:
                        return "FullParallel"
    
    return "Unknown"

def test_small_suite_uses_inprocess():
    """Test that small suites (≤20 tests) use InProcess strategy."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        # Test with 5, 10, 15, 20 tests
        for num_tests in [5, 10, 15, 20]:
            test_file = create_test_suite(tmpdir, num_tests)
            strategy = run_fastest_and_capture_strategy(tmpdir)
            
            assert strategy == "InProcess", \
                f"Expected InProcess for {num_tests} tests, got {strategy}"
            
            print(f"✓ {num_tests} tests correctly used InProcess strategy")

def test_medium_suite_uses_warmworkers():
    """Test that medium suites (21-100 tests) use WarmWorkers strategy."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        # Test with 25, 50, 75, 100 tests
        for num_tests in [25, 50, 75, 100]:
            test_file = create_test_suite(tmpdir, num_tests)
            strategy = run_fastest_and_capture_strategy(tmpdir)
            
            assert strategy == "WarmWorkers", \
                f"Expected WarmWorkers for {num_tests} tests, got {strategy}"
            
            print(f"✓ {num_tests} tests correctly used WarmWorkers strategy")

def test_large_suite_uses_fullparallel():
    """Test that large suites (>100 tests) use FullParallel strategy."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        # Test with 101, 200, 500 tests
        for num_tests in [101, 200, 500]:
            test_file = create_test_suite(tmpdir, num_tests, LARGE_SUITE_TEMPLATE)
            strategy = run_fastest_and_capture_strategy(tmpdir)
            
            assert strategy == "FullParallel", \
                f"Expected FullParallel for {num_tests} tests, got {strategy}"
            
            print(f"✓ {num_tests} tests correctly used FullParallel strategy")

def test_strategy_boundaries():
    """Test strategy switching at exact boundaries (20/21, 100/101)."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        # Test 20 -> 21 boundary
        test_file = create_test_suite(tmpdir, 20)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "InProcess", "20 tests should use InProcess"
        
        test_file = create_test_suite(tmpdir, 21)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "WarmWorkers", "21 tests should use WarmWorkers"
        
        # Test 100 -> 101 boundary
        test_file = create_test_suite(tmpdir, 100)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "WarmWorkers", "100 tests should use WarmWorkers"
        
        test_file = create_test_suite(tmpdir, 101)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "FullParallel", "101 tests should use FullParallel"
        
        print("✓ Strategy boundaries correctly enforced")

def measure_performance_by_strategy():
    """Measure actual performance of each strategy."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        results = []
        
        # Test different sizes
        test_configs = [
            (5, "InProcess"),
            (20, "InProcess"),
            (50, "WarmWorkers"),
            (100, "WarmWorkers"),
            (200, "FullParallel"),
        ]
        
        for num_tests, expected_strategy in test_configs:
            test_file = create_test_suite(tmpdir, num_tests)
            
            # Time the execution
            start_time = time.time()
            cmd = ["./target/release/fastest", str(tmpdir)]
            result = subprocess.run(cmd, capture_output=True, text=True)
            elapsed = time.time() - start_time
            
            # Extract actual strategy used
            strategy = run_fastest_and_capture_strategy(tmpdir)
            
            results.append({
                "tests": num_tests,
                "expected_strategy": expected_strategy,
                "actual_strategy": strategy,
                "time": elapsed,
                "passed": result.returncode == 0
            })
            
            print(f"{num_tests:3d} tests | {strategy:12s} | {elapsed:.3f}s")
        
        return results

if __name__ == "__main__":
    print("Testing execution strategy selection...\n")
    
    print("1. Testing small suites (InProcess)...")
    test_small_suite_uses_inprocess()
    
    print("\n2. Testing medium suites (WarmWorkers)...")
    test_medium_suite_uses_warmworkers()
    
    print("\n3. Testing large suites (FullParallel)...")
    test_large_suite_uses_fullparallel()
    
    print("\n4. Testing strategy boundaries...")
    test_strategy_boundaries()
    
    print("\n5. Measuring performance by strategy...")
    results = measure_performance_by_strategy()
    
    print("\n✅ All strategy tests passed!")