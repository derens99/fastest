from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "benchmarks" / "official.py"


def load_benchmark_module():
    spec = importlib.util.spec_from_file_location("official_benchmark", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec is not None
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_ensure_output_dir_creates_nested_artifact_path(tmp_path):
    benchmark = load_benchmark_module()
    output_dir = tmp_path / "target" / "benchmark-artifacts" / "quick"

    benchmark.ensure_output_dir(output_dir)

    assert output_dir.is_dir()


def test_extract_fastest_count_accepts_bare_numeric_line():
    benchmark = load_benchmark_module()
    runner = benchmark.OfficialBenchmark.__new__(benchmark.OfficialBenchmark)

    count = runner.extract_test_count(
        "\x1b[1;36mFastest v1.0.10 - Rust-backed Python test runner\x1b[0m\n148\n",
        "",
        is_fastest=True,
    )

    assert count == 148


def test_get_versions_uses_fastest_version_subcommand(tmp_path):
    benchmark = load_benchmark_module()
    fake_fastest = tmp_path / "fastest"
    fake_fastest.write_text(
        "#!/usr/bin/env python3\n"
        "import sys\n"
        "if sys.argv[1:] == ['version']:\n"
        "    print('Fastest v9.9.9 - Rust-backed Python test runner')\n"
        "    print('fastest 9.9.9')\n"
        "    raise SystemExit(0)\n"
        "if sys.argv[1:] == ['--version']:\n"
        "    print('unexpected argument --version', file=sys.stderr)\n"
        "    raise SystemExit(2)\n"
        "raise SystemExit(1)\n"
    )
    fake_fastest.chmod(0o755)
    runner = benchmark.OfficialBenchmark.__new__(benchmark.OfficialBenchmark)
    runner.fastest_binary = str(fake_fastest)

    fastest_version, _pytest_version = runner.get_versions()

    assert fastest_version == "fastest 9.9.9"


def test_save_results_uses_neutral_artifact_filenames(tmp_path):
    benchmark = load_benchmark_module()
    runner = benchmark.OfficialBenchmark.__new__(benchmark.OfficialBenchmark)
    runner.output_dir = tmp_path
    results = benchmark.BenchmarkSuite(
        timestamp="2026-06-20T00:00:00+00:00",
        system_info={
            "platform": "test-platform",
            "architecture": "arm64",
            "cpu_count": "10",
            "python_version": "3.12.0",
        },
        fastest_version="fastest 9.9.9",
        pytest_version="pytest 8.3.5",
        comparisons=[],
        summary={},
    )

    json_file = runner.save_json_results(results)
    md_file = runner.save_markdown_results(results)

    assert json_file == tmp_path / "benchmark_results.json"
    assert md_file == tmp_path / "benchmark_results.md"
    assert json_file.exists()
    assert md_file.exists()
