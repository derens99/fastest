"""
Fastest - A blazing fast Python test runner built with Rust
"""

__version__ = "0.1.0"

# Import from Rust extension
try:
    from ._fastest import (
        discover_tests,
        discover_tests_cached,
        discover_tests_ast,
        run_test,
        run_tests_batch,
        run_tests_parallel,
        TestItem,
        TestResult,
    )
except ImportError:
    raise ImportError(
        "Failed to import Rust extension. "
        "Please build the project with: maturin develop"
    )

# Import marker system
from .markers import mark

# Make mark available at package level
__all__ = [
    "discover_tests",
    "discover_tests_cached", 
    "discover_tests_ast",
    "run_test",
    "run_tests_batch",
    "run_tests_parallel",
    "TestItem",
    "TestResult",
    "mark",
]

def main():
    """Entry point for command line usage."""
    import sys
    import subprocess
    
    # Forward to the actual CLI binary
    try:
        result = subprocess.run(["fastest"] + sys.argv[1:], sys.exit(result.returncode))
    except FileNotFoundError:
        print("Error: fastest CLI not found. Please install it first.")
        sys.exit(1) 