# strand

CLI tool for managing GitLab skills repositories.

## Installation

### From source

```bash
git clone <repo-url>
cd strand
cargo build --release
```

The binary will be available at `./target/release/strand`.

### Using Docker

```bash
docker build -t strand .
docker run --rm strand --help
```

## Authentication

`strand` supports three authentication methods (tried in order):

1. **glab CLI** - Uses your existing `glab auth status` token
2. **Environment variable** - Set `strand_GITLAB_TOKEN`
3. **Interactive prompt** - Paste a Personal Access Token when prompted

## Quick Start

```bash
# Initialize a new strand project
strand init

# List available skills
strand ls-remote

# Sync installed skills with remote
strand sync

# Install/reinstall skills from config
strand install
```

## Commands

### `init`

Initialize a new strand project in the current directory.

```bash
strand init
```

Creates:
- `.strand/config.json` - Project configuration
- `.agents/skills/` - Local skills directory
- `.codex/skills/` - Codex skills directory (if enabled)

### `ls-remote`

List available skills from the configured repository and optionally install one interactively.

```bash
strand ls-remote
```

### `sync`

Compare installed skill versions with the remote repository and optionally upgrade.

```bash
strand sync
```

### `install`

Ensure installed skills match the versions pinned in `.strand/config.json`.

```bash
strand install        # Apply changes
strand install --dry-run  # Preview changes
```

## Configuration

After running `strand init`, edit `.strand/config.json`:

```json
{
  "version": 1,
  "targets": {
    "opencode": true,
    "codex": false
  },
  "skillsRepo": {
    "provider": "gitlab",
    "project": "your-group/skills-repo",
    "branch": "main"
  },
  "skills": []
}
```

### Environment Variables

- `strand_GITLAB_TOKEN` - GitLab Personal Access Token
- `strand_SKILLS_REPO` - Override skills repository path
- `strand_SKILLS_REPO_PATH` - Use local skills directory

## Development

```bash
# Run tests
cargo test

# Build release binary
cargo build --release

# Run with cargo
cargo run -- <command>
```

## License

[Specify your license]
