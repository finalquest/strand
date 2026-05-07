# `gitignore`

**Purpose**: Ensures `.gitignore` entries for installed skills.
**Files**: `src/gitignore.rs`

## Public API

```rust
pub fn ensure_gitignore_entries(skill_name: &str) -> Result<()>
```

## Used By
- `commands::ls_remote`
- `commands::sync`
- `commands::install`

## Notes
- Appends entries to `.gitignore` if they don't already exist.
- Typical entries: `.agents/skills/{name}/`, `.codex/skills/{name}/`.
