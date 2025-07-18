# GitHub Actions Workflows

## CI Workflow (ci.yml)
- Runs on pushes and pull requests to the `master` branch
- Tests on multiple platforms (Ubuntu, macOS, Windows) with stable and beta Rust
- Includes nightly testing on Ubuntu (allowed to fail)
- Runs:
  - Code formatting check (`cargo fmt`)
  - Clippy linting (`cargo clippy`)
  - Build verification
  - Test suite
  - Security audit

## Release Workflow (release.yml)
- Triggers when you push a tag in the format `X.Y.Z` (e.g., `1.0.0`, `2.1.3`)
- Creates a GitHub Release
- Builds binaries for:
  - Linux x64 (AMD64)
  - Linux ARM64
  - macOS x64 (Intel)
  - macOS ARM64 (Apple Silicon)
  - Windows x64
- Uploads compressed binaries to the release
- Optionally publishes to crates.io (requires CARGO_TOKEN secret)

## How to Create a Release

1. Update version in `Cargo.toml`
2. Commit changes
3. Create and push a tag:
   ```bash
   git tag 1.0.0
   git push origin 1.0.0
   ```
4. The release workflow will automatically build and publish binaries