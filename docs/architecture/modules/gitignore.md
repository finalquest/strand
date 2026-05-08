# `gitignore`

**Purpose**: Ensures `.gitignore` entries for installed skills and agents.
**Files**: `src/gitignore.rs`

## Public API

**Decision**: Generalize the existing function with an `ArtifactType` parameter rather than adding a separate agent-specific function.

```rust
pub enum ArtifactType {
    Skill,
    Agent,
}

pub fn ensure_gitignore_entries_for(name: &str, artifact_type: ArtifactType) -> Result<()>

// Backward-compatible wrapper (preserves existing behavior)
pub fn ensure_gitignore_entries(skill_name: &str) -> Result<()>
```

## Used By
- `commands::ls_remote`
- `commands::sync`
- `commands::install`
- `commands::agents::helpers` (via `ArtifactType::Agent`)

## Notes
- Appends entries to `.gitignore` if they don't already exist.
- `ArtifactType::Skill` entries: `.agents/skills/{name}/`, `.codex/skills/{name}/`.
- `ArtifactType::Agent` entries: `.agents/agents/{name}/`, `.opencode/agents/{name}/`, `.codex/agents/{name}/`.
- Existing `ensure_gitignore_entries(skill_name: &str)` wrapper remains unchanged to preserve backward compatibility with current callers.
