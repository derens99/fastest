#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import os
import site
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


OUTCOME_NAMES = ("passed", "failed", "skipped", "xfailed", "xpassed")
ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")
SUMMARY_RE = re.compile(r"(\d+)\s+(passed|failed|skipped|xfailed|xpassed)")


@dataclass(frozen=True)
class SuiteResult:
    suite: str
    path: str
    command: list[str]
    returncode: int
    summary: dict[str, int]
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        return self.returncode == 0

    def to_json(self) -> dict[str, object]:
        return {
            "suite": self.suite,
            "path": self.path,
            "command": self.command,
            "returncode": self.returncode,
            "ok": self.ok,
            "summary": self.summary,
        }


def strip_ansi(text: str) -> str:
    return ANSI_RE.sub("", text)


def parse_fastest_summary(output: str) -> dict[str, int]:
    clean_output = strip_ansi(output)
    summary = {name: 0 for name in OUTCOME_NAMES}

    for count, outcome in SUMMARY_RE.findall(clean_output):
        summary[outcome] += int(count)

    summary["total"] = sum(summary.values())
    return summary


def contains_python_tests(path: Path) -> bool:
    return any(child.is_file() and child.suffix == ".py" for child in path.iterdir())


def discover_suites(root: Path, requested: list[str]) -> list[Path]:
    root = root.resolve()
    if requested:
        suites = []
        for suite in requested:
            suite_path = (root / suite).resolve()
            if not suite_path.exists():
                raise FileNotFoundError(
                    f"Compatibility suite does not exist: {suite_path}"
                )
            if not suite_path.is_dir():
                raise NotADirectoryError(
                    f"Compatibility suite is not a directory: {suite_path}"
                )
            suites.append(suite_path)
        return suites

    suites = [
        path
        for path in root.rglob("*")
        if path.is_dir() and contains_python_tests(path)
    ]
    return sorted(suites)


def normalize_timeout_output(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        return value.decode(errors="replace")
    return value


def fastest_child_env() -> dict[str, str]:
    env = os.environ.copy()
    site_packages = [path for path in site.getsitepackages() if Path(path).exists()]
    existing_pythonpath = env.get("PYTHONPATH", "")
    pythonpath_parts = site_packages + (
        [existing_pythonpath] if existing_pythonpath else []
    )
    if pythonpath_parts:
        env["PYTHONPATH"] = os.pathsep.join(pythonpath_parts)
    return env


def run_suite(
    fastest_binary: Path,
    suite_path: Path,
    no_cache: bool,
    timeout_seconds: float,
) -> SuiteResult:
    command = [str(fastest_binary), str(suite_path)]
    if no_cache:
        command.append("--no-cache")

    try:
        completed = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            env=fastest_child_env(),
        )
    except subprocess.TimeoutExpired as exc:
        stdout = normalize_timeout_output(exc.stdout)
        stderr = normalize_timeout_output(exc.stderr)
        if stderr:
            stderr += "\n"
        stderr += f"timed out after {timeout_seconds:g}s"
        output = stdout + stderr
        return SuiteResult(
            suite=str(suite_path),
            path=str(suite_path),
            command=command,
            returncode=124,
            summary=parse_fastest_summary(output),
            stdout=stdout,
            stderr=stderr,
        )

    output = completed.stdout + completed.stderr
    summary = parse_fastest_summary(output)

    return SuiteResult(
        suite=str(suite_path),
        path=str(suite_path),
        command=command,
        returncode=completed.returncode,
        summary=summary,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )


def print_text_report(results: list[SuiteResult]) -> None:
    print("Compatibility Report")
    print("====================")
    for result in results:
        status = "PASS" if result.ok else "FAIL"
        summary = result.summary
        if summary["total"] == 0 and result.returncode != 0:
            print(
                f"{status} {result.suite}: no Fastest summary parsed "
                f"(returncode {result.returncode})"
            )
            continue
        print(
            f"{status} {result.suite}: "
            f"{summary['passed']} passed, "
            f"{summary['failed']} failed, "
            f"{summary['skipped']} skipped, "
            f"{summary['xfailed']} xfailed, "
            f"{summary['xpassed']} xpassed"
        )


def write_json_report(results: list[SuiteResult], output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "suites": [result.to_json() for result in results],
        "ok": all(result.ok for result in results),
    }
    output_path.write_text(json.dumps(payload, indent=2) + "\n")


def report_exit_code(results: list[SuiteResult], allow_failures: bool) -> int:
    if allow_failures:
        return 0
    return 0 if all(result.ok for result in results) else 1


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run Fastest compatibility suites and report results."
    )
    parser.add_argument(
        "suites", nargs="*", help="Suite paths relative to pytest-compat-suite"
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("pytest-compat-suite"),
        help="Compatibility suite root",
    )
    parser.add_argument(
        "--fastest-binary",
        type=Path,
        default=Path("target/debug/fastest"),
        help="Fastest binary to execute",
    )
    parser.add_argument(
        "--json-output",
        type=Path,
        help="Write a machine-readable JSON report to this path",
    )
    parser.add_argument(
        "--no-cache",
        action="store_true",
        default=True,
        help="Pass --no-cache to Fastest runs",
    )
    parser.add_argument(
        "--use-cache",
        action="store_false",
        dest="no_cache",
        help="Allow Fastest discovery cache during compatibility runs",
    )
    parser.add_argument(
        "--allow-failures",
        action="store_true",
        help="Return success after writing the report even when suites fail",
    )
    parser.add_argument(
        "--suite-timeout",
        type=float,
        default=60.0,
        help="Maximum seconds allowed for each compatibility suite",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if not args.fastest_binary.exists():
        print(f"Fastest binary not found: {args.fastest_binary}", file=sys.stderr)
        return 2

    suites = discover_suites(args.root, args.suites)
    if not suites:
        print(f"No compatibility suites found under {args.root}", file=sys.stderr)
        return 2

    results = [
        run_suite(args.fastest_binary, suite, args.no_cache, args.suite_timeout)
        for suite in suites
    ]
    print_text_report(results)

    if args.json_output:
        write_json_report(results, args.json_output)

    return report_exit_code(results, args.allow_failures)


if __name__ == "__main__":
    raise SystemExit(main())
