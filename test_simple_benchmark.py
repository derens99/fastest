"""Simple benchmark to test Fastest vs pytest performance."""
import subprocess
import time
import tempfile
from pathlib import Path

# Create a simple test file
TEST_CONTENT = """
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

with tempfile.TemporaryDirectory() as tmpdir:
    tmpdir = Path(tmpdir)
    test_file = tmpdir / "test_simple.py"
    test_file.write_text(TEST_CONTENT)
    
    print("Running pytest...")
    start = time.time()
    result = subprocess.run(["pytest", str(test_file), "-q"], capture_output=True)
    pytest_time = time.time() - start
    print(f"pytest time: {pytest_time:.3f}s")
    
    print("\nRunning fastest...")
    start = time.time()
    result = subprocess.run(["./target/release/fastest", str(test_file)], capture_output=True)
    fastest_time = time.time() - start
    print(f"fastest time: {fastest_time:.3f}s")
    print(f"\nSpeedup: {pytest_time/fastest_time:.1f}x")
    
    # Show output for debugging
    print("\nFastest output:")
    print(result.stderr.decode())
    print(result.stdout.decode())