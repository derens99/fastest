# CI/CD Validation Guide

This guide helps prevent CI failures by catching issues before pushing to GitHub.

## Quick Setup

1. **Enable Git hooks** (one-time setup):
   ```bash
   ./scripts/setup-hooks.sh
   ```

2. **That's it!** The pre-push hook will now run automatically before each push.

## Manual Validation

To run validation checks manually:

```bash
./scripts/pre-push-check.sh
```

## What Gets Checked

The validation script runs the same checks as our CI pipeline:

1. **Code Formatting** - Ensures consistent style
2. **Clippy Lints** - Catches common mistakes and warnings
3. **Release Build** - Tests compilation with optimizations
4. **Unit Tests** - Runs all test suites
5. **Architecture Compatibility** - Checks for platform-specific issues
6. **Dependencies** - Validates Cargo.lock is up to date

## Common Issues and Fixes

### Unused Imports
```rust
// Bad
use std::arch::x86_64::*;

// Good
#[cfg(target_arch = "x86_64")]
#[allow(unused_imports)]
use std::arch::x86_64::*;
```

### Array Initialization with Non-Copy Types
```rust
// Bad
let mut buffer = [None; 8];  // Error if type isn't Copy

// Good
let mut buffer = [const { None }; 8];
```

### Platform-Specific Code
Always use conditional compilation:
```rust
#[cfg(target_arch = "x86_64")]
fn x86_specific_function() { ... }

#[cfg(target_arch = "aarch64")]
fn arm_specific_function() { ... }
```

## Bypassing Checks (Emergency Only)

If you absolutely need to push without validation:

```bash
git push --no-verify
```

⚠️ **Warning**: Only use this in emergencies. CI will still run these checks!

## Additional Tools

For more thorough checking, install:

```bash
# Check for unused dependencies
cargo install cargo-machete

# Security audit
cargo install cargo-audit

# More comprehensive linting
cargo install cargo-cranky
```

## CI Pipeline Details

Our GitHub Actions workflow runs on:
- **OS**: Ubuntu (latest), macOS (latest), Windows (latest)
- **Rust**: Stable, Beta
- **Architectures**: x86_64, aarch64 (ARM64)

Always test release builds locally as they catch different errors than debug builds!