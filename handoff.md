# Handoff

## Objective
Distribute strand CLI via GitHub Releases with public installer.

## Relevant Files/Symbols

### Distribution / Release Pipeline
- `scripts/build-release.sh` — builds macOS arm64 + Linux x86_64 binaries, creates GitHub release via `gh release create`
- `scripts/Dockerfile.linux` — Docker build for Linux x86_64 cross-compilation
- `install.sh` — public installer: `curl` downloads latest binary from GitHub Releases, installs to `~/.local/bin/strand`
- `Cargo.toml` — version `0.0.1`

### CLI
- `src/cli.rs` — added `#[command(version)]` for `--version` / `-V` flag
- `src/main.rs` — command dispatch

### Tests (de-Poincenotized)
- `tests/e2e/main.rs` — `DEFAULT_BASE_URL = "https://gitlab.example.com"`, `DEFAULT_PROJECT = "example-group/sandbox/dev-skills"`
- `tests/e2e/README.md` — updated target URLs
- `tests/e2e/cli_ls_remote.rs` — updated env vars

### Specs (de-Poincenotized)
- `specs/auth_refactor_glab_direct.md` — all `gitlab.poincenot.net` references replaced with `gitlab.example.com`

## Decisions Made
- Migrated repo from private GitLab (`gitlab.poincenot.net/ai-ideas/tools/apps/strand`) to public GitHub (`github.com/finalquest/strand`)
- Entire git history was replaced with a single clean commit (orphan branch + force push) to remove all internal references from history
- GitLab Release distribution model abandoned (`glab release download` fails with 404 on private releases due to auth redirect bug)
- GitHub Releases model adopted because public releases allow unauthenticated `curl` downloads
- Local git config set to GitHub identity: `Fernando Basello <ff8leonheart@gmail.com>` (not global)
- No auth required for end-user installation

## Current State
- Repo public: `https://github.com/finalquest/strand`
- Git history: single commit (`e35e837`), no prior history accessible
- GitHub release: **v0.0.1** published with both binaries (`strand-macos-arm64`, `strand-linux-x86_64`)
- Install command:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/finalquest/strand/main/install.sh | bash
  ```
- No references to `poincenot`, `pcnt`, `gitlab.poincenot.net`, or `ai-ideas` anywhere in codebase or history
- `cargo build` passes
- `strand --version` works (outputs `strand 0.0.1`)

## Blockers/Risks
- None
