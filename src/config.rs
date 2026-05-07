use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const CONFIG_PATH: &str = ".strand/config.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub version: i32,
    pub targets: TargetConfig,
    #[serde(rename = "skillsRepo")]
    pub skills_repo: SkillsRepoConfig,
    pub skills: Vec<SkillEntry>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TargetConfig {
    pub opencode: bool,
    #[serde(default)]
    pub codex: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SkillsRepoConfig {
    pub provider: String,
    pub project: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_base_url() -> String {
    "https://gitlab.com".to_string()
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SkillEntry {
    pub name: String,
    pub version: String,
    #[serde(rename = "installedPath")]
    pub installed_path: String,
}

pub fn add_skill(skill: &crate::models::skill::Skill) -> Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    if !config_path.exists() {
        return Ok(());
    }

    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
    let mut config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;

    config.skills.retain(|s| s.name != skill.name);

    config.skills.push(SkillEntry {
        name: skill.name.clone(),
        version: skill.version.clone(),
        installed_path: format!(".agents/skills/{}", skill.name),
    });

    let config_json = serde_json::to_string_pretty(&config)
        .with_context(|| format!("Failed to serialize {}", CONFIG_PATH))?;
    fs::write(config_path, config_json)
        .with_context(|| format!("Failed to write {}", CONFIG_PATH))?;

    Ok(())
}
