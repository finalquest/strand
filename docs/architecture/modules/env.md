# `env`

**Purpose**: Environment variable configuration for repository overrides.
**Files**: `src/env.rs`

## Public API

```rust
pub fn agents_repo_project() -> Option<String>
pub fn agents_repo_branch() -> String
pub fn skills_repo_project() -> Option<String>
pub fn skills_repo_branch() -> String
```

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `strand_AGENTS_REPO` | Overrides `agentsRepo.project` from config | None |
| `strand_AGENTS_REPO_BRANCH` | Overrides `agentsRepo.branch` from config | `main` |
| `strand_SKILLS_REPO` | Overrides `skillsRepo.project` from config | None |
| `strand_SKILLS_REPO_BRANCH` | Overrides `skillsRepo.branch` from config | `main` |
| `strand_GITLAB_URL` | Overrides the base URL for both repos | `https://gitlab.com` |

## Used By
- `config` (`resolve_agents_repo()`)
- `commands::agents::ls` (repo resolution)
- `commands::agents::ls_remote` (repo resolution)

## Dependencies
- None

## Notes
- Environment variables take precedence over config file values.
- The `*_BRANCH` variables can be set independently (without setting `*_REPO`).
- All reads are wrapped in `ENV_MUTEX` during tests to prevent race conditions.
