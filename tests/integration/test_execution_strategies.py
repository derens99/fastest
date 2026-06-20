"""Test execution strategy selection and performance."""

import os
import subprocess
import time
import tempfile
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[2]

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


def fastest_binary():
    """Return the Fastest binary to exercise from pytest."""
    configured = os.environ.get("FASTEST_BINARY")
    if configured:
        return Path(configured)

    candidate = REPO_ROOT / "target" / "debug" / "fastest"
    if not candidate.exists():
        pytest.skip(
            "target/debug/fastest is missing; run `cargo build -p fastest-cli` "
            "or set FASTEST_BINARY"
        )
    return candidate


def detect_strategy(output):
    """Detect the strategy label printed by the current Rust runner."""
    lowered = output.lower()

    if "ultra in-process" in lowered or "ultra-inprocess" in lowered:
        return "UltraInProcess"
    if "burst execution" in lowered or "hybridburst" in lowered:
        return "HybridBurst"
    if "workstealing" in lowered or "work-stealing" in lowered:
        return "WorkStealing"

    return "Unknown"


def run_fastest_and_capture_strategy(test_path):
    """Run Fastest and capture which strategy was used."""
    cmd = [str(fastest_binary()), str(test_path), "-v"]
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO_ROOT)

    output = result.stderr + result.stdout

    assert result.returncode == 0, (
        "Fastest failed while running strategy test suite\n"
        f"command: {' '.join(cmd)}\n"
        f"stdout:\n{result.stdout}\n"
        f"stderr:\n{result.stderr}"
    )

    return detect_strategy(output)


def test_small_suite_uses_ultra_inprocess():
    """Small suites currently use the compatibility-first ultra in-process path."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Test with 5, 10, 15, 20 tests
        for num_tests in [5, 10, 15, 20]:
            create_test_suite(tmpdir, num_tests)
            strategy = run_fastest_and_capture_strategy(tmpdir)

            assert (
                strategy == "UltraInProcess"
            ), f"Expected UltraInProcess for {num_tests} tests, got {strategy}"


def test_medium_suite_uses_ultra_inprocess():
    """Medium suites currently stay on the compatibility-first ultra in-process path."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Test with 25, 50, 75, 100 tests
        for num_tests in [25, 50, 75, 100]:
            create_test_suite(tmpdir, num_tests)
            strategy = run_fastest_and_capture_strategy(tmpdir)

            assert (
                strategy == "UltraInProcess"
            ), f"Expected UltraInProcess for {num_tests} tests, got {strategy}"


def test_large_suite_uses_ultra_inprocess():
    """Large suites currently stay on the compatibility-first ultra in-process path."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Test with 101, 200, 500 tests
        for num_tests in [101, 200, 500]:
            create_test_suite(tmpdir, num_tests, LARGE_SUITE_TEMPLATE)
            strategy = run_fastest_and_capture_strategy(tmpdir)

            assert (
                strategy == "UltraInProcess"
            ), f"Expected UltraInProcess for {num_tests} tests, got {strategy}"


def test_strategy_boundaries_are_stable_in_compatibility_mode():
    """Compatibility mode should not switch strategies at old thresholds."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        # Test 20 -> 21 boundary
        create_test_suite(tmpdir, 20)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "UltraInProcess", "20 tests should use UltraInProcess"

        create_test_suite(tmpdir, 21)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "UltraInProcess", "21 tests should use UltraInProcess"

        # Test 100 -> 101 boundary
        create_test_suite(tmpdir, 100)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "UltraInProcess", "100 tests should use UltraInProcess"

        create_test_suite(tmpdir, 101)
        strategy = run_fastest_and_capture_strategy(tmpdir)
        assert strategy == "UltraInProcess", "101 tests should use UltraInProcess"


def measure_performance_by_strategy():
    """Measure actual performance of each strategy."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)

        results = []

        # Test different sizes
        test_configs = [
            (5, "UltraInProcess"),
            (20, "UltraInProcess"),
            (50, "UltraInProcess"),
            (100, "UltraInProcess"),
            (200, "UltraInProcess"),
        ]

        for num_tests, expected_strategy in test_configs:
            create_test_suite(tmpdir, num_tests)

            # Time the execution
            start_time = time.time()
            cmd = [str(fastest_binary()), str(tmpdir)]
            result = subprocess.run(cmd, capture_output=True, text=True, cwd=REPO_ROOT)
            elapsed = time.time() - start_time

            # Extract actual strategy used
            strategy = run_fastest_and_capture_strategy(tmpdir)

            results.append(
                {
                    "tests": num_tests,
                    "expected_strategy": expected_strategy,
                    "actual_strategy": strategy,
                    "time": elapsed,
                    "passed": result.returncode == 0,
                }
            )

            print(f"{num_tests:3d} tests | {strategy:12s} | {elapsed:.3f}s")

        return results


if __name__ == "__main__":
    print("Testing execution strategy selection...\n")

    print("1. Testing small suites...")
    test_small_suite_uses_ultra_inprocess()

    print("\n2. Testing medium suites...")
    test_medium_suite_uses_ultra_inprocess()

    print("\n3. Testing large suites...")
    test_large_suite_uses_ultra_inprocess()

    print("\n4. Testing strategy boundary stability...")
    test_strategy_boundaries_are_stable_in_compatibility_mode()

    print("\n5. Measuring performance by strategy...")
    results = measure_performance_by_strategy()

    print("\n✅ All strategy tests passed!")
