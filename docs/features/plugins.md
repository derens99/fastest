# Plugin Compatibility

Fastest has plugin scaffolding and a pytest-compatible shim layer for common
plugin-style behavior. The current compatibility suite for plugins passes under
Fastest, but this is still not the same as broad third-party plugin ecosystem
support.

## Current Verified Gate

Run the plugin compatibility category with:

```bash
PYO3_PYTHON=$(command -v python3.12 || command -v python3) \
  cargo run -p fastest-cli -- pytest-compat-suite/features/plugins --no-cache
```

The full generated baseline is available through:

```bash
make compat-report-all
```

Run the narrow third-party package smoke gate with:

```bash
make plugin-smoke
```

That gate verifies the dev environment contains `pytest-asyncio`, `pytest-cov`,
`pytest-mock`, `pytest-timeout`, and `pytest-xdist`, then runs a small supported
shim subset under both Fastest and pytest.

## Supported Shim Behavior

The current worker supports:

- `pytest.hookimpl` and `pytest.hookspec` as no-op-compatible decorators.
- Common request helpers such as `request.node.iter_markers()`,
  `request.node.get_closest_marker()`, and `request.config.workerinput`.
- Builtin fixtures used by plugin-style tests, including `mocker`, `event_loop`,
  `cache`, `tmp_path`, and `tmpdir_factory`.
- A basic `mocker` API with `Mock`, `MagicMock`, `AsyncMock`, `PropertyMock`,
  `mock_open`, `patch`, `patch.object`, `spy`, `stub`, `resetall`, and
  `stopall`.

## Known Limits

- Third-party plugin validation is still narrow. The current smoke gate checks
  package availability and a supported subset; it does not prove arbitrary
  third-party plugin hooks work.
- Coverage flags are still experimental until they produce a verified report.
- Distributed execution and real xdist worker orchestration remain future work.
- Async generator fixtures are intentionally xfailed in the compatibility suite.

## Conftest Fixtures

Project-local fixtures in `conftest.py` are part of the supported compatibility
surface:

```python
import pytest

@pytest.fixture
def client():
    return {"status": "ready"}

def test_client(client):
    assert client["status"] == "ready"
```

## Plugin Development Direction

Near-term plugin work should focus on evidence-backed gates:

1. Expand smoke tests for real third-party plugins.
2. Promote only the plugin behaviors covered by passing gates.
3. Keep unsupported plugin hooks explicit in docs and test xfails.
4. Avoid marketing full pytest plugin compatibility until real plugin packages
   are verified.
