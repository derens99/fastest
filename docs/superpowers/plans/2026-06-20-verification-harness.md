# Verification Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reproducible local verification target and a compatibility-suite report command.

**Architecture:** Keep the harness as a small Python script under `scripts/development/` so it can be tested with pytest and invoked by Makefile targets. The Makefile stays the user-facing entry point for local gates.

**Tech Stack:** Rust/Cargo, Make, Python via `uv`, pytest.

## Global Constraints

- Use `uv` for Python commands.
- Keep project tests that pass under pytest in `tests/`.
- Keep compatibility input suites under `pytest-compat-suite/`.
- Put markdown documentation under `docs/`.

---

### Task 1: Compatibility Report Script

**Files:**
- Create: `scripts/development/compatibility_report.py`
- Create: `tests/integration/test_compatibility_report.py`

**Interfaces:**
- Produces: `parse_fastest_summary(output: str) -> dict[str, int]`
- Produces: `discover_suites(root: Path, requested: list[str]) -> list[Path]`

- [x] Write failing parser and suite-discovery tests.
- [x] Run `uv run pytest tests/integration/test_compatibility_report.py -q` and confirm the missing script fails.
- [x] Implement the report script with JSON and text output.
- [x] Run the focused pytest file and confirm it passes.

### Task 2: Makefile Integration

**Files:**
- Modify: `Makefile`
- Modify: `docs/reference/roadmap.md`
- Modify: `docs/development/WORKPLAN.md`

**Interfaces:**
- Produces: `make compat-report`
- Produces: `make verify`

- [x] Add `compat-report` target that runs the script against selected suites.
- [x] Add `verify` target that runs lint, Rust tests, Python tests, and the core compatibility report.
- [x] Update docs with the new commands.
- [x] Run `make compat-report COMPAT_SUITES="core/basic features/fixtures"`.
- [x] Run `make verify`.
