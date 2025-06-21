# Release Process

This document outlines the release process for Fastest.

## Pre-release Checklist

- [ ] All tests passing on main branch
- [ ] CHANGELOG.md updated with new features/fixes
- [ ] Documentation updated
- [ ] Version bumped in all Cargo.toml files
- [ ] Performance benchmarks run and compared

## Version Bumping

Update version in the following files:
- `Cargo.toml` (workspace version)
- `crates/fastest-core/Cargo.toml`
- `crates/fastest-cli/Cargo.toml`
- `crates/fastest-python/Cargo.toml` (if exists)

```bash
# Example: bump to version 0.2.0
sed -i '' 's/version = ".*"/version = "0.2.0"/' Cargo.toml
sed -i '' 's/version.workspace = true/version = "0.2.0"/' crates/*/Cargo.toml
```

## Release Steps

1. **Create Release Branch**
   ```bash
   git checkout -b release/v0.2.0
   ```

2. **Update CHANGELOG.md**
   - Add release date
   - Organize changes by category (Added, Changed, Fixed, etc.)
   - Add comparison link at bottom

3. **Commit Changes**
   ```bash
   git add -A
   git commit -m "chore: prepare v0.2.0 release"
   ```

4. **Create PR and Merge**
   - Create PR from release branch to main
   - Ensure all CI checks pass
   - Get approval and merge

5. **Tag Release**
   ```bash
   git checkout main
   git pull origin main
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```

6. **Monitor Release Workflow**
   - Check GitHub Actions for release workflow
   - Ensure all artifacts are built
   - Verify crates.io publication
   - Check Docker Hub for new images

## Post-release Tasks

- [ ] Verify installation methods work:
  - [ ] Cargo install
  - [ ] Binary downloads
  - [ ] Docker image
  - [ ] Homebrew (after formula updates)
- [ ] Update documentation site
- [ ] Announce release:
  - [ ] GitHub Release notes
  - [ ] Twitter/Social media
  - [ ] Blog post (for major releases)
- [ ] Update fastest.dev website (if applicable)

## Platform-specific Releases

### Crates.io

The release workflow automatically publishes to crates.io. Ensure:
- API token is set in GitHub secrets
- Dependencies are published first (fastest-core before fastest-cli)

### Docker Hub

Docker images are automatically built and pushed with tags:
- `latest` - always points to newest release
- `0.2.0` - specific version tag

### Homebrew

The workflow creates a PR to the homebrew tap. Manual steps:
1. Review the automated PR
2. Test installation: `brew install --build-from-source ./Formula/fastest.rb`
3. Merge PR

## Rollback Procedure

If issues are found after release:

1. **Delete the problematic release tag**
   ```bash
   git tag -d v0.2.0
   git push origin :refs/tags/v0.2.0
   ```

2. **Yank from crates.io** (if published)
   ```bash
   cargo yank --vers 0.2.0 fastest-cli
   cargo yank --vers 0.2.0 fastest-core
   ```

3. **Fix issues and re-release**
   - Use a new patch version (e.g., v0.2.1)
   - Never reuse version numbers

## Security Releases

For security fixes:

1. Follow responsible disclosure practices
2. Prepare fix in private
3. Release with clear security advisory
4. Backport to supported versions if needed

## Version Support Policy

- Latest minor version: Full support
- Previous minor version: Security fixes only
- Older versions: No support

Example:
- v0.3.x - Full support
- v0.2.x - Security fixes only
- v0.1.x - No support 