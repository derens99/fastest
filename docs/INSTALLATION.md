# Installation Guide

Fastest can be installed in several ways depending on your needs and platform.

## Quick Install

### One-line Install (Recommended)

**macOS and Linux:**
```bash
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/yourusername/fastest/main/install.ps1 | iex
```

This will:
- Download the latest release for your platform
- Install it to `~/.local/bin` (Unix) or `%LOCALAPPDATA%\fastest\bin` (Windows)
- Add the installation directory to your PATH

### Install via pip

```bash
pip install fastest-runner
```

The pip package will automatically download the appropriate binary for your platform on first run.

### Install via Cargo

If you have Rust installed:

```bash
cargo install fastest-cli
```

### Install via Homebrew (macOS)

```bash
brew tap yourusername/fastest
brew install fastest
```

## Platform-Specific Instructions

### macOS

1. **Via installer script (recommended):**
   ```bash
   curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh
   ```

2. **Via Homebrew:**
   ```bash
   brew tap yourusername/fastest
   brew install fastest
   ```

3. **Manual installation:**
   - Download the latest release from [GitHub Releases](https://github.com/yourusername/fastest/releases)
   - Extract: `tar -xzf fastest-*-apple-darwin.tar.gz`
   - Move to PATH: `sudo mv fastest /usr/local/bin/`

### Linux

1. **Via installer script (recommended):**
   ```bash
   curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh
   ```

2. **Via package managers:**
   ```bash
   # Arch Linux (AUR)
   yay -S fastest-bin
   
   # Ubuntu/Debian (via cargo)
   cargo install fastest-cli
   ```

3. **Manual installation:**
   - Download the latest release for your architecture
   - Extract: `tar -xzf fastest-*-linux-gnu.tar.gz`
   - Move to PATH: `sudo mv fastest /usr/local/bin/`

### Windows

1. **Via PowerShell installer (recommended):**
   ```powershell
   irm https://raw.githubusercontent.com/yourusername/fastest/main/install.ps1 | iex
   ```

2. **Via Scoop:**
   ```powershell
   scoop bucket add fastest https://github.com/yourusername/scoop-fastest
   scoop install fastest
   ```

3. **Manual installation:**
   - Download `fastest-*-pc-windows-msvc.zip` from releases
   - Extract the zip file
   - Add the directory to your PATH

## Docker

Run fastest in a container:

```bash
docker run --rm -v $(pwd):/workspace ghcr.io/yourusername/fastest tests/
```

Or add to your `docker-compose.yml`:

```yaml
services:
  test:
    image: ghcr.io/yourusername/fastest:latest
    volumes:
      - .:/workspace
    command: tests/ -v
```

## Installer Options

### Shell Installer Options

```bash
# Install specific version
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh -s -- --version v0.2.0

# Install to custom directory
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh -s -- --install-dir /opt/fastest

# Show help
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh -s -- --help
```

### PowerShell Installer Options

```powershell
# Install specific version
irm https://raw.githubusercontent.com/yourusername/fastest/main/install.ps1 | iex -Version "v0.2.0"

# Install to custom directory
irm https://raw.githubusercontent.com/yourusername/fastest/main/install.ps1 | iex -InstallDir "C:\tools\fastest"
```

## Verifying Installation

After installation, verify that fastest is working:

```bash
# Check version
fastest --version

# Run help
fastest --help

# Run a simple test
echo "def test_example(): assert True" > test_example.py
fastest test_example.py
```

## Updating

### Update via installer

**Unix:**
```bash
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh
```

**Windows:**
```powershell
irm https://raw.githubusercontent.com/yourusername/fastest/main/install.ps1 | iex
```

### Update via pip

```bash
pip install --upgrade fastest-runner
```

### Update via cargo

```bash
cargo install --force fastest-cli
```

## Uninstalling

### Installed via script

**Unix:**
```bash
rm ~/.local/bin/fastest
# Remove from PATH in ~/.bashrc, ~/.zshrc, etc.
```

**Windows:**
```powershell
Remove-Item "$env:LOCALAPPDATA\fastest" -Recurse
# Remove from PATH via System Properties
```

### Installed via pip

```bash
pip uninstall fastest-runner
```

### Installed via cargo

```bash
cargo uninstall fastest-cli
```

## Troubleshooting

### Common Issues

1. **"command not found" after installation**
   - Restart your shell or run `source ~/.bashrc` (or equivalent)
   - Check that the install directory is in your PATH

2. **Permission denied**
   - Don't use `sudo` with the installer script
   - The default install location (`~/.local/bin`) doesn't require root

3. **SSL/TLS errors**
   - Update your certificates: `update-ca-certificates` (Linux)
   - Use `--insecure` flag (not recommended)

4. **Wrong architecture downloaded**
   - Manually specify platform in download URL
   - Check `uname -m` output

### Getting Help

If you encounter issues:

1. Check the [troubleshooting guide](TROUBLESHOOTING.md)
2. Search [existing issues](https://github.com/yourusername/fastest/issues)
3. Open a new issue with:
   - Your OS and architecture
   - Installation method used
   - Complete error message
   - Output of `fastest --version` (if it runs)

## Building from Source

If you want to build from source:

```bash
# Clone the repository
git clone https://github.com/yourusername/fastest.git
cd fastest

# Build release binary
cargo build --release

# Install
cargo install --path crates/fastest-cli

# Or copy binary manually
cp target/release/fastest ~/.local/bin/
```

Requirements:
- Rust 1.75 or later
- Git 