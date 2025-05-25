# Test Scripts

This directory contains scripts for testing and demonstrating the `fastest` extension functionality.

## Test Scripts

### test_fastest.py
Basic test script to verify that the fastest extension is working correctly. It:
- Displays the version
- Discovers tests in the current directory
- Shows the first 5 discovered tests
- Attempts to run the first test found

### test_enhanced.py
Enhanced test script demonstrating newer features of the fastest extension:
- Pretty-printed test results with status icons
- Detailed test result information (stdout, stderr, duration)
- Async test detection
- Error handling demonstration

## Running Tests

To run the test scripts:

```bash
python tests/test_fastest.py
python tests/test_enhanced.py
```

Make sure `fastest` is installed first:
```bash
maturin develop
```

## Note
These are demonstration/validation scripts, not the actual unit tests for the fastest extension. The real unit tests would typically be in the Rust codebase under `crates/*/src/` or in a dedicated test directory. 