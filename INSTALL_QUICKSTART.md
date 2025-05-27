# Quick Installation & Testing Guide

This guide will help you install and test `fastest` on your system in just a few minutes.

## 🚀 Quick Install (macOS/Linux)

### Option 1: Install from this repository (Recommended for testing)

```bash
# Clone this repository
git clone https://github.com/yourusername/fastest.git
cd fastest

# Run the development install script
./install-dev.sh

# Add to PATH if needed (the script will tell you)
export PATH="$HOME/.local/bin:$PATH"
```

### Option 2: Download pre-built binary

```bash
# Download latest release (replace URL with actual release)
curl -LO https://github.com/yourusername/fastest/releases/download/v0.1.0/fastest-aarch64-apple-darwin.tar.gz

# Extract
tar -xzf fastest-*.tar.gz

# Install
mkdir -p ~/.local/bin
mv fastest ~/.local/bin/
chmod +x ~/.local/bin/fastest

# Add to PATH
export PATH="$HOME/.local/bin:$PATH"
```

## 🧪 Test in a New Project

### Automatic Setup (Recommended)

We've created a quickstart script that sets up a demo project:

```bash
# From the fastest repository directory
python3 quickstart.py
```

This will:
1. Create a `fastest-demo` directory with sample tests
2. Show you how to run fastest with different optimizers
3. Optionally run performance comparisons

### Manual Setup

1. **Create a new Python project**:
```bash
mkdir my-project
cd my-project
mkdir tests
```

2. **Create a simple test file** `tests/test_example.py`:
```python
def test_addition():
    assert 1 + 1 == 2

def test_string():
    assert "hello".upper() == "HELLO"

class TestMath:
    def test_multiplication(self):
        assert 3 * 4 == 12
```

3. **Run fastest**:
```bash
# Run all tests
fastest tests/

# Run with specific optimizer
fastest tests/ --optimizer simple    # Fastest for simple tests
fastest tests/ --optimizer lightning # Balanced performance

# Run specific file
fastest tests/test_example.py

# Verbose output
fastest tests/ -v
```

## 📊 Compare with pytest

```bash
# Install pytest if not already installed
pip install pytest

# Time both
time pytest tests/ -q
time fastest tests/ --optimizer simple
```

## ⚙️ Configuration

Create a `pytest.ini` in your project root:

```ini
[tool:pytest]
# Fastest settings
fastest_optimizer = lightning
fastest_workers = 4
```

Or use `pyproject.toml`:

```toml
[tool.fastest]
optimizer = "lightning"
workers = 4
```

## 🎯 Performance Tips by Test Suite Size

| Test Count | Recommended Optimizer | Expected Speedup |
|------------|----------------------|------------------|
| < 50 | `--optimizer simple` | 2-3x faster |
| 50-500 | `--optimizer lightning` | 1.5-2.5x faster |
| 500+ | `--optimizer ultra` | 1.5-2x faster |

## 🔧 Troubleshooting

### "fastest: command not found"
```bash
# Check if installed
ls ~/.local/bin/fastest

# Add to current session
export PATH="$HOME/.local/bin:$PATH"

# Add permanently to ~/.bashrc or ~/.zshrc
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Build from source issues
```bash
# Ensure Rust is installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update Rust
rustup update

# Clean build
cargo clean
cargo build --release
```

## 📈 Example Performance Results

On a typical test suite:
- **pytest**: 170ms
- **fastest (simple)**: 66ms (**2.6x faster** 🚀)
- **fastest (lightning)**: 113ms (**1.5x faster** ⚡)

## 🆘 Getting Help

- Run `fastest --help` for command options
- Check [INSTALLATION.md](INSTALLATION.md) for detailed install instructions
- See [README.md](README.md) for full documentation

## 🎉 Next Steps

1. Try fastest on your existing test suite
2. Experiment with different optimizers
3. Configure for your specific needs
4. Enjoy faster test runs!

Happy testing! 🚀