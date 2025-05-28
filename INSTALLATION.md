# 📦 Installation Guide

> ⚡ Multiple ways to install fastest - choose what works best for you!

## 🚀 Quick Install (Recommended)

**One-line installation using our installer script:**

```bash
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
```

This automatically:
- Detects your platform (Linux, macOS, Windows)
- Downloads the latest release
- Installs to `~/.local/bin/`
- Adds to your PATH
- Verifies the installation

## 🐍 Install via pip

**Option 1: Install from PyPI (when available):**
```bash
pip install fastest-runner
```

**Option 2: Install from source:**
```bash
pip install git+https://github.com/derens99/fastest.git
```

## 🍺 Install via Homebrew (macOS)

*Coming soon...*

```bash
brew tap derens99/fastest
brew install fastest
```

## 📦 Install via Conda

*Coming soon...*

```bash
conda install -c conda-forge fastest-runner
```

## 🏗️ Build from Source

**Prerequisites:**
- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Python 3.8+

**Build steps:**
```bash
git clone https://github.com/derens99/fastest.git
cd fastest
cargo build --release
./target/release/fastest --help
```

**Install locally:**
```bash
cargo install --path crates/fastest-cli
```

## 🎯 Installation Options

### Custom Installation Directory

```bash
# Install to /usr/local/bin
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh -s -- --install-dir /usr/local/bin

# Or use environment variable
FASTEST_INSTALL_DIR=/usr/local/bin curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
```

### Specific Version

```bash
# Install specific version
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh -s -- --version v0.1.0

# Or use environment variable
FASTEST_VERSION=v0.1.0 curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
```

## ✅ Verify Installation

```bash
fastest --version
fastest --help
```

## 🚀 Quick Start

```bash
# Run tests in current directory
fastest

# Run specific test file
fastest test_example.py

# Run with verbose output (see which strategy is used)
fastest tests/ --verbose

# Run with custom workers
fastest tests/ --workers 8
```

## 🔥 Performance Features

Fastest automatically optimizes execution based on your test suite size:

- **≤20 tests**: In-process execution (10x faster startup)
- **21-100 tests**: Optimized worker pool (5x faster)  
- **>100 tests**: Full parallel execution (3x faster)

Use `--verbose` to see which strategy is selected!

## 🆘 Troubleshooting

### Installation Issues

**Binary not found after installation:**
```bash
# Check if directory is in PATH
echo $PATH

# Add to PATH manually (adjust for your shell)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

**Permission denied:**
```bash
# Make sure the binary is executable
chmod +x ~/.local/bin/fastest
```

**curl not found:**
```bash
# Install curl first
# Ubuntu/Debian:
sudo apt update && sudo apt install curl

# macOS (with Homebrew):
brew install curl

# Or download manually from GitHub releases
```

### Runtime Issues

**Python module not found:**
- Make sure your virtual environment is activated
- Install test dependencies: `pip install pytest` (if needed)

**Tests not discovered:**
- Fastest follows pytest conventions for test discovery
- Make sure test files start with `test_` or end with `_test.py`
- Make sure test functions start with `test_`

### Getting Help

- **Documentation**: https://github.com/derens99/fastest/tree/main/docs
- **Issues**: https://github.com/derens99/fastest/issues
- **Discussions**: https://github.com/derens99/fastest/discussions
- **Benchmarks**: https://github.com/derens99/fastest/tree/main/benchmarks

## 📈 Performance Comparison

| Test Suite Size | pytest | fastest | Speedup |
|-----------------|--------|---------|---------|
| Small (≤20 tests) | 0.8s | 0.08s | **10x faster** |
| Medium (50 tests) | 2.1s | 0.4s | **5x faster** |
| Large (200+ tests) | 8.5s | 2.8s | **3x faster** |

*Results may vary based on test complexity and system specifications.*

---

**Need help?** Open an issue on [GitHub](https://github.com/derens99/fastest/issues) or check our [documentation](https://github.com/derens99/fastest/tree/main/docs)!