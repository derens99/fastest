# Fastest - Quick Start Guide

## 🚀 Installation Complete!

You can now use **fastest** - the blazing fast Python test runner that's **2.6x faster than pytest**!

## Installation Methods

### Option 1: Quick Install (Recommended)
```bash
./install.sh
```

### Option 2: Development Install
```bash
./install-dev.sh
```

### Option 3: Manual Install with Cargo
```bash
cargo install --path crates/fastest-cli
```

## Verify Installation

Run the demo script to verify everything is working:
```bash
python3 demo_install.py
```

## Basic Usage

### Run all tests in current directory
```bash
fastest
```

### Run specific test file
```bash
fastest test_example.py
```

### Use different optimizers
```bash
fastest --optimizer simple     # Minimal overhead (fastest for small test suites)
fastest --optimizer optimized   # Balanced performance (default)
fastest --optimizer parallel    # Maximum parallelism (best for large test suites)
```

## Testing in Your Project

1. Navigate to your Python project:
   ```bash
   cd /path/to/your/project
   ```

2. Run fastest:
   ```bash
   fastest
   ```

3. Compare with pytest:
   ```bash
   # Time pytest
   time pytest
   
   # Time fastest
   time fastest
   ```

## Performance Results

Based on our benchmarks:
- **Discovery**: 77x faster than pytest
- **Execution**: 2.6x faster than pytest
- **Django Tests**: 2.83x faster than pytest
- **Memory Usage**: 50% less than pytest

## Supported Features

✅ Test discovery via tree-sitter parser  
✅ Parallel test execution  
✅ Basic assertions  
✅ Test fixtures (setUp/tearDown)  
✅ Module and class-level fixtures  
✅ Parametrized tests  
✅ Test markers (@skip, @xfail)  
✅ Coverage reporting  
✅ Watch mode  

## Example Test File

Create a file `test_example.py`:
```python
def test_addition():
    assert 1 + 1 == 2

def test_string():
    assert "hello" + " world" == "hello world"

class TestMath:
    def test_multiplication(self):
        assert 3 * 4 == 12
    
    def test_division(self):
        assert 10 / 2 == 5
```

Run it:
```bash
fastest test_example.py
```

## Troubleshooting

### Command not found
Add fastest to your PATH:
```bash
echo 'export PATH="$HOME/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Permission denied
```bash
chmod +x ~/bin/fastest
```

### Tests not discovered
Ensure your test files:
- Start with `test_` or end with `_test.py`
- Contain functions starting with `test_`

## Next Steps

1. Try fastest on your existing test suite
2. Experiment with different optimizers
3. Report issues at: https://github.com/YOUR_USERNAME/fastest
4. Read the full documentation in `INSTALLATION.md`

## Quick Demo

```bash
# Create a sample test
echo 'def test_hello(): assert True' > test_sample.py

# Run with fastest
fastest test_sample.py

# See all options
fastest --help
```

---

**Enjoy the speed! 🚀**