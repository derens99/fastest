# Semantic Versioning Guide for Fastest

This project uses **semantic versioning** with **conventional commits** to automatically manage version numbers and releases.

## Overview

When you push commits to the `main` branch, the CI system will:
1. Analyze commit messages to determine the version bump (major/minor/patch)
2. Update version numbers in all project files
3. Generate a changelog
4. Create a git tag and GitHub release
5. Build and publish binaries for all platforms
6. Publish to PyPI and crates.io

## Commit Message Format

Use conventional commit messages to trigger automatic versioning:

### Version Bumps

| Commit Type | Version Bump | Example |
|-------------|--------------|---------|
| `feat:` | Minor (0.x.0) | `feat: add work-stealing executor` |
| `fix:` | Patch (0.0.x) | `fix: correct test discovery in subdirectories` |
| `perf:` | Patch (0.0.x) | `perf: optimize SIMD JSON parsing` |
| `BREAKING CHANGE:` | Major (x.0.0) | `feat!: change CLI argument format` |

### Non-versioning Commits

These commits won't trigger a release:
- `chore:` - Maintenance tasks
- `docs:` - Documentation only
- `style:` - Code style changes
- `refactor:` - Code refactoring
- `test:` - Test additions/changes
- `build:` - Build system changes
- `ci:` - CI configuration changes

### Examples

```bash
# Minor version bump (0.2.0 → 0.3.0)
git commit -m "feat: add pytest-xdist compatibility"

# Patch version bump (0.2.0 → 0.2.1)
git commit -m "fix: handle unicode in test names"

# Major version bump (0.2.0 → 1.0.0)
git commit -m "feat!: remove deprecated API endpoints"
# or
git commit -m "feat: new test runner

BREAKING CHANGE: removed support for Python 3.7"

# No version bump
git commit -m "chore: update dependencies"
git commit -m "docs: improve installation guide"
```

## Multiple Commits

When multiple commits are pushed together, the highest version bump wins:
- `feat:` + `fix:` = Minor bump
- `fix:` + `fix:` = Single patch bump
- `feat!:` + anything = Major bump

## Skipping CI

To push commits without triggering the release workflow:
```bash
git commit -m "chore: update docs [skip ci]"
```

## Manual Release

If needed, you can still trigger a manual release:
```bash
# Create and push a tag
git tag -a v0.3.0 -m "Release v0.3.0"
git push origin v0.3.0
```

## Workflow Files

- `.github/workflows/semantic-release.yml` - Analyzes commits and creates releases
- `.github/workflows/release.yml` - Builds and publishes binaries
- `.releaserc.json` - Semantic release configuration
- `scripts/semantic-bump-version.sh` - Updates version in all files

## Version Files Updated

The following files are automatically updated with new versions:
- `Cargo.toml` (workspace root)
- `crates/*/Cargo.toml` (all crate manifests)
- `Cargo.lock`
- `pyproject.toml`
- `setup.py`
- `CHANGELOG.md`

## Changelog

The changelog is automatically generated and includes:
- ✨ Features
- 🐛 Bug Fixes
- ⚡ Performance Improvements
- ⏪ Reverts
- 📚 Documentation (if significant)
- ♻️ Code Refactoring (if significant)

## Best Practices

1. **Write clear commit messages** that describe what changed and why
2. **Use the correct commit type** to ensure proper versioning
3. **Group related changes** in a single commit when possible
4. **Mark breaking changes** clearly with `!` or `BREAKING CHANGE:`
5. **Review the changelog** after each release to ensure accuracy

## Troubleshooting

### Release not triggered
- Check that commits follow conventional format
- Ensure you're pushing to the `main` branch
- Verify GitHub Actions are enabled

### Version not updated in a file
- Check `scripts/semantic-bump-version.sh` includes the file
- Ensure the file uses standard version format

### Build failures
- The release workflow will create the tag even if builds fail
- Fix the issue and re-run the failed jobs

## Local Testing

To test version bumping locally:
```bash
# Install dependencies
npm install

# Dry run (won't create commits/tags)
npx semantic-release --dry-run
```