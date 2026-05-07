# `config`

**Purpose**: Config schema and persistence. Single source of truth for `.strand/config.json`.
**Files**: `src/config.rs`

## Public API

```rust
pub const CONFIG_PATH: &str = ".strand/config.json";

pub struct Config {
    pub version: i32,
    pub targets: TargetConfig,
    pub skills_repo: SkillsRepoConfig,
    pub skills: Vec<SkillEntry>,
}

pub struct TargetConfig { pub opencode: bool, pub codex: bool }
pub struct SkillsRepoConfig { pub provider: String, pub project: String }
pub struct SkillEntry { pub name: String, pub version: String, pub installed_path: String }

pub fn add_skill(skill: &Skill) -> Result<()>
```

## Used By
- `commands::init` (creates config)
- `commands::sync` (reads/writes config)
- `commands::install` (reads config)
- `commands::list` (reads `skills_repo.project`)
- `codex` (reads `targets.codex`)

## Dependencies
- `models::skill::Skill`

## Notes
- Changing the schema requires updating `commands::init` (creation) and all consumers.
- Config file uses camelCase keys via `serde(rename)`.
