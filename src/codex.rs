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

    let codex_skills_dir = Path::new(".codex/skills");
    if !codex_skills_dir.exists() {
        fs::create_dir_all(codex_skills_dir)
            .with_context(|| "Failed to create .codex/skills directory")?;
    }

    let symlink_path = codex_skills_dir.join(skill_name);
    let target_path = Path::new(".agents/skills").join(skill_name);

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
            fs::remove_file(&symlink_path)
                .with_context(|| format!("Failed to remove existing symlink at {}", symlink_path.display()))?;
        }
        symlink(&target_path, &symlink_path)
            .with_context(|| format!("Failed to create symlink from {} to {}", symlink_path.display(), target_path.display()))?;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_dir;
        if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
            fs::remove_dir(&symlink_path)
                .with_context(|| format!("Failed to remove existing symlink at {}", symlink_path.display()))?;
        }
        symlink_dir(&target_path, &symlink_path)
            .with_context(|| format!("Failed to create symlink from {} to {}", symlink_path.display(), target_path.display()))?;
    }

    Ok(())
}
