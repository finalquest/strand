use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::codex;
use crate::config::{Config, CONFIG_PATH, SkillEntry};
use crate::download;
use crate::gitignore;
use crate::gitlab::GitLabClient;
use crate::models::skill::Skill;
use crate::version::{compare_versions, VersionComparison};

pub struct SyncStatus {
    pub name: String,
    pub installed_version: String,
    pub latest_version: String,
    pub status: String,
    pub needs_update: bool,
}

pub fn execute() -> Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    if !config_path.exists() {
        println!("No configuration found. Run 'strand init' first.");
        return Ok(());
    }

    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
    let mut config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;

    if config.skills.is_empty() {
        println!("No skills installed.");
        return Ok(());
    }

    let (project, base_url, branch) = resolve_repo_config(&config)?;

    if project.is_empty() {
        println!("No skills repository configured.");
        println!("Run 'strand init' to initialize configuration.");
        return Ok(());
    }

    let client = GitLabClient::for_project(base_url, project)
        .map_err(|e| anyhow::anyhow!("Authentication failed: {}", e))?
        .with_branch(&branch);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Checking skill status...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut statuses = Vec::new();
    let mut errors = Vec::new();

    for skill_entry in &config.skills {
        match check_skill_status(&client, skill_entry) {
            Ok(status) => statuses.push(status),
            Err(e) => errors.push((skill_entry.name.clone(), e.to_string())),
        }
    }

    pb.finish_and_clear();

    render_status_table(&statuses);

    if !errors.is_empty() {
        eprintln!("\nWarning: Some skills could not be checked:");
        for (name, err) in &errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    let outdated: Vec<&SyncStatus> = statuses.iter().filter(|s| s.needs_update).collect();

    if outdated.is_empty() {
        println!("\nAll skills are up to date.");
        return Ok(());
    }

    println!("\n{} skill(s) can be updated.", outdated.len());

    if !prompt_upgrade()? {
        println!("Upgrade cancelled.");
        return Ok(());
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    let mut updated = 0;
    let mut upgrade_errors = 0;

    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    for status in &statuses {
        if !status.needs_update {
            continue;
        }

        pb.set_message(format!("Upgrading {}...", status.name));

        match upgrade_skill(&client, &mut config, status) {
            Ok(_) => {
                pb.suspend(|| {
                    println!("  Upgraded {} to v{}", status.name, status.latest_version);
                });
                updated += 1;
            }
            Err(e) => {
                pb.suspend(|| {
                    eprintln!("  Error upgrading {}: {}", status.name, e);
                });
                upgrade_errors += 1;
            }
        }
    }

    pb.finish_and_clear();

    println!("\nSync complete.");
    println!("  Updated: {}", updated);
    println!("  Errors: {}", upgrade_errors);

    Ok(())
}

fn check_skill_status(client: &GitLabClient, skill_entry: &SkillEntry) -> Result<SyncStatus> {
    let remote_skill_md = format!("skills/{}/SKILL.md", skill_entry.name);
    let content = client
        .fetch_file(&remote_skill_md)
        .map_err(|e| anyhow::anyhow!("Failed to fetch SKILL.md: {}", e))?;
    let skill: Skill = crate::models::skill::parse_skill_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse SKILL.md: {}", e))?
        .to_skill();

    let comparison = compare_versions(&skill_entry.version, &skill.version);

    let (status_text, needs_update) = match comparison {
        VersionComparison::UpToDate => ("Up to date".to_string(), false),
        VersionComparison::Behind(ref latest) => (format!("Update available ({})", latest), true),
        VersionComparison::Ahead(ref latest) => (format!("Ahead (remote: {})", latest), false),
        VersionComparison::Invalid(ref msg) => (msg.clone(), false),
    };

    Ok(SyncStatus {
        name: skill_entry.name.clone(),
        installed_version: skill_entry.version.clone(),
        latest_version: skill.version.clone(),
        status: status_text,
        needs_update,
    })
}

fn render_status_table(statuses: &[SyncStatus]) {
    let name_header = "Skill";
    let installed_header = "Installed";
    let latest_header = "Latest";
    let status_header = "Status";

    let name_width = statuses
        .iter()
        .map(|s| s.name.len())
        .chain(std::iter::once(name_header.len()))
        .max()
        .unwrap_or(10)
        .max(10);

    let installed_width = statuses
        .iter()
        .map(|s| s.installed_version.len())
        .chain(std::iter::once(installed_header.len()))
        .max()
        .unwrap_or(9)
        .max(9);

    let latest_width = statuses
        .iter()
        .map(|s| s.latest_version.len())
        .chain(std::iter::once(latest_header.len()))
        .max()
        .unwrap_or(6)
        .max(6);

    let status_width = statuses
        .iter()
        .map(|s| s.status.len())
        .chain(std::iter::once(status_header.len()))
        .max()
        .unwrap_or(10)
        .max(10);

    println!("Skill Sync Status");
    println!();

    // Top border
    print!("┌");
    print!("{}", "─".repeat(name_width + 2));
    print!("┬");
    print!("{}", "─".repeat(installed_width + 2));
    print!("┬");
    print!("{}", "─".repeat(latest_width + 2));
    print!("┬");
    print!("{}", "─".repeat(status_width + 2));
    println!("┐");

    // Header row
    print!("│ {: <width$} ", name_header, width = name_width);
    print!("│ {: <width$} ", installed_header, width = installed_width);
    print!("│ {: <width$} ", latest_header, width = latest_width);
    print!("│ {: <width$} ", status_header, width = status_width);
    println!("│");

    // Separator
    print!("├");
    print!("{}", "─".repeat(name_width + 2));
    print!("┼");
    print!("{}", "─".repeat(installed_width + 2));
    print!("┼");
    print!("{}", "─".repeat(latest_width + 2));
    print!("┼");
    print!("{}", "─".repeat(status_width + 2));
    println!("┤");

    // Data rows
    for status in statuses {
        print!("│ {: <width$} ", status.name, width = name_width);
        print!("│ {: <width$} ", status.installed_version, width = installed_width);
        print!("│ {: <width$} ", status.latest_version, width = latest_width);
        print!("│ {: <width$} ", status.status, width = status_width);
        println!("│");
    }

    // Bottom border
    print!("└");
    print!("{}", "─".repeat(name_width + 2));
    print!("┴");
    print!("{}", "─".repeat(installed_width + 2));
    print!("┴");
    print!("{}", "─".repeat(latest_width + 2));
    print!("┴");
    print!("{}", "─".repeat(status_width + 2));
    println!("┘");
}

fn prompt_upgrade() -> Result<bool> {
    print!("Do you want to upgrade outdated skills? [y/N]: ");
    io::stdout().flush().context("Failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;

    let trimmed = input.trim().to_lowercase();
    Ok(trimmed == "y" || trimmed == "yes")
}

fn upgrade_skill(
    client: &GitLabClient,
    config: &mut Config,
    status: &SyncStatus,
) -> Result<()> {
    let remote_skill_md = format!("skills/{}/SKILL.md", status.name);
    let content = client
        .fetch_file(&remote_skill_md)
        .map_err(|e| anyhow::anyhow!("Failed to fetch SKILL.md: {}", e))?;
    let skill: Skill = crate::models::skill::parse_skill_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse SKILL.md: {}", e))?
        .to_skill();

    // Download and install
    download::download_and_install(client, &skill)?;

    // Update config
    if let Some(entry) = config.skills.iter_mut().find(|e| e.name == status.name) {
        entry.version = skill.version.clone();
    }

    let config_json = serde_json::to_string_pretty(config)
        .with_context(|| format!("Failed to serialize {}", CONFIG_PATH))?;
    fs::write(CONFIG_PATH, config_json)
        .with_context(|| format!("Failed to write {}", CONFIG_PATH))?;

    // Post-install hooks
    gitignore::ensure_gitignore_entries(&skill.name)?;
    if config.targets.codex {
        codex::create_symlink(&skill.name)?;
    }

    Ok(())
}

fn resolve_repo_config(config: &Config) -> Result<(String, String, String)> {
    let env_base_url = std::env::var("strand_GITLAB_URL").ok();

    if let Ok(project) = std::env::var("strand_SKILLS_REPO") {
        let branch = std::env::var("strand_SKILLS_REPO_BRANCH").unwrap_or_else(|_| "main".to_string());
        let base_url = env_base_url.unwrap_or_else(|| "https://gitlab.com".to_string());
        return Ok((project, base_url, branch));
    }

    if !config.skills_repo.project.is_empty() {
        let branch = if config.skills_repo.branch.is_empty() {
            "main".to_string()
        } else {
            config.skills_repo.branch.clone()
        };
        let base_url = env_base_url
            .or_else(|| {
                if config.skills_repo.base_url.is_empty() {
                    None
                } else {
                    Some(config.skills_repo.base_url.clone())
                }
            })
            .unwrap_or_else(|| "https://gitlab.com".to_string());
        return Ok((config.skills_repo.project.clone(), base_url, branch));
    }

    let base_url = env_base_url.unwrap_or_else(|| "https://gitlab.com".to_string());
    Ok((String::new(), base_url, "main".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_status_table_empty() {
        let statuses: Vec<SyncStatus> = vec![];
        render_status_table(&statuses);
    }

    #[test]
    fn test_render_status_table_with_data() {
        let statuses = vec![
            SyncStatus {
                name: "test-skill".to_string(),
                installed_version: "1.0.0".to_string(),
                latest_version: "1.1.0".to_string(),
                status: "Update available (1.1.0)".to_string(),
                needs_update: true,
            },
            SyncStatus {
                name: "other-skill".to_string(),
                installed_version: "2.0.0".to_string(),
                latest_version: "2.0.0".to_string(),
                status: "Up to date".to_string(),
                needs_update: false,
            },
        ];
        render_status_table(&statuses);
    }

    #[test]
    fn test_sync_status_struct() {
        let status = SyncStatus {
            name: "my-skill".to_string(),
            installed_version: "1.0.0".to_string(),
            latest_version: "1.1.0".to_string(),
            status: "Update available".to_string(),
            needs_update: true,
        };
        assert_eq!(status.name, "my-skill");
        assert!(status.needs_update);
    }
}
