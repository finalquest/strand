use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config::{Config, CONFIG_PATH, AgentEntry};
use crate::gitlab::GitLabClient;
use crate::models::agent::Agent;

pub struct AgentStatus {
    pub name: String,
    pub local_version: String,
    pub remote_version: String,
    pub outdated: bool,
    pub installed: bool,
}

pub fn execute() -> Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    if !config_path.exists() {
        println!("No configuration found. Run 'strand init' first.");
        return Ok(());
    }

    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
    let config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;

    if config.agents.is_empty() {
        println!("No agents installed.");
        return Ok(());
    }

    let (project, base_url, branch) = config.resolve_agents_repo();

    if project.is_empty() {
        println!("No agents repository configured.");
        println!("Run 'strand init' to initialize configuration.");
        return Ok(());
    }

    let client = GitLabClient::for_project(base_url, project)
        .map_err(|e| anyhow::anyhow!("Authentication failed: {}", e))?
        .with_branch(&branch);

    let mut statuses = Vec::new();
    let mut errors = Vec::new();

    for agent_entry in &config.agents {
        match check_agent_version(&client, agent_entry) {
            Ok(status) => statuses.push(status),
            Err(e) => errors.push((agent_entry.name.clone(), e.to_string())),
        }
    }

    render_table(&statuses);

    if !errors.is_empty() {
        eprintln!("\nWarning: Some agents could not be checked:");
        for (name, err) in &errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    Ok(())
}

fn check_agent_version(client: &GitLabClient, agent_entry: &AgentEntry) -> Result<AgentStatus> {
    let installed_path = Path::new(&agent_entry.installed_path);
    let agent_md_path = installed_path.join("AGENT.md");
    let installed = installed_path.exists() && agent_md_path.exists();

    let remote_agent_md = format!("agents/{}/AGENT.md", agent_entry.name);
    let content = client
        .fetch_file(&remote_agent_md)
        .map_err(|e| anyhow::anyhow!("Failed to fetch AGENT.md: {}", e))?;
    let agent: Agent = crate::models::agent::parse_agent_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse AGENT.md: {}", e))?
        .to_agent();

    let outdated = installed && agent_entry.version != agent.version;

    Ok(AgentStatus {
        name: agent_entry.name.clone(),
        local_version: agent_entry.version.clone(),
        remote_version: agent.version,
        outdated,
        installed,
    })
}

fn render_table(statuses: &[AgentStatus]) {
    let name_header = "Agent";
    let local_header = "Local Version";
    let remote_header = "Remote Version";
    let status_header = "Status";

    let name_width = statuses
        .iter()
        .map(|s| s.name.len())
        .chain(std::iter::once(name_header.len()))
        .max()
        .unwrap_or(10)
        .max(10);

    let local_width = statuses
        .iter()
        .map(|s| s.local_version.len())
        .chain(std::iter::once(local_header.len()))
        .max()
        .unwrap_or(13)
        .max(13);

    let remote_width = statuses
        .iter()
        .map(|s| s.remote_version.len())
        .chain(std::iter::once(remote_header.len()))
        .max()
        .unwrap_or(14)
        .max(14);

    let status_width = statuses
        .iter()
        .map(|s| {
            if s.installed {
                if s.outdated {
                    "Outdated".len()
                } else {
                    "Up to date".len()
                }
            } else {
                "Not installed".len()
            }
        })
        .chain(std::iter::once(status_header.len()))
        .max()
        .unwrap_or(13)
        .max(13);

    println!("Agents");
    println!();

    // Top border
    print!("┌");
    print!("{}", "─".repeat(name_width + 2));
    print!("┬");
    print!("{}", "─".repeat(local_width + 2));
    print!("┬");
    print!("{}", "─".repeat(remote_width + 2));
    print!("┬");
    print!("{}", "─".repeat(status_width + 2));
    println!("┐");

    // Header row
    print!("│ {: <width$} ", name_header, width = name_width);
    print!("│ {: <width$} ", local_header, width = local_width);
    print!("│ {: <width$} ", remote_header, width = remote_width);
    print!("│ {: <width$} ", status_header, width = status_width);
    println!("│");

    // Separator
    print!("├");
    print!("{}", "─".repeat(name_width + 2));
    print!("┼");
    print!("{}", "─".repeat(local_width + 2));
    print!("┼");
    print!("{}", "─".repeat(remote_width + 2));
    print!("┼");
    print!("{}", "─".repeat(status_width + 2));
    println!("┤");

    // Data rows
    for status in statuses {
        print!("│ {: <width$} ", status.name, width = name_width);

        if !status.installed {
            print!("│ {: <width$} ", "—", width = local_width);
        } else if status.outdated {
            let red_version = status.local_version.red().to_string();
            let visible_len = status.local_version.len();
            let padding = local_width.saturating_sub(visible_len);
            print!("│ {}{} ", red_version, " ".repeat(padding));
        } else {
            print!("│ {: <width$} ", status.local_version, width = local_width);
        }

        print!("│ {: <width$} ", status.remote_version, width = remote_width);

        if !status.installed {
            let red_status = "Not installed".red().to_string();
            let visible_len = "Not installed".len();
            let padding = status_width.saturating_sub(visible_len);
            print!("│ {}{} ", red_status, " ".repeat(padding));
        } else if status.outdated {
            let yellow_status = "Outdated".yellow().to_string();
            let visible_len = "Outdated".len();
            let padding = status_width.saturating_sub(visible_len);
            print!("│ {}{} ", yellow_status, " ".repeat(padding));
        } else {
            let green_status = "Up to date".green().to_string();
            let visible_len = "Up to date".len();
            let padding = status_width.saturating_sub(visible_len);
            print!("│ {}{} ", green_status, " ".repeat(padding));
        }
        println!("│");
    }

    // Bottom border
    print!("└");
    print!("{}", "─".repeat(name_width + 2));
    print!("┴");
    print!("{}", "─".repeat(local_width + 2));
    print!("┴");
    print!("{}", "─".repeat(remote_width + 2));
    print!("┴");
    print!("{}", "─".repeat(status_width + 2));
    println!("┘");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_table_empty() {
        let statuses: Vec<AgentStatus> = vec![];
        render_table(&statuses);
    }

    #[test]
    fn test_render_table_with_data() {
        let statuses = vec![
            AgentStatus {
                name: "test-agent".to_string(),
                local_version: "1.0.0".to_string(),
                remote_version: "1.1.0".to_string(),
                outdated: true,
                installed: true,
            },
            AgentStatus {
                name: "other-agent".to_string(),
                local_version: "2.0.0".to_string(),
                remote_version: "2.0.0".to_string(),
                outdated: false,
                installed: true,
            },
            AgentStatus {
                name: "missing-agent".to_string(),
                local_version: "1.0.0".to_string(),
                remote_version: "1.0.0".to_string(),
                outdated: false,
                installed: false,
            },
        ];
        render_table(&statuses);
    }

    #[test]
    fn test_agent_status_struct() {
        let status = AgentStatus {
            name: "my-agent".to_string(),
            local_version: "1.0.0".to_string(),
            remote_version: "1.1.0".to_string(),
            outdated: true,
            installed: true,
        };
        assert_eq!(status.name, "my-agent");
        assert!(status.outdated);
        assert!(status.installed);
    }
}
