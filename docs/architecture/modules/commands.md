# `commands`

**Purpose**: Implementation of each CLI subcommand.
**Files**: `src/commands/mod.rs`, `src/commands/{init,install,ls,ls_remote,sync,validate}.rs`, `src/commands/agents/{mod,ls,ls_remote,validate,helpers}.rs`

## Public API

| Module | Entrypoint | Description |
|--------|-----------|-------------|
| `commands::init` | `pub fn init() -> Result<()>` | Creates dirs + config file |
| `commands::list` | `pub fn execute() -> Result<()>` | Lists remote skills |
| `commands::ls_remote` | `pub fn execute() -> Result<()>` | Lists remote skills with fuzzy select |
| `commands::ls` | `pub fn execute() -> Result<()>` | Lists installed skills with version comparison |
| `commands::sync` | `pub fn execute() -> Result<()>` | Check for updates, optionally upgrade |
| `commands::install` | `pub fn execute(InstallOptions) -> Result<()>` | Reinstall skills pinned in config |
| `commands::validate`  | `pub fn execute() -> Result<()>` | Validate local skills |
| `commands::agents::ls` | `pub fn execute() -> Result<()>` | Lists installed agents with version comparison |
| `commands::agents::ls_remote` | `pub fn execute() -> Result<()>` | Lists remote agents with fuzzy select |
| `commands::agents::validate` | `pub fn execute() -> Result<()>` | Validate local agents |

## Common Pattern

All commands follow this pattern:
1. `resolve_repo_config()` → get `(project, base_url)`
2. `GitLabClient::for_project(base_url, project)` → create client
3. Call GitLab API (`list_tree`, `fetch_file`)
4. Process results
5. Side effects (`config::add_skill`, `gitignore`, `codex` symlink)

## Dependencies per Command

```
commands::init      → config
commands::list      → config, gitlab::client, models::skill
commands::ls_remote → codex, config, download, gitignore, gitlab::client, models::skill
commands::ls        → config, gitlab::client, models::skill
commands::sync      → codex, config, download, gitignore, gitlab::client, models::skill, version
commands::install   → codex, config, download, gitignore, gitlab::client, models::skill
commands::validate  → fix, models::skill, report
commands::agents::ls       → config, gitlab::client, models::agent
commands::agents::ls_remote → config, gitlab::client, models::agent
commands::agents::validate → fix, models::agent, report
commands::agents::helpers  → config, download, gitignore, gitlab::client, models::agent, symlinks
```

## Skill Format

Commands that interact with skills now use `SKILL.md` with YAML frontmatter:

```yaml
---
name: skill-name
description: "Skill description"
metadata:
  version: "1.0.0"
---
```

The old `skill.json` format is no longer used. During migration, `validate` detects legacy `skill.json` files and offers to convert them.

## Agent Format

Commands that interact with agents use `AGENT.md` with YAML frontmatter:

```yaml
---
name: agent-name
description: "Agent description"
metadata:
  version: "1.0.0"
---
```

## Adding a New Command

1. Add variant to `Commands` enum in `src/cli.rs`
2. Create `src/commands/<name>.rs` with `pub fn execute() -> Result<()>`
3. Add `pub mod <name>;` to `src/commands/mod.rs`
4. Wire dispatch in `src/main.rs`
