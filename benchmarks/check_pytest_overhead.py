"""Check pytest's execution model."""
import subprocess
import time

# Create a minimal test file
test_code = """
def test_1():
    assert True

def test_2():
    assert True

def test_3():
    assert True
"""

with open("minimal_test.py", "w") as f:
    f.write(test_code)

# Time direct Python execution
print("=== Direct Python execution ===")
direct_code = """
import time
start = time.perf_counter()

def test_1():
    assert True

def test_2():
    assert True

def test_3():
    assert True

test_1()
test_2()
test_3()

print(f"Execution time: {(time.perf_counter() - start) * 1000:.2f}ms")
"""

start = time.time()
subprocess.run(["python3", "-c", direct_code])
print(f"Total time: {(time.time() - start) * 1000:.2f}ms\n")

# Time pytest
print("=== pytest execution ===")
start = time.time()
subprocess.run(["python3", "-m", "pytest", "minimal_test.py", "-q"])
print(f"Total time: {(time.time() - start) * 1000:.2f}ms\n")

# Clean up
import os
os.remove("minimal_test.py")