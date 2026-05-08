# `models`

**Purpose**: Data models shared across the system.
**Files**: `src/models/mod.rs`, `src/models/skill.rs`, `src/models/agent.rs`

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

pub struct Agent {
    pub name: String,
    pub description: String,
    pub version: String,
}

pub struct AgentFrontmatter {
    pub name: String,
    pub description: String,
    pub metadata: AgentMetadata,
}

pub struct AgentMetadata {
    pub version: String,
}
```

## Used By
- `commands::list`, `commands::sync`, `commands::install`, `commands::ls`, `commands::ls_remote` (Skill)
- `commands::validate`, `fix` (SkillFrontmatter)
- `commands::agents::{ls,ls_remote,validate,helpers}` (Agent, AgentFrontmatter)
- `config` (Skill, Agent)
- `download` (Skill)

## Notes
- `Skill` is the runtime representation used by commands.
- `SkillFrontmatter` is the on-disk schema for `SKILL.md` YAML frontmatter.
- `Agent` mirrors `Skill` for the agents namespace.
- `AgentFrontmatter` is the on-disk schema for `AGENT.md` YAML frontmatter.
- Changing fields requires updating serde deserialization and all consumers.
