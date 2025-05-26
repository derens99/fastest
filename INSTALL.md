# Installing Fastest in a Virtual Environment

This guide shows you how to install Fastest in a Python virtual environment for local development and testing.

## Prerequisites

- Python 3.8 or higher
- Rust toolchain (for building from source)
- Git

## Method 1: Install from PyPI (Recommended)

```bash
# Create a virtual environment
python3 -m venv myproject-env

# Activate the virtual environment
# On macOS/Linux:
source myproject-env/bin/activate
# On Windows:
# myproject-env\Scripts\activate

# Install fastest from PyPI
pip install fastest

# Verify installation
fastest --version
```

## Method 2: Install from Source

### Step 1: Clone the Repository

```bash
git clone https://github.com/derens99/fastest.git
cd fastest
```

### Step 2: Create and Activate Virtual Environment

```bash
# Create virtual environment in your project
python3 -m venv venv

# Activate it
# On macOS/Linux:
source venv/bin/activate
# On Windows:
# venv\Scripts\activate
```

### Step 3: Build and Install

#### Option A: Using the Install Script (Easiest)

```bash
# Run the install script
./scripts/install.sh

# Or on Windows:
# python scripts/install.py
```

#### Option B: Manual Build and Install

```bash
# Build the Rust components
cargo build --release

# Install the Python package in editable mode
pip install -e python/

# Or install it normally
pip install python/
```

#### Option C: Using Maturin (Development Mode)

```bash
# Install maturin
pip install maturin

# Build and install in development mode
cd python
maturin develop --release

# Or build a wheel and install it
maturin build --release
pip install target/wheels/fastest-*.whl
```

## Method 3: Install Pre-built Wheels

If pre-built wheels are available for your platform:

```bash
# Activate your virtual environment
source myproject-env/bin/activate

# Install directly from GitHub releases
pip install https://github.com/derens99/fastest/releases/download/v0.1.1/fastest-0.1.1-py3-none-any.whl
```

## Verifying the Installation

After installation, verify that Fastest is working:

```bash
# Check version
fastest --version

# Run help
fastest --help

# Test on a sample project
fastest tests/
```

## Using Fastest in Your Project

### Basic Usage

```bash
# Run all tests
fastest

# Run specific directory
fastest tests/

# Run with options
fastest -v -n 4  # Verbose mode with 4 workers
```

### Integration with Development Workflow

1. **Add to requirements-dev.txt**:
   ```
   fastest>=0.1.0
   ```

2. **Create an alias** (optional):
   ```bash
   # Add to your .bashrc/.zshrc
   alias ft='fastest'
   ```

3. **VS Code Integration**:
   Add to `.vscode/settings.json`:
   ```json
   {
     "python.testing.fastestEnabled": true,
     "python.testing.pytestEnabled": false
   }
   ```

## Troubleshooting

### Common Issues

1. **"fastest: command not found"**
   - Make sure your virtual environment is activated
   - Check if the package is installed: `pip list | grep fastest`
   - Try: `python -m fastest` instead

2. **"No module named 'fastest'"**
   - Ensure you're in the correct virtual environment
   - Reinstall: `pip install --force-reinstall fastest`

3. **Build errors on macOS**
   - Install Xcode Command Line Tools: `xcode-select --install`
   - Ensure Rust is installed: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

4. **Build errors on Linux**
   - Install Python development headers: `sudo apt-get install python3-dev`
   - Install build essentials: `sudo apt-get install build-essential`

### Virtual Environment Best Practices

1. **Project-specific environments**:
   ```bash
   cd myproject
   python3 -m venv .venv
   source .venv/bin/activate
   pip install fastest
   ```

2. **Using pyenv for Python version management**:
   ```bash
   pyenv install 3.11.0
   pyenv local 3.11.0
   python -m venv .venv
   source .venv/bin/activate
   pip install fastest
   ```

3. **Using Poetry**:
   ```bash
   poetry add --group dev fastest
   poetry run fastest tests/
   ```

## Uninstalling

To remove Fastest from your virtual environment:

```bash
pip uninstall fastest
```

## Next Steps

- Check out the [Migration Guide](docs/MIGRATION_GUIDE.md) to migrate from pytest
- Read about [configuration options](docs/CONFIG.md)
- Learn about [performance optimization](docs/PERFORMANCE.md) 