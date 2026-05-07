use std::fs;
use std::io::{self, IsTerminal, Write};
use anyhow::Result;
use dialoguer::Confirm;

use crate::config::{Config, TargetConfig, SkillsRepoConfig, CONFIG_PATH};
use crate::auth::glab::GlabAuth;

fn prompt_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn resolve_base_url(is_interactive: bool) -> String {
    // Env var always wins
    if let Ok(url) = std::env::var("strand_GITLAB_URL") {
        return url;
    }
    if let Ok(url) = std::env::var("strand_INIT_BASE_URL") {
        return url;
    }

    let glab = GlabAuth::new();
    if glab.is_installed() {
        let hosts = glab.configured_hosts();
        match hosts.len() {
            0 => {}
            1 => {
                let host = &hosts[0];
                println!("Using GitLab host: {}", host);
                return format!("https://{}", host);
            }
            _ => {
                if is_interactive {
                    println!("Multiple GitLab hosts detected in glab:");
                    for (i, host) in hosts.iter().enumerate() {
                        println!("  {}. {}", i + 1, host);
                    }
                    loop {
                        let input = match prompt_input("Select host number (or type full URL): ") {
                            Ok(s) => s,
                            Err(_) => break,
                        };
                        if let Ok(num) = input.parse::<usize>() {
                            if num > 0 && num <= hosts.len() {
                                return format!("https://{}", hosts[num - 1]);
                            }
                        }
                        if input.starts_with("http") {
                            return input;
                        }
                        println!("Invalid selection. Try again.");
                    }
                }
            }
        }
    }

    "https://gitlab.com".to_string()
}

pub fn init() -> Result<()> {
    // Create directories
    fs::create_dir_all(".strand")?;
    fs::create_dir_all(".agents/skills")?;

    let is_interactive = io::stdin().is_terminal();

    // Prompt for Codex integration (or env override)
    let enable_codex = if is_interactive {
        Confirm::new()
            .with_prompt("Enable Codex integration?")
            .default(false)
            .interact()?
    } else if let Ok(val) = std::env::var("strand_INIT_CODEX") {
        val == "1" || val.eq_ignore_ascii_case("true") || val.eq_ignore_ascii_case("yes")
    } else {
        false
    };

    if enable_codex {
        fs::create_dir_all(".codex/skills")?;
    }

    // Resolve base URL / hostname
    let base_url = resolve_base_url(is_interactive);

    // Prompt for skills repository configuration (or env overrides)
    let project = if is_interactive {
        prompt_input("GitLab project path for skills repository (e.g., namespace/project): ")?
    } else {
        std::env::var("strand_INIT_PROJECT").unwrap_or_default()
    };

    let branch = if is_interactive && !project.is_empty() {
        let input = prompt_input("Branch or tag to use [main]: ")?;
        if input.is_empty() { "main".to_string() } else { input }
    } else {
        std::env::var("strand_INIT_BRANCH").unwrap_or_else(|_| "main".to_string())
    };

    // Load existing config or create new one
    let mut config = if std::path::Path::new(CONFIG_PATH).exists() {
        let config_str = fs::read_to_string(CONFIG_PATH)?;
        serde_json::from_str(&config_str).unwrap_or_else(|_| Config {
            version: 1,
            targets: TargetConfig::default(),
            skills_repo: SkillsRepoConfig::default(),
            skills: Vec::new(),
        })
    } else {
        Config {
            version: 1,
            targets: TargetConfig::default(),
            skills_repo: SkillsRepoConfig::default(),
            skills: Vec::new(),
        }
    };

    // Update configurable fields
    config.targets.opencode = true;
    config.targets.codex = enable_codex;
    config.skills_repo.provider = "gitlab".to_string();
    config.skills_repo.project = project;
    config.skills_repo.branch = branch;
    config.skills_repo.base_url = base_url;
    // skills are preserved from existing config

    let config_json = serde_json::to_string_pretty(&config)?;
    fs::write(CONFIG_PATH, config_json)?;

    Ok(())
}
