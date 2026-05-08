# Strand Architecture

> AI-first documentation. Read this first before touching any code.

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI Layer                             │
│  main.rs → cli.rs → commands::{init,ls,ls_remote,sync,...}   │
│                    → commands::agents::{ls,ls_remote,validate} │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Business Logic                           │
│  config.rs · download.rs · fix.rs · report.rs · version.rs  │
│  interactive/ · gitignore.rs · codex.rs · symlinks.rs · env.rs │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      GitLab Client                           │
│         gitlab::client → gitlab::transport                   │
│              ↓                      ↓                        │
│    gitlab::ReqwestTransport    gitlab::GlabTransport         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                       Auth Layer                             │
│   auth::authenticate(hostname) → AuthBackend enum            │
│   GlabAuth · PatAuth · InteractiveAuth                      │
└─────────────────────────────────────────────────────────────┘
```

## Entrypoints

| Command | File | Function | Description |
|---------|------|----------|-------------|
| `init` | `src/commands/init.rs` | `init()` | Creates `.strand/`, `.agents/skills/`, writes config |
| `list` | `src/commands/list.rs` | `execute()` | Lists available skills from remote repo |
| `ls-remote` | `src/commands/ls_remote.rs` | `execute()` | Lists remote skills with fuzzy select (T-009) |
| `ls` | `src/commands/ls.rs` | `execute()` | Lists installed skills with version comparison (T-008) |
| `ls-remote` | `src/commands/ls_remote.rs` | `execute()` | Lists remote skills with fuzzy select and install option (T-012) |
| `sync` | `src/commands/sync.rs` | `execute()` | Checks for updates, optionally upgrades |
| `install` | `src/commands/install.rs` | `execute(opts)` | Reinstalls skills pinned in config |
| `validate` | `src/commands/validate.rs` | `execute()` | Validates local `skills/` directory |
| `agents ls` | `src/commands/agents/ls.rs` | `execute()` | Lists installed agents with version comparison |
| `agents ls-remote` | `src/commands/agents/ls_remote.rs` | `execute()` | Lists remote agents with fuzzy select |
| `agents validate` | `src/commands/agents/validate.rs` | `execute()` | Validates local `agents/` directory |

## Project Conventions

- **One command = one module** in `src/commands/`. Each exposes a single `execute()` function.
- **Error handling**: Subsystems use `thiserror` enums; commands convert to `anyhow::Result` at the boundary.
- **Tests are inline**: Every module has a `#[cfg(test)]` block with mocks. No separate `tests/` for unit tests (integration tests live in `tests/`).
- **Config is single source of truth**: `.strand/config.json` is read/written by `config.rs`. Post-install hooks keep filesystem in sync.
- **Transport abstraction**: All GitLab API calls go through the `Transport` trait. Two implementations: `ReqwestTransport` (HTTP) and `GlabTransport` (CLI).
- **Auth fallback chain**: `auth::authenticate()` tries glab → env PAT → interactive prompt.

## Quick Navigation

| Want to... | Go to |
|------------|-------|
| Add a new CLI command | `src/cli.rs` (add variant) → `src/commands/` (new module) → `src/main.rs` (wire) |
| Change how auth works | `src/auth/mod.rs` (fallback chain) → `src/auth/{glab,pat,interactive}.rs` |
| Change GitLab API calls | `src/gitlab/client.rs` (endpoints) → `src/gitlab/transport.rs` (how they execute) |
| Change config schema | `src/config.rs` (struct) → `src/commands/init.rs` (creation) → all commands (consumption) |
| Change validation/reporting | `src/commands/validate.rs` (flow) → `src/fix.rs` (auto-fix) → `src/report.rs` (output) |
| Add a new transport backend | `src/gitlab/transport.rs` (implement `Transport` trait) → `src/gitlab/client.rs` (wire in factory) |
| Change skill model | `src/models/skill.rs` (struct) → `src/config.rs` (persistence) → commands (usage) |
| Change agent model | `src/models/agent.rs` (struct) → `src/config.rs` (persistence) → commands::agents (usage) |
| Change symlink logic | `src/symlinks.rs` (generic) → `src/codex.rs` (skills) / `src/commands/agents/helpers.rs` (agents) |
| Change env vars | `src/env.rs` (definitions) → `src/config.rs` (resolution) |

## Module Catalog

See [modules/index.md](modules/index.md) for the atomized catalog. Each module has its own file with API, dependencies, and notes.

## Data Flows

See [data-flow.md](data-flow.md) for how config, auth, and commands flow through the system.

## Architectural Decisions

See [decisions.md](decisions.md) for why things are built this way.
