use anyhow::{Context, Result};
use dialoguer::{FuzzySelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::path::Path;

use crate::config::{Config, CONFIG_PATH, TargetConfig};
use crate::gitlab::GitLabClient;
use crate::models::agent::Agent;

pub fn execute() -> Result<()> {
    // Read config (or use default)
    let config = if Path::new(CONFIG_PATH).exists() {
        let config_str = std::fs::read_to_string(CONFIG_PATH)
            .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
        serde_json::from_str::<Config>(&config_str)
            .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?
    } else {
        Config::default()
    };

    let (project, base_url, branch) = config.resolve_agents_repo();

    if project.is_empty() {
        println!("No agents repository configured.");
        println!(
            "Run 'strand init' to initialize configuration or set the strand_AGENTS_REPO environment variable."
        );
        return Ok(());
    }

    // Create GitLab client
    let client = GitLabClient::for_project(base_url, project)
        .map_err(|e| anyhow::anyhow!("Authentication failed: {}", e))?
        .with_branch(&branch);

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Fetching remote agents...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // List agents directory
    let entries = match client.list_tree("agents") {
        Ok(entries) => entries,
        Err(e) => {
            match e {
                crate::gitlab::GitLabError::AuthError(msg) => {
                    return Err(anyhow::anyhow!("Authentication failed: {}", msg));
                }
                crate::gitlab::GitLabError::NotFound(_) => {
                    println!("No agents found in the repository.");
                    return Ok(());
                }
                _ => {
                    return Err(anyhow::anyhow!("Failed to fetch agents: {}", e));
                }
            }
        }
    };

    if entries.is_empty() {
        println!("No agents found in the repository.");
        return Ok(());
    }

    // Fetch and parse each AGENT.md
    let mut agents = Vec::new();
    let mut errors = Vec::new();

    for entry in entries {
        if entry.entry_type != "tree" {
            continue;
        }

        let agent_md_path = format!("agents/{}/AGENT.md", entry.name);
        match client.fetch_file(&agent_md_path) {
            Ok(content) => {
                match crate::models::agent::parse_agent_md(&content) {
                    Ok(frontmatter) => {
                        let agent = frontmatter.to_agent();
                        agents.push(agent);
                    }
                    Err(e) => {
                        errors.push((entry.name, format!("Failed to parse AGENT.md: {}", e)));
                    }
                }
            }
            Err(crate::gitlab::GitLabError::NotFound(_)) => {
                errors.push((entry.name, "AGENT.md not found".to_string()));
            }
            Err(e) => {
                errors.push((entry.name, format!("Failed to fetch AGENT.md: {}", e)));
            }
        }
    }

    pb.finish_and_clear();

    if agents.is_empty() {
        if !errors.is_empty() {
            eprintln!("Warning: Could not load any agents:");
            for (name, err) in &errors {
                eprintln!("  {}: {}", name, err);
            }
        }
        println!("No agents available.");
        return Ok(());
    }

    if std::io::stdin().is_terminal() {
        // Interactive mode: fuzzy select with names only, then show details
        loop {
            let items: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();

            let selection = FuzzySelect::new()
                .with_prompt("Select an agent (type to filter)")
                .items(&items)
                .interact_opt()?;

            if let Some(index) = selection {
                let agent = &agents[index];
                println!("\nAgent:        {}", agent.name);
                println!("Description:  {}", agent.description);
                println!("Version:      {}", agent.version);

                let options = vec!["Install this agent", "Back to list", "Quit"];
                let action = Select::new()
                    .with_prompt("What would you like to do?")
                    .items(&options)
                    .default(0)
                    .interact_opt()?;

                match action {
                    Some(0) => {
                        // Install
                        if let Err(e) = install_agent(&client, agent) {
                            eprintln!("Installation failed: {}", e);
                        }
                        break;
                    }
                    Some(1) => {
                        // Back to list
                        continue;
                    }
                    _ => {
                        // Quit or cancelled
                        break;
                    }
                }
            } else {
                // User cancelled fuzzy select
                break;
            }
        }
    } else {
        // Non-interactive mode: print table without description
        render_table(&agents);
    }

    // Show any errors
    if !errors.is_empty() {
        eprintln!("\nWarning: Some agents could not be loaded:");
        for (name, err) in &errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    Ok(())
}

fn render_table(agents: &[Agent]) {
    let name_header = "Agent";
    let ver_header = "Version";

    let name_width = agents
        .iter()
        .map(|a| a.name.len())
        .chain(std::iter::once(name_header.len()))
        .max()
        .unwrap_or(10)
        .max(10);

    let ver_width = agents
        .iter()
        .map(|a| a.version.len())
        .chain(std::iter::once(ver_header.len()))
        .max()
        .unwrap_or(7)
        .max(7);

    println!("Available Agents");
    println!();

    print!("┌");
    print!("{}", "─".repeat(name_width + 2));
    print!("┬");
    print!("{}", "─".repeat(ver_width + 2));
    println!("┐");

    print!("│ {: <width$} ", name_header, width = name_width);
    print!("│ {: <width$} ", ver_header, width = ver_width);
    println!("│");

    print!("├");
    print!("{}", "─".repeat(name_width + 2));
    print!("┼");
    print!("{}", "─".repeat(ver_width + 2));
    println!("┤");

    for agent in agents {
        print!("│ {: <width$} ", agent.name, width = name_width);
        print!("│ {: <width$} ", agent.version, width = ver_width);
        println!("│");
    }

    print!("└");
    print!("{}", "─".repeat(name_width + 2));
    print!("┴");
    print!("{}", "─".repeat(ver_width + 2));
    println!("┘");
}

fn install_agent(client: &GitLabClient, agent: &Agent) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Installing {}...", agent.name));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    crate::commands::agents::helpers::download_and_install_agent(client, agent)?;

    pb.finish_and_clear();

    crate::commands::agents::helpers::update_config_with_agent(agent)?;
    crate::commands::agents::helpers::ensure_gitignore_entries_for_agent(&agent.name)?;

    // Read config to get targets for symlinks
    let targets = if Path::new(CONFIG_PATH).exists() {
        let config_str = std::fs::read_to_string(CONFIG_PATH)
            .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
        let config: Config = serde_json::from_str(&config_str)
            .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;
        config.targets
    } else {
        TargetConfig::default()
    };

    crate::commands::agents::helpers::create_agent_symlinks(&agent.name, &targets)?;

    println!(
        "Successfully installed {} v{}",
        agent.name, agent.version
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_table_empty() {
        let agents: Vec<Agent> = vec![];
        render_table(&agents);
    }

    #[test]
    fn test_render_table_with_data() {
        let agents = vec![
            Agent {
                name: "test-agent".to_string(),
                description: "A test agent".to_string(),
                version: "1.0.0".to_string(),
            },
            Agent {
                name: "other-agent".to_string(),
                description: "Another agent".to_string(),
                version: "2.0.0".to_string(),
            },
        ];
        render_table(&agents);
    }
}
