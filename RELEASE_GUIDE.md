# 🚀 Fastest Release Guide

This guide walks you through releasing a new version of Fastest.

## Prerequisites

1. **GitHub Repository Setup**:
   - Ensure you have push access to the main branch
   - Set up a PyPI API token as `PYPI_API_TOKEN` in GitHub Secrets
   - (Optional) Set up a crates.io token as `CRATES_TOKEN` in GitHub Secrets

## Step-by-Step Release Process

### 1. Prepare the Release

```bash
# First, ensure all changes are committed and pushed
git status
git add .
git commit -m "feat: your feature description"
git push origin main

# Run tests to ensure everything works
make test
make bench  # Optional: verify performance
```

### 2. Update the Version

```bash
# Use the version bump script (example: going from 0.2.0 to 0.2.1)
./scripts/bump-version.sh 0.2.1

# This updates version in:
# - Cargo.toml
# - pyproject.toml  
# - setup.py
# - Other relevant files
```

### 3. Update the Changelog

Edit `CHANGELOG.md` to add your release notes:

```markdown
## [0.2.1] - 2025-05-28

### Added
- New feature X
- Enhancement Y

### Fixed
- Bug fix Z

### Changed
- Improved performance of A
```

### 4. Commit Version Changes

```bash
# Commit the version bump
git add .
git commit -m "chore: bump version to 0.2.1"
git push origin main
```

### 5. Create and Push the Release Tag

```bash
# Create an annotated tag
git tag -a v0.2.1 -m "Release v0.2.1 - Brief description"

# Push the tag to trigger the release workflow
git push origin v0.2.1
```

### 6. Monitor the Release

1. Go to https://github.com/derens99/fastest/actions
2. Watch the "Release" workflow run
3. It will automatically:
   - Build binaries for all platforms
   - Create a GitHub release with changelog
   - Upload binaries with checksums
   - Publish to PyPI
   - Update the version manifest

### 7. Verify the Release

Once complete, verify:

```bash
# Check GitHub release page
# https://github.com/derens99/fastest/releases

# Test binary installation
curl -L https://raw.githubusercontent.com/derens99/fastest/main/install.sh | bash

# Test PyPI installation
pip install --upgrade fastest-runner

# Test the update command
fastest update --check
```

## Release Workflow Details

The GitHub Actions workflow (`.github/workflows/release.yml`) handles:

1. **Multi-Platform Builds**:
   - Linux (x64, ARM64)
   - macOS (x64, ARM64)
   - Windows (x64)

2. **Distribution Channels**:
   - GitHub Releases (binaries)
   - PyPI (Python wheels)
   - Crates.io (Rust crates)

3. **Security**:
   - SHA256 checksums for all binaries
   - Signed releases (if configured)

4. **Automation**:
   - Changelog generation
   - Version manifest update
   - Cross-platform wheel building

## Troubleshooting

### Release workflow fails

1. Check GitHub Actions logs for specific errors
2. Common issues:
   - Missing PyPI token
   - Version already exists on PyPI
   - Build failures on specific platforms

### Update command not finding new version

- The version manifest updates after all builds complete
- Wait a few minutes after release completes
- Check `.github/version.json` is updated in the repo

### Platform-specific build issues

- Linux ARM builds use cross-compilation
- Windows builds require MSVC
- macOS builds require both x64 and ARM64 targets

## Manual Release (Emergency)

If automation fails, you can manually:

```bash
# Build locally
cargo build --release --target x86_64-unknown-linux-gnu

# Create GitHub release manually
# Upload binaries through GitHub UI

# Publish to PyPI manually
python -m build
twine upload dist/*
```

## Version Numbering

We follow semantic versioning:
- **MAJOR.MINOR.PATCH** (e.g., 0.2.1)
- **MAJOR**: Breaking changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, performance improvements

## Post-Release

After a successful release:

1. Announce on social media
2. Update documentation if needed
3. Close related GitHub issues
4. Plan next release features

---

**Remember**: The release process is automated, but always verify the release worked correctly!