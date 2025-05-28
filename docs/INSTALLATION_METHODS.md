# 🚀 Installation Methods Comparison

## Method 1: Shell Script (Recommended)

**Best for: Most users, CI/CD, quick setup**

```bash
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
```

✅ **Pros:**
- One command installation
- Automatically detects platform (Linux, macOS, Windows)
- Downloads optimized binary for your system
- Sets up PATH automatically
- No Python dependencies required
- Smallest installation size
- Best performance (native binary)

❌ **Cons:**
- Requires curl
- Downloads from GitHub releases

---

## Method 2: pip install (Python Integration)

**Best for: Python projects, virtual environments, existing pip workflows**

```bash
pip install fastest-runner
```

✅ **Pros:**
- Integrates with Python packaging ecosystem
- Works with virtual environments
- Can be included in requirements.txt
- Familiar pip workflow
- Automatic dependency management

❌ **Cons:**
- Requires fastest binary to be installed separately
- Additional Python wrapper overhead (minimal)
- Larger installation size

### Combined Installation (Recommended for Python users):

```bash
# Install the binary first
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh

# Then install the Python package
pip install fastest-runner
```

---

## Method 3: Build from Source

**Best for: Developers, contributors, custom builds**

```bash
git clone https://github.com/derens99/fastest.git
cd fastest
cargo build --release
cargo install --path crates/fastest-cli
```

✅ **Pros:**
- Latest development features
- Full control over build options
- Can modify source code
- Optimize for specific system

❌ **Cons:**
- Requires Rust toolchain
- Longer installation time
- More complex setup

---

## Performance Comparison

| Installation Method | Binary Size | Install Time | Runtime Performance |
|---------------------|------------|--------------|-------------------|
| Shell Script | ~5MB | ~10s | 🔥 Fastest |
| pip install | ~5MB + Python overhead | ~30s | 🔥 Fastest* |
| Build from Source | ~5MB | ~2-5min | 🔥 Fastest |

*Python wrapper adds minimal overhead (~1ms)

---

## Platform-Specific Notes

### macOS
- Shell script: Works on Intel and Apple Silicon
- Homebrew: `brew install fastest` (coming soon)
- pip: Works with any Python installation

### Linux
- Shell script: Works on x86_64 and ARM64
- pip: Works with system Python or virtualenv
- Package managers: `.deb` and `.rpm` packages (coming soon)

### Windows
- Shell script: Works with PowerShell or WSL
- pip: Works with any Python installation
- Chocolatey: `choco install fastest` (coming soon)

---

## CI/CD Integration

### GitHub Actions
```yaml
- name: Install fastest
  run: curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
  
- name: Run tests
  run: fastest tests/ --output json
```

### GitLab CI
```yaml
test:
  before_script:
    - curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
  script:
    - fastest tests/
```

### Docker
```dockerfile
# Option 1: Add to existing Python image
RUN curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh

# Option 2: Multi-stage build
FROM rust:1.70 as builder
COPY . .
RUN cargo build --release

FROM python:3.11-slim
COPY --from=builder /app/target/release/fastest /usr/local/bin/
```

---

## Verification

After installation with any method:

```bash
# Check version
fastest --version

# Test basic functionality
fastest --help

# Run sample tests
fastest tests/ --verbose
```

---

## Troubleshooting

### Binary not found after pip install
The Python package requires the fastest binary to be installed separately:
```bash
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
```

### Permission denied
```bash
# Install to user directory (default)
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh

# Or install to system directory (requires sudo)
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sudo sh -s -- --install-dir /usr/local/bin
```

### PATH issues
```bash
# Add to PATH manually
export PATH="$HOME/.local/bin:$PATH"

# Make permanent (adjust for your shell)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
```

---

## Next Steps

After installation, see:
- [Quick Start Guide](QUICKSTART.md)
- [Performance Guide](PERFORMANCE.md) 
- [Configuration](CONFIG.md)
- [Migration from pytest](MIGRATION_GUIDE.md)