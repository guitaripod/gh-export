# gh-export

A fast and reliable GitHub repository exporter written in Rust. Export all repositories from a GitHub user account with a single command.

## Features

- **Fast parallel downloads** - Clone multiple repositories concurrently
- **Smart sync** - Update existing clones instead of re-downloading
- **Progress tracking** - Real-time progress bars for all operations
- **Secure token storage** - Tokens stored with proper file permissions
- **Flexible filtering** - Include/exclude archived repos, forks, and more
- **Resume capability** - Handles interruptions gracefully
- **Rate limit handling** - Respects GitHub's API limits

## Installation

### From source

```bash
git clone https://github.com/guitaripod/gh-export
cd gh-export
cargo install --path .
```

### From crates.io

```bash
cargo install gh-export
```

## Quick Start

1. **First run** - Interactive setup:
```bash
gh-export
```

The tool will guide you through:
- Creating a GitHub personal access token
- Configuring the output directory
- Starting the export

2. **Subsequent runs** - Uses saved configuration:
```bash
gh-export
```

## Usage

### Basic Commands

```bash
# Export all repositories (interactive setup on first run)
gh-export

# Export with explicit token
gh-export --token YOUR_GITHUB_TOKEN

# Export to specific directory
gh-export --output /path/to/backup

# Sync existing repositories
gh-export sync

# Show last export status
gh-export status
```

### Configuration Management

```bash
# Show current configuration
gh-export config show

# Set configuration values
gh-export config set token YOUR_TOKEN
gh-export config set output /new/path
gh-export config set parallel 6

# Clear configuration
gh-export config clear
```

### Advanced Options

```bash
# Include archived repositories
gh-export --include-archived

# Exclude forked repositories
gh-export --exclude-forks

# Shallow clone (faster, no history)
gh-export --shallow

# Adjust parallel downloads (default: 4)
gh-export --parallel 8

# Filter repositories by name
gh-export --filter "rust"

# Quiet mode
gh-export --quiet

# Verbose logging
gh-export --verbose
```

## GitHub Token

You'll need a GitHub personal access token with appropriate permissions:

1. Go to [GitHub Settings → Tokens](https://github.com/settings/tokens)
2. Click "Generate new token" → "Generate new token (classic)"
3. Give it a descriptive name (e.g., "gh-export")
4. Select scopes:
   - `repo` - Full control of private repositories (includes public)
   - `public_repo` - Access to public repositories only
5. Click "Generate token" and copy it

## Directory Structure

Repositories are organized by username:

```
output_directory/
├── username/
│   ├── repo1/
│   ├── repo2/
│   └── .gh-export-metadata.json
```

## Configuration

Configuration is stored in:
- Linux/macOS: `~/.config/gh-export/config.toml`
- Windows: `%APPDATA%\gh-export\config.toml`

Example configuration:

```toml
github_token = "ghp_..."
output_directory = "/home/user/github-backup"
parallel_downloads = 4
include_archived = false
exclude_forks = false
shallow_clone = false
```

## Environment Variables

- `GITHUB_TOKEN` - GitHub personal access token (overrides config file)

## Building from Source

Requirements:
- Rust 1.70 or later
- Git

```bash
git clone https://github.com/guitaripod/gh-export
cd gh-export
cargo build --release
```

The binary will be in `target/release/gh-export`.

## License

MIT License - see LICENSE file for details.
