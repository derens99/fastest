# Installation Guide for Fastest

Fastest is a blazing-fast Python test runner written in Rust. Here are multiple ways to install it on your system.

## Quick Install (Recommended)

### Option 1: Using the install script (Unix/Linux/macOS)

```bash
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh
```

This will:
- Download the latest binary for your platform
- Install it to `~/.local/bin`
- Add it to your PATH if needed

### Option 2: Using Cargo (All platforms)

If you have Rust installed:

```bash
cargo install fastest-cli
```

## Manual Installation

### Step 1: Download the Binary

Download the appropriate binary for your platform from the [releases page](https://github.com/yourusername/fastest/releases):

- **macOS (Apple Silicon)**: `fastest-aarch64-apple-darwin.tar.gz`
- **macOS (Intel)**: `fastest-x86_64-apple-darwin.tar.gz`
- **Linux (x64)**: `fastest-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `fastest-aarch64-unknown-linux-gnu.tar.gz`
- **Windows**: `fastest-x86_64-pc-windows-msvc.zip`

### Step 2: Extract and Install

#### Unix/Linux/macOS:
```bash
# Extract the archive
tar -xzf fastest-*.tar.gz

# Move to a directory in your PATH
sudo mv fastest /usr/local/bin/
# OR for user-only installation:
mkdir -p ~/.local/bin
mv fastest ~/.local/bin/

# Make it executable
chmod +x ~/.local/bin/fastest

# Add to PATH if needed (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"
```

#### Windows:
1. Extract the ZIP file
2. Move `fastest.exe` to a directory of your choice
3. Add that directory to your PATH environment variable

## Building from Source

### Prerequisites
- Rust 1.70 or later
- Python 3.8+ (for running tests)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/yourusername/fastest.git
cd fastest

# Build in release mode
cargo build --release

# The binary will be at target/release/fastest
# Copy it to your PATH
sudo cp target/release/fastest /usr/local/bin/
```

## Python Package Installation (Optional)

For Python integration and fixtures support:

```bash
pip install fastest-runner
```

## Verification

After installation, verify it works:

```bash
# Check version
fastest --version

# Run help
fastest --help

# Run on a test file
fastest test_example.py
```

## Configuration

Create a `pytest.ini` or `pyproject.toml` file in your project root:

### pytest.ini
```ini
[tool:pytest]
# Fastest-specific settings
fastest_workers = 4
fastest_parser = tree-sitter
fastest_optimizer = lightning
```

### pyproject.toml
```toml
[tool.fastest]
workers = 4
parser = "tree-sitter"
optimizer = "lightning"
```

## Usage Examples

```bash
# Run all tests in current directory
fastest

# Run specific test file
fastest test_math.py

# Run tests in a directory
fastest tests/

# Use specific optimizer
fastest tests/ --optimizer simple    # Fastest for simple tests
fastest tests/ --optimizer lightning # Good balance
fastest tests/ --optimizer ultra     # For large test suites

# Run with multiple workers
fastest tests/ -n 8

# Filter tests by name
fastest tests/ -k "test_addition"

# Verbose output
fastest tests/ -v
```

## Performance Tips

1. **For small test suites (<100 tests)**: Use `--optimizer simple`
2. **For medium test suites (100-1000 tests)**: Use `--optimizer lightning`
3. **For large test suites (>1000 tests)**: Use `--optimizer ultra`
4. **For tests with fixtures**: Use `--optimizer optimized`

## Troubleshooting

### "Command not found" error
- Ensure the installation directory is in your PATH
- Try using the full path: `~/.local/bin/fastest`

### Permission denied
- Make sure the binary is executable: `chmod +x fastest`

### Python module not found errors
- Ensure you're running from the correct directory
- Check that your Python environment is activated

### Performance issues
- Try different optimizers
- Reduce the number of workers if you have many small tests

## Uninstallation

### If installed via script or manually:
```bash
rm /usr/local/bin/fastest
# or
rm ~/.local/bin/fastest
```

### If installed via Cargo:
```bash
cargo uninstall fastest-cli
```

### If installed via pip:
```bash
pip uninstall fastest-runner
```

## Support

- GitHub Issues: [github.com/yourusername/fastest/issues](https://github.com/yourusername/fastest/issues)
- Documentation: [github.com/yourusername/fastest/docs](https://github.com/yourusername/fastest/docs)

## System Requirements

- **OS**: Linux, macOS, Windows
- **Architecture**: x86_64, aarch64/arm64
- **Python**: 3.8 or later
- **Memory**: 512MB minimum
- **Disk Space**: 50MB for binary