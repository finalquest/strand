use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::gitlab::GitLabClient;
use crate::models::skill::Skill;

pub fn download_and_install(client: &GitLabClient, skill: &Skill) -> Result<()> {
    let install_dir = Path::new(".agents/skills").join(&skill.name);
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("Failed to create directory {}", install_dir.display()))?;

    download_directory(client, &format!("skills/{}", skill.name), &install_dir)?;

    Ok(())
}

fn download_directory(client: &GitLabClient, remote_path: &str, local_path: &Path) -> Result<()> {
    let entries = client
        .list_tree(remote_path)
        .map_err(|e| anyhow::anyhow!("Failed to list directory {}: {}", remote_path, e))?;

    for entry in entries {
        let local_file_path = local_path.join(&entry.name);

        if entry.entry_type == "tree" {
            fs::create_dir_all(&local_file_path).with_context(|| {
                format!(
                    "Failed to create directory {}",
                    local_file_path.display()
                )
            })?;
            download_directory(client, &entry.path, &local_file_path)?;
        } else {
            let content = client
                .fetch_file(&entry.path)
                .map_err(|e| anyhow::anyhow!("Failed to fetch file {}: {}", entry.path, e))?;
            fs::write(&local_file_path, content).with_context(|| {
                format!("Failed to write file {}", local_file_path.display())
            })?;
        }
    }

    Ok(())
}


