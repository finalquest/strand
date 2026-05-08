# `symlinks`

**Purpose**: Generic symlink creation utility for both skills and agents.
**Files**: `src/symlinks.rs`

## Public API

```rust
pub fn create_symlink(target_dir: &str, link_dir: &str, name: &str) -> Result<()>
```

## Used By
- `codex` (skill symlinks to `.codex/skills/`)
- `commands::agents::helpers` (agent symlinks to `.opencode/agents/` and `.codex/agents/`)
- `commands::init` (target directory setup)

## Dependencies
- None

## Notes
- Platform-aware: uses `std::os::unix::fs::symlink` on Unix, `std::os::windows::fs::symlink_dir` on Windows.
- Idempotent: removes existing symlinks before creating new ones.
- Creates parent directories if they don't exist.
- Generic parameters allow reuse for any target/link combination.

## Examples

```rust
// Skill symlink
create_symlink(".agents/skills/my-skill", ".codex/skills", "my-skill")?;

// Agent symlinks
create_symlink(".agents/agents/my-agent", ".opencode/agents", "my-agent")?;
create_symlink(".agents/agents/my-agent", ".codex/agents", "my-agent")?;
```
