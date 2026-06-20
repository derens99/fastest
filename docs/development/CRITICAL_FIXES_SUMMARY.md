# Critical Fixes Summary

This document is now an archival cleanup summary. It should not be used as a source for compatibility percentages, performance multipliers, or release readiness claims.

## Current Sources Of Truth

- `docs/development/WORKPLAN.md`
- `roadmap.md`
- `make verify`
- `make compat-report-all`
- Focused regression tests in `crates/fastest-cli/tests/integration_test.rs`

## Stabilization Completed

- Class lifecycle and teardown ordering regressions are covered by integration tests.
- Unicode and test-id handling are covered by project tests and compatibility suites.
- Compatibility suites are organized under `pytest-compat-suite/`.
- The compatibility report harness runs all discovered categories and records expected skips, xfails, xpasses, passes, and failures.
- The full generated compatibility report currently passes all discovered categories with expected skips and xfails.

## Current Documentation Policy

- Do not publish stale compatibility percentages such as 91% or 93%.
- Do not publish fixed performance multipliers unless they come from a fresh benchmark artifact for the current checkout.
- Prefer generated report output over manually maintained status numbers.
- Mark advanced execution and plugin behavior as experimental unless covered by tests.

## Remaining Cleanup Themes

- Keep docs aligned with the generated compatibility report.
- Keep PyO3 setup examples consistent across docs and CI.
- Revalidate performance strategy behavior with current benchmarks before advertising speedups.
- Expand third-party plugin smoke tests before making broad pytest ecosystem compatibility claims.
