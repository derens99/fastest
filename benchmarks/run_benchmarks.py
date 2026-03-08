#!/usr/bin/env python3
"""
Benchmark: Fastest vs Pytest -- Runtime Comparison

Generates synthetic test suites of various sizes, times both test runners
across multiple trials, and saves results as JSON for graph generation.

Usage:
    python benchmarks/run_benchmarks.py

Output:
    benchmarks/results/results.json          -- raw timing data
"""

import json
import os
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

PYTHON = sys.executable
REPO_ROOT = Path(__file__).resolve().parent.parent
FASTEST_BIN = REPO_ROOT / "target" / "release" / "fastest.exe"
if not FASTEST_BIN.exists():
    FASTEST_BIN = REPO_ROOT / "target" / "release" / "fastest"
RESULTS_DIR = REPO_ROOT / "benchmarks" / "results"

# Use a directory inside the repo for temp files (Windows path compat)
BENCH_TMP = REPO_ROOT / "benchmarks" / "_tmp_bench"

# Suite sizes to benchmark
SUITE_SIZES = [10, 50, 100, 250, 500, 1000]

# Number of timing trials per (runner x size)
TRIALS = 5

# Warm-up runs (discarded)
WARMUP = 1


# ---------------------------------------------------------------------------
# Test-suite generation
# ---------------------------------------------------------------------------

def generate_test_suite(directory: Path, n_tests: int) -> None:
    """Create a directory of Python test files totalling *n_tests* tests.

    Tests are spread across multiple files (<=50 tests per file) to simulate
    a realistic project layout.  Each test does a small amount of computation
    so parallelism benefits show up.
    """
    tests_per_file = 50
    n_files = max(1, (n_tests + tests_per_file - 1) // tests_per_file)

    for file_idx in range(n_files):
        start = file_idx * tests_per_file
        end = min(start + tests_per_file, n_tests)
        if start >= n_tests:
            break

        lines = ["import time\n"]
        for t in range(start, end):
            lines.append(
                f"def test_generated_{t}():\n"
                f"    total = sum(range(500))\n"
                f"    assert total == 124750\n"
            )

        filepath = directory / f"test_gen_{file_idx:04d}.py"
        filepath.write_text("\n".join(lines))


def generate_discovery_suite(directory: Path, n_files: int, tests_per_file: int = 20) -> int:
    """Create test files for discovery-only benchmarking. Returns total test count."""
    total = 0
    for file_idx in range(n_files):
        lines = []
        for t in range(tests_per_file):
            lines.append(f"def test_disc_{file_idx}_{t}():\n    pass\n")
            total += 1
        filepath = directory / f"test_disc_{file_idx:04d}.py"
        filepath.write_text("\n".join(lines))
    return total


# ---------------------------------------------------------------------------
# Timing helpers
# ---------------------------------------------------------------------------

def time_command(cmd: list, cwd: Path, trials: int = TRIALS, warmup: int = WARMUP) -> dict:
    """Run *cmd* (warmup + trials) times and return timing statistics."""
    times = []
    for i in range(warmup + trials):
        start = time.perf_counter()
        result = subprocess.run(
            cmd,
            cwd=str(cwd),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=300,
        )
        elapsed = time.perf_counter() - start
        if i >= warmup:
            times.append(elapsed)

    return {
        "mean": statistics.mean(times),
        "median": statistics.median(times),
        "min": min(times),
        "max": max(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0.0,
        "raw": times,
    }


# ---------------------------------------------------------------------------
# Benchmark: Execution (end-to-end)
# ---------------------------------------------------------------------------

def run_execution_benchmarks() -> list:
    """Run Fastest and pytest on every suite size and return results."""
    results = []

    for size in SUITE_SIZES:
        print(f"\n{'='*60}")
        print(f"  Execution Benchmark: {size} tests  ({TRIALS} trials, {WARMUP} warmup)")
        print(f"{'='*60}")

        suite_dir = BENCH_TMP / f"exec_{size}"
        if suite_dir.exists():
            shutil.rmtree(suite_dir)
        suite_dir.mkdir(parents=True)

        try:
            generate_test_suite(suite_dir, size)

            # Verify fastest sees the tests
            verify = subprocess.run(
                [str(FASTEST_BIN), "discover", str(suite_dir)],
                cwd=str(REPO_ROOT),
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
            discovered = verify.stdout.decode().strip().split("\n")[-1]
            print(f"  Fastest discovered: {discovered}")

            # --- Fastest ---
            print(f"  [fastest] Running ...", end="", flush=True)
            fastest_stats = time_command(
                [str(FASTEST_BIN), str(suite_dir), "--no-progress", "-q"],
                cwd=REPO_ROOT,
            )
            print(f"  {fastest_stats['mean']:.3f}s (mean)")

            # --- Pytest ---
            print(f"  [pytest]  Running ...", end="", flush=True)
            pytest_stats = time_command(
                [PYTHON, "-m", "pytest", str(suite_dir), "-q", "--tb=no", "-p", "no:cacheprovider"],
                cwd=REPO_ROOT,
            )
            print(f"  {pytest_stats['mean']:.3f}s (mean)")

            speedup = pytest_stats["mean"] / fastest_stats["mean"] if fastest_stats["mean"] > 0 else 0

            row = {
                "size": size,
                "fastest": fastest_stats,
                "pytest": pytest_stats,
                "speedup": round(speedup, 2),
            }
            results.append(row)
            print(f"  >> Speedup: {speedup:.1f}x")

        finally:
            shutil.rmtree(suite_dir, ignore_errors=True)

    return results


# ---------------------------------------------------------------------------
# Benchmark: Discovery only
# ---------------------------------------------------------------------------

def run_discovery_benchmarks() -> list:
    """Benchmark test discovery/collection speed across file counts."""
    file_counts = [10, 50, 100, 250, 500]
    results = []

    for n_files in file_counts:
        print(f"\n{'='*60}")
        print(f"  Discovery Benchmark: {n_files} files  ({TRIALS} trials, {WARMUP} warmup)")
        print(f"{'='*60}")

        suite_dir = BENCH_TMP / f"disc_{n_files}"
        if suite_dir.exists():
            shutil.rmtree(suite_dir)
        suite_dir.mkdir(parents=True)

        try:
            total_tests = generate_discovery_suite(suite_dir, n_files)
            print(f"  {total_tests} tests across {n_files} files")

            # --- Fastest discover ---
            print(f"  [fastest] Discovering ...", end="", flush=True)
            fastest_stats = time_command(
                [str(FASTEST_BIN), "discover", str(suite_dir)],
                cwd=REPO_ROOT,
            )
            print(f"  {fastest_stats['mean']:.3f}s (mean)")

            # --- Pytest collect-only ---
            print(f"  [pytest]  Collecting  ...", end="", flush=True)
            pytest_stats = time_command(
                [PYTHON, "-m", "pytest", str(suite_dir), "--collect-only", "-q",
                 "-p", "no:cacheprovider"],
                cwd=REPO_ROOT,
            )
            print(f"  {pytest_stats['mean']:.3f}s (mean)")

            speedup = pytest_stats["mean"] / fastest_stats["mean"] if fastest_stats["mean"] > 0 else 0

            row = {
                "n_files": n_files,
                "total_tests": total_tests,
                "fastest": fastest_stats,
                "pytest": pytest_stats,
                "speedup": round(speedup, 2),
            }
            results.append(row)
            print(f"  >> Speedup: {speedup:.1f}x")

        finally:
            shutil.rmtree(suite_dir, ignore_errors=True)

    return results


# ---------------------------------------------------------------------------
# Benchmark: Realistic execution (tests with ~10ms work each)
# ---------------------------------------------------------------------------

def generate_realistic_suite(directory: Path, n_tests: int) -> None:
    """Generate tests that each do ~10ms of CPU work (realistic test time)."""
    tests_per_file = 50
    n_files = max(1, (n_tests + tests_per_file - 1) // tests_per_file)

    for file_idx in range(n_files):
        start = file_idx * tests_per_file
        end = min(start + tests_per_file, n_tests)
        if start >= n_tests:
            break

        lines = ["import time\n"]
        for t in range(start, end):
            lines.append(
                f"def test_realistic_{t}():\n"
                f"    time.sleep(0.01)\n"
                f"    total = sum(range(10000))\n"
                f"    assert total == 49995000\n"
            )

        filepath = directory / f"test_real_{file_idx:04d}.py"
        filepath.write_text("\n".join(lines))


def run_realistic_benchmarks() -> list:
    """Benchmark with tests that do actual work (~10ms each)."""
    sizes = [50, 100, 250, 500, 1000]
    results = []

    for size in sizes:
        print(f"\n{'='*60}")
        print(f"  Realistic Benchmark: {size} tests (~10ms each)  ({TRIALS} trials, {WARMUP} warmup)")
        print(f"{'='*60}")

        suite_dir = BENCH_TMP / f"real_{size}"
        if suite_dir.exists():
            shutil.rmtree(suite_dir)
        suite_dir.mkdir(parents=True)

        try:
            generate_realistic_suite(suite_dir, size)

            # Verify
            verify = subprocess.run(
                [str(FASTEST_BIN), "discover", str(suite_dir)],
                cwd=str(REPO_ROOT),
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
            discovered = verify.stdout.decode().strip().split("\n")[-1]
            print(f"  Fastest discovered: {discovered}")

            # --- Fastest (parallel) ---
            print(f"  [fastest] Running ...", end="", flush=True)
            fastest_stats = time_command(
                [str(FASTEST_BIN), str(suite_dir), "--no-progress", "-q"],
                cwd=REPO_ROOT,
            )
            print(f"  {fastest_stats['mean']:.3f}s (mean)")

            # --- Pytest (sequential) ---
            print(f"  [pytest]  Running ...", end="", flush=True)
            pytest_stats = time_command(
                [PYTHON, "-m", "pytest", str(suite_dir), "-q", "--tb=no", "-p", "no:cacheprovider"],
                cwd=REPO_ROOT,
            )
            print(f"  {pytest_stats['mean']:.3f}s (mean)")

            speedup = pytest_stats["mean"] / fastest_stats["mean"] if fastest_stats["mean"] > 0 else 0

            row = {
                "size": size,
                "fastest": fastest_stats,
                "pytest": pytest_stats,
                "speedup": round(speedup, 2),
            }
            results.append(row)
            print(f"  >> Speedup: {speedup:.1f}x")

        finally:
            shutil.rmtree(suite_dir, ignore_errors=True)

    return results


# ---------------------------------------------------------------------------
# Entry-point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    # Verify prerequisites
    if not FASTEST_BIN.exists():
        sys.exit(f"ERROR: fastest binary not found at {FASTEST_BIN}\n"
                 f"Run `cargo build --release` first.")

    print("=" * 60)
    print("  Fastest vs Pytest -- Runtime Benchmark")
    print(f"  fastest: {FASTEST_BIN}")
    print(f"  pytest:  {PYTHON} -m pytest")
    print(f"  sizes:   {SUITE_SIZES}")
    print(f"  trials:  {TRIALS}  warmup: {WARMUP}")
    print("=" * 60)

    # Clean tmp
    if BENCH_TMP.exists():
        shutil.rmtree(BENCH_TMP)
    BENCH_TMP.mkdir(parents=True)

    # Run all benchmarks
    exec_results = run_execution_benchmarks()
    disc_results = run_discovery_benchmarks()

    # Save raw results
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    all_results = {
        "execution": exec_results,
        "discovery": disc_results,
        "metadata": {
            "fastest_bin": str(FASTEST_BIN),
            "python": PYTHON,
            "trials": TRIALS,
            "warmup": WARMUP,
        },
    }
    results_path = RESULTS_DIR / "results.json"
    with open(results_path, "w") as f:
        json.dump(all_results, f, indent=2)
    print(f"\nRaw results saved to {results_path}")

    # Print summary tables
    print(f"\n{'='*60}")
    print("  EXECUTION SUMMARY")
    print(f"{'='*60}")
    print(f"{'Size':>6} {'Fastest':>10} {'Pytest':>10} {'Speedup':>10}")
    print("-" * 40)
    for r in exec_results:
        print(f"{r['size']:>6} {r['fastest']['mean']:>9.3f}s {r['pytest']['mean']:>9.3f}s {r['speedup']:>9.1f}x")

    print(f"\n{'='*60}")
    print("  DISCOVERY SUMMARY")
    print(f"{'='*60}")
    print(f"{'Files':>6} {'Tests':>7} {'Fastest':>10} {'Pytest':>10} {'Speedup':>10}")
    print("-" * 48)
    for r in disc_results:
        print(f"{r['n_files']:>6} {r['total_tests']:>7} {r['fastest']['mean']:>9.3f}s {r['pytest']['mean']:>9.3f}s {r['speedup']:>9.1f}x")

    # Run realistic execution benchmark (tests with actual work)
    realistic_results = run_realistic_benchmarks()

    print(f"\n{'='*60}")
    print("  REALISTIC EXECUTION SUMMARY (tests with ~10ms work each)")
    print(f"{'='*60}")
    print(f"{'Size':>6} {'Fastest':>10} {'Pytest':>10} {'Speedup':>10}")
    print("-" * 40)
    for r in realistic_results:
        print(f"{r['size']:>6} {r['fastest']['mean']:>9.3f}s {r['pytest']['mean']:>9.3f}s {r['speedup']:>9.1f}x")

    # Re-save with realistic results
    all_results["realistic"] = realistic_results
    with open(results_path, "w") as f:
        json.dump(all_results, f, indent=2)

    # Cleanup
    shutil.rmtree(BENCH_TMP, ignore_errors=True)

    print(f"\nNow run:  python benchmarks/generate_graphs.py")
