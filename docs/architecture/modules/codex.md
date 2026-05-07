# `codex`

**Purpose**: Codex integration. Creates symlinks in `.codex/skills/`.
**Files**: `src/codex.rs`

## Public API

```rust
pub fn create_symlink(skill_name: &str) -> Result<()>
```

## Used By
- `commands::ls_remote`
- `commands::sync`
- `commands::install`

## Dependencies
- `config::Config`
- `config::CONFIG_PATH`

## Notes
- Only creates symlinks if `config.targets.codex` is `true`.
- Symlinks from `.agents/skills/{name}` to `.codex/skills/{name}`.
