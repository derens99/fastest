"""Analyze where the time is being spent in test execution."""
import subprocess
import time
import tempfile
from pathlib import Path

# Very simple test that should execute instantly
MINIMAL_TEST = """
def test_minimal():
    pass
"""

# Create different sized test suites
def create_tests(n):
    return "\n".join([f"def test_{i}(): pass" for i in range(n)])

with tempfile.TemporaryDirectory() as tmpdir:
    tmpdir = Path(tmpdir)
    
    print("Analyzing startup overhead...\n")
    
    for test_count in [1, 5, 10, 20, 50]:
        test_file = tmpdir / f"test_{test_count}.py"
        test_file.write_text(create_tests(test_count))
        
        # Time pytest with minimal output
        start = time.time()
        subprocess.run(["pytest", str(test_file), "-q", "--tb=no"], 
                      capture_output=True, check=True)
        pytest_time = time.time() - start
        
        # Time fastest
        start = time.time()
        result = subprocess.run(["./target/release/fastest", str(test_file)], 
                               capture_output=True, text=True)
        fastest_time = time.time() - start
        
        # Parse actual test execution time from fastest output
        execution_time = None
        if "passed in" in result.stdout:
            try:
                time_str = result.stdout.split("passed in ")[1].split("s")[0]
                execution_time = float(time_str)
            except:
                pass
        
        print(f"{test_count} tests:")
        print(f"  pytest total:     {pytest_time:.3f}s")
        print(f"  fastest total:    {fastest_time:.3f}s")
        if execution_time:
            print(f"  fastest exec:     {execution_time:.3f}s")
            print(f"  fastest overhead: {fastest_time - execution_time:.3f}s")
        print(f"  speedup:          {pytest_time/fastest_time:.1f}x")
        print()

# Now test with actual work
print("\nTesting with actual work in tests...\n")

WORK_TEST = """
def test_work_{i}():
    # Do some actual work
    result = sum(range(1000))
    assert result == 499500
"""

with tempfile.TemporaryDirectory() as tmpdir:
    tmpdir = Path(tmpdir)
    
    for test_count in [10, 50, 100]:
        test_file = tmpdir / f"test_work_{test_count}.py"
        content = "\n".join([WORK_TEST.format(i=i) for i in range(test_count)])
        test_file.write_text(content)
        
        # Time pytest
        start = time.time()
        subprocess.run(["pytest", str(test_file), "-q", "--tb=no"], 
                      capture_output=True, check=True)
        pytest_time = time.time() - start
        
        # Time fastest  
        start = time.time()
        result = subprocess.run(["./target/release/fastest", str(test_file)], 
                               capture_output=True, text=True)
        fastest_time = time.time() - start
        
        print(f"{test_count} work tests:")
        print(f"  pytest:   {pytest_time:.3f}s")
        print(f"  fastest:  {fastest_time:.3f}s")
        print(f"  speedup:  {pytest_time/fastest_time:.1f}x")
        print()