# `models`

**Purpose**: Data models shared across the system.
**Files**: `src/models/mod.rs`, `src/models/skill.rs`

## Public API

```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
}

pub struct SkillFrontmatter {
    pub name: String,
    pub description: String,
    pub metadata: SkillMetadata,
}

pub struct SkillMetadata {
    pub version: String,
}
```

## Used By
- `commands::list`, `commands::sync`, `commands::install`, `commands::ls`, `commands::ls_remote` (Skill)
- `commands::validate`, `fix` (SkillFrontmatter)
- `config` (Skill)
- `download` (Skill)

## Notes
- `Skill` is the runtime representation used by commands.
- `SkillFrontmatter` is the on-disk schema for `SKILL.md` YAML frontmatter.
- Changing fields requires updating serde deserialization and all consumers.
