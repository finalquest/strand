# Architectural Decisions

> Why things are built this way. Chronological record of significant choices.

## AD-001: Transport Trait Abstraction

**Context**: strand needs to talk to GitLab API. Two possible approaches: raw HTTP or `glab` CLI.

**Decision**: Define a `Transport` trait with two implementations (`ReqwestTransport` for HTTP, `GlabTransport` for CLI wrapper). `GitLabClient` depends on the trait, not a specific transport.

**Rationale**: Users who already have `glab` authenticated get zero-config auth. Users without `glab` fall back to HTTP with a PAT. The abstraction lets either path work without command-level branching.

**Consequences**: Commands never know which transport is in use. Adding a new backend (e.g., GitHub) means implementing the trait, not changing commands.

## AD-002: Auth Fallback Chain

**Context**: Multiple ways to obtain a GitLab token.

**Decision**: `auth::authenticate()` tries three backends in order: glab CLI → env var → interactive prompt. First success wins.

**Rationale**: Best user experience for each audience. CI/CD users set an env var. Developer laptop users likely have `glab` already. First-time users get a prompt.

**Consequences**: No single auth method is mandatory. The fallback order is hardcoded; if priority needs to change, `auth/mod.rs` is the single place to edit.

## AD-003: Config as Single Source of Truth

**Context**: Skills and agents need persisted state (what's installed, versions, repo config).

**Decision**: `.strand/config.json` is the canonical state. All reads go through `config.rs`. All writes go through `config::add_skill()` / `config::add_agent()`.

**Rationale**: Avoids filesystem-as-state issues (stale files, partial installs). Config explicitly lists what's installed and at what version.

**Consequences**: If `config.json` is deleted, strand loses track of installed skills even if files exist on disk. The `validate` command exists to detect and repair this mismatch.

## AD-004: One Command = One Module

**Context**: Growing number of CLI subcommands.

**Decision**: Each command lives in its own file under `src/commands/` with a single `execute()` function. Agent commands are namespaced under `src/commands/agents/`.

**Rationale**: Easy to find, easy to reason about. Each command is a self-contained flow. Adding a command means adding a file + one line in `cli.rs` + one line in `main.rs`.

**Consequences**: Some duplication between similar commands (e.g., skill `ls` vs agent `ls`). Shared helpers are extracted to `commands/agents/helpers.rs` for agents.

## AD-005: YAML Frontmatter for Skill/Agent Metadata

**Context**: Skills and agents need metadata (name, description, version). Initially used `skill.json`.

**Decision**: Migrated from `skill.json` to `SKILL.md` / `AGENT.md` with YAML frontmatter. The markdown body contains documentation; the frontmatter contains machine-readable metadata.

**Rationale**: Single file serves dual purpose: documentation for humans and metadata for strand. Eliminates the need for a separate JSON file per skill.

**Consequences**: `validate` command detects legacy `skill.json` and offers migration. `fix::NeedsMigration` handles the conversion automatically.

## AD-006: Generic Symlink Utility

**Context**: Both skills and agents need symlinks, but to different target directories.

**Decision**: A single `symlinks::create_symlink(target_dir, link_dir, name)` function with generic parameters. Skill-specific and agent-specific callers pass their own directories.

**Rationale**: Avoids code duplication. The symlink logic is identical regardless of artifact type; only the paths differ.

**Consequences**: `codex.rs` is a thin wrapper that reads config and calls `symlinks`. Agent helpers call `symlinks` directly.

## AD-007: Generic Gitignore with ArtifactType

**Context**: Gitignore entries differ between skills (`.agents/skills/`, `.codex/skills/`) and agents (`.agents/agents/`, `.opencode/agents/`, `.codex/agents/`).

**Decision**: `ArtifactType` enum parameterizes the gitignore function. A backward-compatible wrapper `ensure_gitignore_entries(skill_name)` delegates to the generic version with `ArtifactType::Skill`.

**Rationale**: One function handles both artifact types. Existing callers don't need changes.

**Consequences**: New artifact types would add a new `ArtifactType` variant and the associated directory entries.

## AD-008: Env Vars Override Config

**Context**: Users need to test against different repos or branches without modifying `config.json`.

**Decision**: `env.rs` reads `strand_*` environment variables. These take precedence over `config.json` values.

**Rationale**: CI/CD and testing workflows often need to point at a different repo. Env vars are the standard mechanism for this. Config remains untouched.

**Consequences**: The resolution chain is: env var → config → default. All commands go through the same resolution, so overrides are consistent.

## AD-009: Inline Tests

**Context**: Where to put unit tests.

**Decision**: Every module has a `#[cfg(test)]` block at the bottom of the file. Integration tests live in `tests/`.

**Rationale**: Tests are closest to the code they test. No hunting through a separate test directory tree. Rust convention.

**Consequences**: Module files can get long. The `tests/` directory is reserved for end-to-end tests that exercise the compiled binary.

## AD-010: Resolver as Unused Abstraction

**Context**: A `resolver` module exists that abstracts over local and remote skill sources via a `SkillSource` trait.

**Decision**: Keep it but acknowledge it's unused. Commands call `GitLabClient` directly.

**Rationale**: The resolver was built as a potential future abstraction for unified local/remote access. It's not yet needed because all commands deal with remote sources. Removing it would be premature; keeping it documents the possibility.

**Consequences**: New commands should use `GitLabClient` directly for now. If local+remote unification becomes necessary, the resolver pattern is ready.
