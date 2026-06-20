# Installation Guide

Fastest is a Rust-backed Python test runner under active compatibility work.
This guide covers installation methods for development and local validation.

## 🚀 Quick Install (Recommended)

### macOS/Linux

```bash
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
```

### Windows (PowerShell)

```powershell
iwr -useb https://raw.githubusercontent.com/derens99/fastest/main/install.ps1 | iex
```

These scripts will:
- Download the latest binary for your platform
- Install it to `~/.local/bin` (Unix) or `%USERPROFILE%\.local\bin` (Windows)
- Add it to your PATH if needed

## 📦 Package Managers

### Using Cargo (All Platforms)

If you have Rust installed:

```bash
cargo install fastest-cli
```

### Using Homebrew (macOS)

```bash
brew install fastest
```

## 🔧 Development Installation

For contributors or those who want the latest development version:

```bash
# Clone the repository
git clone https://github.com/derens99/fastest.git
cd fastest

# Install development version
./scripts/install-dev.sh
```

This will:
- Build from source with optimizations
- Install development dependencies
- Set up pre-commit hooks
- Create a symlink for easy updates

## 📥 Manual Installation

### Step 1: Download Binary

Download the appropriate binary from the [releases page](https://github.com/derens99/fastest/releases):

- **macOS (Apple Silicon)**: `fastest-aarch64-apple-darwin.tar.gz`
- **macOS (Intel)**: `fastest-x86_64-apple-darwin.tar.gz`
- **Linux (x64)**: `fastest-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `fastest-aarch64-unknown-linux-gnu.tar.gz`
- **Windows (x64)**: `fastest-x86_64-pc-windows-msvc.zip`

### Step 2: Extract and Install

```bash
# Extract
tar -xzf fastest-*.tar.gz  # or unzip for Windows

# Make executable (Unix only)
chmod +x fastest

# Move to PATH
sudo mv fastest /usr/local/bin/  # or any directory in your PATH
```

## ✅ Verify Installation

```bash
# Check version
fastest version

# Run help
fastest --help

# Run a simple test
echo "def test_example(): assert True" > test_example.py
fastest test_example.py
```

## 🐍 Python Version Requirements

- Python 3.8 or higher
- Works with virtual environments (venv, conda, poetry)
- Automatically detects active Python environment

## 🧩 PyO3 Linking For Development

When building or testing from source, set `PYO3_PYTHON` to a linkable Python.
The Makefile defaults to `python3.12` when available, then falls back to
`python3`.

```bash
export PYO3_PYTHON="$(command -v python3.12 || command -v python3)"
make verify
```

If Cargo tries to link an unavailable system Python, set `PYO3_PYTHON`
explicitly and rerun the command.

## 🔄 Updating Fastest

### Self-Update (Easiest)

```bash
fastest update
```

### Using Installation Method

Re-run the installation command for your chosen method.

## 🚨 Troubleshooting

### Command Not Found

Add the installation directory to your PATH:

```bash
# Add to ~/.bashrc, ~/.zshrc, or equivalent
export PATH="$HOME/.local/bin:$PATH"
```

### Permission Denied

```bash
# Make binary executable
chmod +x ~/.local/bin/fastest
```

### Python Not Found

Ensure Python is installed and in PATH:

```bash
python --version  # or python3
```

## 📚 Next Steps

- [Quickstart Guide](quickstart.md) - Get running in 5 minutes
- [Migration from pytest](migration-guide.md) - Migrate existing test suites
- [Features Overview](../features/) - Learn about all features
