# `download`

**Purpose**: Recursively downloads a skill directory from GitLab to local `.agents/skills/`.
**Files**: `src/download.rs`

## Public API

```rust
pub fn download_and_install(client: &GitLabClient, skill: &Skill) -> Result<()>
```

## Used By
- `commands::ls_remote`
- `commands::sync`
- `commands::install`

## Dependencies
- `gitlab::GitLabClient`
- `models::skill::Skill`

## Notes
- Recursively fetches all files under `skills/{name}/` from GitLab.
- Writes to `.agents/skills/{name}/` locally.
