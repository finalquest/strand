use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::codex;
use crate::config::{AgentEntry, Config, CONFIG_PATH, SkillEntry};
use crate::download;
use crate::gitignore;
use crate::gitlab::GitLabClient;
use crate::models::agent::Agent;
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

    if config.skills.is_empty() && config.agents.is_empty() {
        println!("No skills or agents installed.");
        return Ok(());
    }

    let mut client_opt = None;
    if !config.skills.is_empty() {
        let (project, base_url, branch) = resolve_repo_config(&config)?;

        if project.is_empty() {
            println!("No skills repository configured.");
            println!("Run 'strand init' to initialize configuration.");
            return Ok(());
        }

        let client = GitLabClient::for_project(base_url, project)
            .map_err(|e| anyhow::anyhow!("Authentication failed: {}", e))?
            .with_branch(&branch);
        client_opt = Some(client);
    }

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

    if let Some(ref client) = client_opt {
        for skill_entry in &config.skills {
            match check_skill_status(client, skill_entry) {
                Ok(status) => statuses.push(status),
                Err(e) => errors.push((skill_entry.name.clone(), e.to_string())),
            }
        }
    }

    let mut agent_statuses = Vec::new();
    let mut agent_errors = Vec::new();
    let mut agents_client_opt = None;

    if !config.agents.is_empty() {
        let (agents_project, agents_base_url, agents_branch) = config.resolve_agents_repo();

        if !agents_project.is_empty() {
            let agents_client = GitLabClient::for_project(agents_base_url, agents_project)
                .map_err(|e| anyhow::anyhow!("Authentication failed for agents repo: {}", e))?
                .with_branch(&agents_branch);

            pb.set_message("Checking agent status...");

            for agent_entry in &config.agents {
                match check_agent_status(&agents_client, agent_entry) {
                    Ok(status) => agent_statuses.push(status),
                    Err(e) => agent_errors.push((agent_entry.name.clone(), e.to_string())),
                }
            }

            agents_client_opt = Some(agents_client);
        }
    }

    pb.finish_and_clear();

    if !statuses.is_empty() {
        render_status_table(&statuses);
    }

    if !agent_statuses.is_empty() {
        println!();
        render_agent_status_table(&agent_statuses);
    }

    if !errors.is_empty() {
        eprintln!("\nWarning: Some skills could not be checked:");
        for (name, err) in &errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    if !agent_errors.is_empty() {
        eprintln!("\nWarning: Some agents could not be checked:");
        for (name, err) in &agent_errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    let outdated_skills: Vec<&SyncStatus> = statuses.iter().filter(|s| s.needs_update).collect();
    let outdated_agents: Vec<&SyncStatus> = agent_statuses.iter().filter(|s| s.needs_update).collect();

    if outdated_skills.is_empty() && outdated_agents.is_empty() {
        println!("\nAll skills and agents are up to date.");
        return Ok(());
    }

    if !outdated_skills.is_empty() {
        println!("\n{} skill(s) can be updated.", outdated_skills.len());
    }
    if !outdated_agents.is_empty() {
        println!("{} agent(s) can be updated.", outdated_agents.len());
    }

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

    let mut skills_updated = 0;
    let mut skills_errors = 0;
    let mut agents_updated = 0;
    let mut agents_errors = 0;

    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    if let Some(ref client) = client_opt {
        for status in &statuses {
            if !status.needs_update {
                continue;
            }

            pb.set_message(format!("Upgrading {}...", status.name));

            match upgrade_skill(client, &mut config, status) {
                Ok(_) => {
                    pb.suspend(|| {
                        println!("  Upgraded {} to v{}", status.name, status.latest_version);
                    });
                    skills_updated += 1;
                }
                Err(e) => {
                    pb.suspend(|| {
                        eprintln!("  Error upgrading {}: {}", status.name, e);
                    });
                    skills_errors += 1;
                }
            }
        }
    }

    if let Some(ref agents_client) = agents_client_opt {
        for status in &agent_statuses {
            if !status.needs_update {
                continue;
            }

            pb.set_message(format!("Upgrading {}...", status.name));

            match upgrade_agent(agents_client, &mut config, status) {
                Ok(_) => {
                    pb.suspend(|| {
                        println!("  Upgraded {} to v{}", status.name, status.latest_version);
                    });
                    agents_updated += 1;
                }
                Err(e) => {
                    pb.suspend(|| {
                        eprintln!("  Error upgrading {}: {}", status.name, e);
                    });
                    agents_errors += 1;
                }
            }
        }
    }

    pb.finish_and_clear();

    println!("\nSync complete.");
    println!("  Skills updated: {}", skills_updated);
    println!("  Agents updated: {}", agents_updated);
    println!("  Errors: {}", skills_errors + agents_errors);

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
    print!("Do you want to upgrade outdated skills and agents? [y/N]: ");
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

    // Install declared agents
    if !skill.agents.is_empty() {
        println!(
            "  {} requires {} agent(s), installing...",
            skill.name,
            skill.agents.len()
        );

        let (agents_project, agents_base_url, agents_branch) = config.resolve_agents_repo();

        if agents_project.is_empty() {
            println!("  Warning: skill requires agents but no agents repo configured");
        } else {
            match GitLabClient::for_project(agents_base_url, agents_project) {
                Ok(agents_client) => {
                    let agents_client = agents_client.with_branch(&agents_branch);
                    crate::commands::agents::helpers::install_skill_agents(
                        &skill.agents,
                        &agents_client,
                        &config.targets,
                    );
                }
                Err(e) => {
                    eprintln!("  Warning: failed to create agents client: {}", e);
                }
            }
        }
    }

    Ok(())
}

fn check_agent_status(client: &GitLabClient, agent_entry: &AgentEntry) -> Result<SyncStatus> {
    let remote_agent_md = format!("agents/{}/AGENT.md", agent_entry.name);
    let content = client
        .fetch_file(&remote_agent_md)
        .map_err(|e| anyhow::anyhow!("Failed to fetch AGENT.md: {}", e))?;
    let agent: Agent = crate::models::agent::parse_agent_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse AGENT.md: {}", e))?
        .to_agent();

    let comparison = compare_versions(&agent_entry.version, &agent.version);

    let (status_text, needs_update) = match comparison {
        VersionComparison::UpToDate => ("Up to date".to_string(), false),
        VersionComparison::Behind(ref latest) => (format!("Update available ({})", latest), true),
        VersionComparison::Ahead(ref latest) => (format!("Ahead (remote: {})", latest), false),
        VersionComparison::Invalid(ref msg) => (msg.clone(), false),
    };

    Ok(SyncStatus {
        name: agent_entry.name.clone(),
        installed_version: agent_entry.version.clone(),
        latest_version: agent.version.clone(),
        status: status_text,
        needs_update,
    })
}

fn render_agent_status_table(statuses: &[SyncStatus]) {
    let name_header = "Agent";
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

    println!("Agent Sync Status");
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

fn upgrade_agent(
    client: &GitLabClient,
    config: &mut Config,
    status: &SyncStatus,
) -> Result<()> {
    let remote_agent_md = format!("agents/{}/AGENT.md", status.name);
    let content = client
        .fetch_file(&remote_agent_md)
        .map_err(|e| anyhow::anyhow!("Failed to fetch AGENT.md: {}", e))?;
    let agent: Agent = crate::models::agent::parse_agent_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse AGENT.md: {}", e))?
        .to_agent();

    // Download and install
    crate::commands::agents::helpers::download_and_install_agent(client, &agent)?;

    // Update config
    if let Some(entry) = config.agents.iter_mut().find(|e| e.name == status.name) {
        entry.version = agent.version.clone();
    }

    let config_json = serde_json::to_string_pretty(config)
        .with_context(|| format!("Failed to serialize {}", CONFIG_PATH))?;
    fs::write(CONFIG_PATH, config_json)
        .with_context(|| format!("Failed to write {}", CONFIG_PATH))?;

    // Post-install hooks
    crate::commands::agents::helpers::ensure_gitignore_entries_for_agent(&agent.name)?;
    crate::commands::agents::helpers::create_agent_symlinks(&agent.name, &config.targets)?;

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

    // Mock transport helpers for sync tests

    struct MockSyncTransport {
        handler: Box<dyn Fn(&str) -> Result<(u16, String), crate::gitlab::GitLabError>>,
    }

    impl crate::gitlab::Transport for MockSyncTransport {
        fn call(&self, endpoint: &str) -> Result<(u16, String), crate::gitlab::GitLabError> {
            (self.handler)(endpoint)
        }
    }

    fn mock_skill_client(name: &str, version: &str) -> GitLabClient {
        let skill_md = format!(
            "---\nname: {}\ndescription: test\nmetadata:\n  version: {}\n---\n",
            name, version
        );
        let name = name.to_string();
        GitLabClient::with_transport(
            Box::new(MockSyncTransport {
                handler: Box::new(move |endpoint| {
                    if endpoint.contains(&format!("files/skills%2F{}%2FSKILL.md", name)) {
                        Ok((200, skill_md.clone()))
                    } else {
                        Ok((404, "Not Found".to_string()))
                    }
                }),
            }),
            "test/project".to_string(),
        )
    }

    fn mock_agent_client(name: &str, version: &str) -> GitLabClient {
        let agent_md = format!(
            "---\nname: {}\ndescription: test\nmetadata:\n  version: {}\n---\n",
            name, version
        );
        let name = name.to_string();
        GitLabClient::with_transport(
            Box::new(MockSyncTransport {
                handler: Box::new(move |endpoint| {
                    if endpoint.contains(&format!("files/agents%2F{}%2FAGENT.md", name)) {
                        Ok((200, agent_md.clone()))
                    } else {
                        Ok((404, "Not Found".to_string()))
                    }
                }),
            }),
            "test/agents".to_string(),
        )
    }

    #[test]
    fn test_check_skill_status_up_to_date() {
        let client = mock_skill_client("test-skill", "1.0.0");
        let skill_entry = SkillEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/skills/test-skill".to_string(),
        };

        let status = check_skill_status(&client, &skill_entry).unwrap();
        assert_eq!(status.name, "test-skill");
        assert_eq!(status.installed_version, "1.0.0");
        assert_eq!(status.latest_version, "1.0.0");
        assert_eq!(status.status, "Up to date");
        assert!(!status.needs_update);
    }

    #[test]
    fn test_check_skill_status_update_available() {
        let client = mock_skill_client("test-skill", "1.1.0");
        let skill_entry = SkillEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/skills/test-skill".to_string(),
        };

        let status = check_skill_status(&client, &skill_entry).unwrap();
        assert_eq!(status.name, "test-skill");
        assert_eq!(status.installed_version, "1.0.0");
        assert_eq!(status.latest_version, "1.1.0");
        assert_eq!(status.status, "Update available (1.1.0)");
        assert!(status.needs_update);
    }

    #[test]
    fn test_check_agent_status_up_to_date() {
        let client = mock_agent_client("test-agent", "1.0.0");
        let agent_entry = AgentEntry {
            name: "test-agent".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/agents/test-agent".to_string(),
        };

        let status = check_agent_status(&client, &agent_entry).unwrap();
        assert_eq!(status.name, "test-agent");
        assert_eq!(status.installed_version, "1.0.0");
        assert_eq!(status.latest_version, "1.0.0");
        assert_eq!(status.status, "Up to date");
        assert!(!status.needs_update);
    }

    #[test]
    fn test_check_agent_status_update_available() {
        let client = mock_agent_client("test-agent", "2.0.0");
        let agent_entry = AgentEntry {
            name: "test-agent".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/agents/test-agent".to_string(),
        };

        let status = check_agent_status(&client, &agent_entry).unwrap();
        assert_eq!(status.name, "test-agent");
        assert_eq!(status.installed_version, "1.0.0");
        assert_eq!(status.latest_version, "2.0.0");
        assert_eq!(status.status, "Update available (2.0.0)");
        assert!(status.needs_update);
    }

    // Config scenario tests: skills-only, agents-only, mixed

    #[test]
    fn test_sync_skills_only_config() {
        let client = mock_skill_client("test-skill", "1.0.0");
        let config = Config {
            version: 1,
            targets: crate::config::TargetConfig::default(),
            skills_repo: crate::config::SkillsRepoConfig {
                provider: "gitlab".to_string(),
                project: "test/project".to_string(),
                branch: "main".to_string(),
                base_url: "https://gitlab.com".to_string(),
            },
            skills: vec![SkillEntry {
                name: "test-skill".to_string(),
                version: "1.0.0".to_string(),
                installed_path: ".agents/skills/test-skill".to_string(),
            }],
            ..Default::default()
        };

        assert!(!config.skills.is_empty());
        assert!(config.agents.is_empty());

        let skill_entry = &config.skills[0];
        let status = check_skill_status(&client, skill_entry).unwrap();
        assert_eq!(status.name, "test-skill");
        assert!(!status.needs_update);
    }

    #[test]
    fn test_sync_agents_only_config() {
        let client = mock_agent_client("test-agent", "1.0.0");
        let config = Config {
            version: 1,
            targets: crate::config::TargetConfig::default(),
            skills_repo: crate::config::SkillsRepoConfig::default(),
            agents_repo: crate::config::AgentsRepoConfig {
                provider: "gitlab".to_string(),
                project: "test/agents".to_string(),
                branch: "main".to_string(),
                base_url: "https://gitlab.com".to_string(),
            },
            skills: vec![],
            agents: vec![AgentEntry {
                name: "test-agent".to_string(),
                version: "1.0.0".to_string(),
                installed_path: ".agents/agents/test-agent".to_string(),
            }],
        };

        assert!(config.skills.is_empty());
        assert!(!config.agents.is_empty());

        let agent_entry = &config.agents[0];
        let status = check_agent_status(&client, agent_entry).unwrap();
        assert_eq!(status.name, "test-agent");
        assert!(!status.needs_update);
    }

    #[test]
    fn test_sync_mixed_config() {
        let skill_client = mock_skill_client("test-skill", "1.0.0");
        let agent_client = mock_agent_client("test-agent", "1.0.0");
        let config = Config {
            version: 1,
            targets: crate::config::TargetConfig::default(),
            skills_repo: crate::config::SkillsRepoConfig {
                provider: "gitlab".to_string(),
                project: "test/project".to_string(),
                branch: "main".to_string(),
                base_url: "https://gitlab.com".to_string(),
            },
            agents_repo: crate::config::AgentsRepoConfig {
                provider: "gitlab".to_string(),
                project: "test/agents".to_string(),
                branch: "main".to_string(),
                base_url: "https://gitlab.com".to_string(),
            },
            skills: vec![SkillEntry {
                name: "test-skill".to_string(),
                version: "1.0.0".to_string(),
                installed_path: ".agents/skills/test-skill".to_string(),
            }],
            agents: vec![AgentEntry {
                name: "test-agent".to_string(),
                version: "1.0.0".to_string(),
                installed_path: ".agents/agents/test-agent".to_string(),
            }],
        };

        assert!(!config.skills.is_empty());
        assert!(!config.agents.is_empty());

        let skill_status = check_skill_status(&skill_client, &config.skills[0]).unwrap();
        assert_eq!(skill_status.name, "test-skill");
        assert!(!skill_status.needs_update);

        let agent_status = check_agent_status(&agent_client, &config.agents[0]).unwrap();
        assert_eq!(agent_status.name, "test-agent");
        assert!(!agent_status.needs_update);
    }
}
