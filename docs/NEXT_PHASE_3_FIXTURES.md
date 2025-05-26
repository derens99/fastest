# Phase 3: Fixture System Implementation Plan

## Overview
Fixtures are a critical part of pytest compatibility. We need to implement:
- Function-scoped fixtures (most common)
- Setup/teardown mechanics
- Fixture dependency resolution
- Basic built-in fixtures (tmp_path, capsys, etc.)

## Implementation Steps

### Step 1: Fixture Discovery (Week 1)
- [ ] Parse fixture definitions from test files
- [ ] Extract fixture parameters from test functions
- [ ] Build fixture dependency graph

### Step 2: Basic Function-Scoped Fixtures (Week 2)
- [ ] Implement fixture execution before tests
- [ ] Pass fixture values to test functions
- [ ] Handle fixture teardown after tests

### Step 3: Built-in Fixtures (Week 3)
- [ ] `tmp_path` - Temporary directory management
- [ ] `capsys` - Capture stdout/stderr
- [ ] `monkeypatch` - Monkey patching support
- [ ] `request` - Test request information

### Step 4: Advanced Scopes (Week 4)
- [ ] Module-scoped fixtures
- [ ] Session-scoped fixtures
- [ ] Class-scoped fixtures
- [ ] Fixture caching and reuse

## Technical Design

### Fixture Definition
```rust
pub struct FixtureDefinition {
    name: String,
    scope: FixtureScope,
    func: PyObject,  // Python function object
    deps: Vec<String>, // Other fixtures this depends on
}
```

### Fixture Resolution
```rust
pub struct FixtureResolver {
    definitions: HashMap<String, FixtureDefinition>,
    instances: HashMap<(String, Scope), PyObject>,
}
```

### Integration Points
1. **Discovery**: Extend parser to find `@pytest.fixture` decorators
2. **Execution**: Modify test runners to resolve and inject fixtures
3. **Python API**: Expose fixture decorator in fastest.py

## Success Criteria
- [ ] Support 80% of common fixture patterns
- [ ] Performance: <5ms overhead per fixture
- [ ] Memory: Proper cleanup of fixture instances
- [ ] Compatibility: Work with existing pytest fixtures

## Example Usage
```python
import fastest

@fastest.fixture
def sample_data():
    return {"key": "value"}

def test_with_fixture(sample_data):
    assert sample_data["key"] == "value"
``` 