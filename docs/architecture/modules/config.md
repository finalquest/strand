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
    pub agents_repo: AgentsRepoConfig,
    pub agents: Vec<AgentEntry>,
}

pub struct TargetConfig { pub opencode: bool, pub codex: bool }
pub struct SkillsRepoConfig { pub provider: String, pub project: String, pub branch: String, pub base_url: String }
pub struct SkillEntry { pub name: String, pub version: String, pub installed_path: String }
pub struct AgentsRepoConfig { pub provider: String, pub project: String, pub branch: String, pub base_url: String }
pub struct AgentEntry { pub name: String, pub version: String, pub installed_path: String }

pub fn add_skill(skill: &Skill) -> Result<()>
pub fn add_agent(agent: &Agent) -> Result<()>
pub fn resolve_agents_repo(&self) -> (String, String, String)
```

## Used By
- `commands::init` (creates config)
- `commands::sync` (reads/writes config)
- `commands::install` (reads config)
- `commands::list` (reads `skills_repo.project`)
- `commands::agents::{ls,ls_remote}` (reads `agents_repo.project`)
- `codex` (reads `targets.codex`)

## Dependencies
- `models::skill::Skill`
- `models::agent::Agent`

## Notes
- Changing the schema requires updating `commands::init` (creation) and all consumers.
- Config file uses camelCase keys via `serde(rename)`.
- New fields `agentsRepo` and `agents` use `#[serde(default)]` for backward compatibility.
- Environment variables override config values: `strand_AGENTS_REPO`, `strand_AGENTS_REPO_BRANCH`.
