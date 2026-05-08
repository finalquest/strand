# Module Index

> Atomized module documentation. Each file covers one module.

| Module | Purpose | Key File |
|--------|---------|----------|
| [auth](auth.md) | Authentication backends and fallback chain | `src/auth/mod.rs` |
| [cli](cli.md) | CLI argument parsing | `src/cli.rs` |
| [commands](commands.md) | CLI subcommand implementations | `src/commands/` |
| [config](config.md) | Config schema and persistence | `src/config.rs` |
| [gitlab](gitlab.md) | GitLab API client and transport | `src/gitlab/` |
| [models](models.md) | Data models | `src/models/` |
| [resolver](resolver.md) | Skill source resolution adapter | `src/resolver/` |
| [interactive](interactive.md) | Interactive UI helpers | `src/interactive/` |
| [download](download.md) | Recursive skill download | `src/download.rs` |
| [fix](fix.md) | Auto-fixable issue detection | `src/fix.rs` |
| [report](report.md) | Validation report formatting | `src/report.rs` |
| [version](version.md) | Semver comparison | `src/version.rs` |
| [codex](codex.md) | Codex integration | `src/codex.rs` |
| [symlinks](symlinks.md) | Generic symlink utility | `src/symlinks.rs` |
| [env](env.md) | Environment variables | `src/env.rs` |
| [gitignore](gitignore.md) | Gitignore management | `src/gitignore.rs` |

## Dependency Graph

```
main.rs
  → cli
  → commands::*

cli
  → (none)

commands::init      → config
commands::list      → config, gitlab::client, models::skill
commands::ls_remote → config, gitlab::client, models::skill
commands::ls        → config, gitlab::client, models::skill
commands::ls_remote → codex, config, download, gitignore, gitlab::client, models::skill
commands::sync      → codex, config, download, gitignore, gitlab::client, models::skill, version
commands::install   → codex, config, download, gitignore, gitlab::client, models::skill
commands::validate  → fix, models::skill, report
commands::agents::ls       → config, gitlab::client, models::agent
commands::agents::ls_remote → config, gitlab::client, models::agent
commands::agents::validate → fix, models::agent, report
commands::agents::helpers  → config, download, gitignore, gitlab::client, models::agent, symlinks

gitlab::client
  → auth, gitlab::errors, gitlab::transport

gitlab::transport
  → auth::Auth, gitlab::errors

auth::mod
  → auth::glab, auth::interactive, auth::pat

download
  → gitlab::client, models::skill

codex
  → config, symlinks

symlinks
  → (none)

env
  → (none)

config
  → models::skill, models::agent

resolver::mod
  → resolver::gitlab, resolver::local, resolver::errors

resolver::gitlab
  → gitlab::client, resolver::errors
```
