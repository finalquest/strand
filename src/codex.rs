use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::config::{Config, CONFIG_PATH};

pub fn create_symlink(skill_name: &str) -> Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    if !config_path.exists() {
        return Ok(());
    }

    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
    let config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;

    if !config.targets.codex {
        return Ok(());
    }

    crate::symlinks::create_symlink(".agents/skills", ".codex/skills", skill_name)
}
