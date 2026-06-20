from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "development" / "compatibility_report.py"


def load_report_module():
    spec = importlib.util.spec_from_file_location("compatibility_report", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec is not None
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_parse_fastest_summary_counts_outcomes():
    report = load_report_module()
    output = """
    \x1b[1mOK\x1b[0m \x1b[32m39 passed\x1b[0m, \x1b[33m1 xfailed\x1b[0m in 1.44s
    """

    summary = report.parse_fastest_summary(output)

    assert summary == {
        "passed": 39,
        "failed": 0,
        "skipped": 0,
        "xfailed": 1,
        "xpassed": 0,
        "total": 40,
    }


def test_parse_fastest_summary_handles_failures_and_xpass():
    report = load_report_module()
    output = "FAIL 12 passed, 3 failed, 2 skipped, 1 xfailed, 4 xpassed in 0.50s"

    summary = report.parse_fastest_summary(output)

    assert summary["passed"] == 12
    assert summary["failed"] == 3
    assert summary["skipped"] == 2
    assert summary["xfailed"] == 1
    assert summary["xpassed"] == 4
    assert summary["total"] == 22


def test_discover_requested_suites_resolves_relative_paths(tmp_path):
    report = load_report_module()
    root = tmp_path / "pytest-compat-suite"
    (root / "core" / "basic").mkdir(parents=True)
    (root / "features" / "fixtures").mkdir(parents=True)
    (root / "features" / "fixtures" / "test_fixture.py").write_text(
        "def test_ok(): pass\n"
    )

    suites = report.discover_suites(root, ["features/fixtures"])

    assert suites == [root / "features" / "fixtures"]


def test_discover_all_suites_returns_leaf_directories_with_tests(tmp_path):
    report = load_report_module()
    root = tmp_path / "pytest-compat-suite"
    (root / "core" / "basic").mkdir(parents=True)
    (root / "core" / "basic" / "test_basic.py").write_text("def test_ok(): pass\n")
    (root / "features" / "fixtures").mkdir(parents=True)
    (root / "features" / "fixtures" / "test_fixture.py").write_text(
        "def test_ok(): pass\n"
    )
    (root / "features" / "empty").mkdir(parents=True)

    suites = report.discover_suites(root, [])

    assert suites == [root / "core" / "basic", root / "features" / "fixtures"]


def test_report_exit_code_can_allow_failures():
    report = load_report_module()
    passing = report.SuiteResult(
        suite="core/basic",
        path="pytest-compat-suite/core/basic",
        command=["fastest", "pytest-compat-suite/core/basic"],
        returncode=0,
        summary={
            "passed": 1,
            "failed": 0,
            "skipped": 0,
            "xfailed": 0,
            "xpassed": 0,
            "total": 1,
        },
        stdout="",
        stderr="",
    )
    failing = report.SuiteResult(
        suite="features/plugins",
        path="pytest-compat-suite/features/plugins",
        command=["fastest", "pytest-compat-suite/features/plugins"],
        returncode=1,
        summary={
            "passed": 1,
            "failed": 1,
            "skipped": 0,
            "xfailed": 0,
            "xpassed": 0,
            "total": 2,
        },
        stdout="",
        stderr="",
    )

    assert report.report_exit_code([passing, failing], allow_failures=False) == 1
    assert report.report_exit_code([passing, failing], allow_failures=True) == 0


def test_text_report_calls_out_missing_summary(capsys):
    report = load_report_module()
    result = report.SuiteResult(
        suite="features/plugins",
        path="pytest-compat-suite/features/plugins",
        command=["fastest", "pytest-compat-suite/features/plugins"],
        returncode=-9,
        summary={
            "passed": 0,
            "failed": 0,
            "skipped": 0,
            "xfailed": 0,
            "xpassed": 0,
            "total": 0,
        },
        stdout="",
        stderr="",
    )

    report.print_text_report([result])

    output = capsys.readouterr().out
    assert "no Fastest summary parsed" in output
    assert "returncode -9" in output


def test_run_suite_reports_timeout(tmp_path):
    report = load_report_module()
    fake_fastest = tmp_path / "fake_fastest"
    fake_fastest.write_text(
        "#!/usr/bin/env python3\n"
        "import time\n"
        "time.sleep(10)\n"
    )
    fake_fastest.chmod(0o755)
    suite = tmp_path / "suite"
    suite.mkdir()

    result = report.run_suite(
        fake_fastest,
        suite,
        no_cache=True,
        timeout_seconds=0.01,
    )

    assert result.returncode == 124
    assert result.summary["total"] == 0
    assert "timed out" in result.stderr
