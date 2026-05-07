# Handoff

## Objective
Implement board tasks in cycle: Ready → Doing → Review → Done.

## Relevant Files/Symbols

### T-008: `strand ls` command (local skill listing)
- `src/commands/ls.rs` — new: `ls()` command that lists installed skills
- `src/commands/mod.rs` — exports `ls` module
- `src/cli.rs` — added `Ls` subcommand variant
- `src/main.rs` — dispatch for `Commands::Ls`
- Shows table with: skill name, local version, remote version
- Local version highlighted in red when different from remote

### T-009: Fuzzy install table
- `src/commands/install.rs` — updated: interactive mode uses `dialoguer::FuzzySelect` with table
- Table columns: Skill, Version, Description
- Non-interactive mode: renders ASCII table and exits gracefully
- Matches `ls-remote` pattern with added version display

### T-010: Loading indicators
- `src/commands/ls_remote.rs` — added spinner while fetching skills
- `src/commands/sync.rs` — added spinners for status check and upgrade operations
- `src/commands/install.rs` — added spinners for fetching and installing skills
- Uses `indicatif::ProgressBar` with spinner style

### T-011: Cambiar formato de versioning (DONE)
- Changed versioning source from `skill.json` to YAML frontmatter in `SKILL.md`
- New format: `metadata.version` in SKILL.md frontmatter
- Removed entrypoint field (SKILL.md is always the entrypoint)
- Added `serde_yaml` dependency
- Migration path: validate detects skills with `skill.json` but no metadata in `SKILL.md` and offers auto-migration
- All commands updated: install, sync, update, ls, ls-remote, validate, fix

## Decisions Made
- T-008 table uses box-drawing characters for visual clarity
- T-009 reuses `ls-remote` fuzzy selection pattern, extending with version column
- Red color for version mismatch follows existing CLI color conventions
- Both commands respect interactive vs non-interactive modes
- T-010 spinners show during network requests and are cleared before rendering output
- T-011 uses YAML frontmatter in SKILL.md for versioning, removing skill.json dependency
- Config (SkillEntry) unchanged - still has name + version + installedPath

## Current State
- T-008: Done ✓
- T-009: Done ✓
- T-010: Done ✓
- T-011: Done ✓
- Board: no cards in Ready, Doing, Blocked, or Review
- Build: `cargo build` passes
- Tests: `cargo test` passes (87 tests)
- Documentation: Updated with new commands and AGENTS.md rule

## Blockers/Risks
- No remaining blockers
