use anyhow::Result;
use dialoguer::{FuzzySelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;

use crate::codex;
use crate::config::Config;
use crate::download;
use crate::gitignore;
use crate::gitlab::GitLabClient;
use crate::models::skill::Skill;

pub fn execute() -> Result<()> {
    // Determine project and base URL
    let (project, base_url, branch) = resolve_repo_config()?;

    if project.is_empty() {
        println!("No skills repository configured.");
        println!(
            "Run 'strand init' to initialize configuration or set the strand_SKILLS_REPO environment variable."
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
    pb.set_message("Fetching remote skills...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // List skills directory
    let entries = match client.list_tree("skills") {
        Ok(entries) => entries,
        Err(e) => {
            match e {
                crate::gitlab::GitLabError::AuthError(msg) => {
                    return Err(anyhow::anyhow!("Authentication failed: {}", msg));
                }
                crate::gitlab::GitLabError::NotFound(_) => {
                    println!("No skills found in the repository.");
                    return Ok(());
                }
                _ => {
                    return Err(anyhow::anyhow!("Failed to fetch skills: {}", e));
                }
            }
        }
    };

    if entries.is_empty() {
        println!("No skills found in the repository.");
        return Ok(());
    }

    // Fetch and parse each SKILL.md
    let mut skills = Vec::new();
    let mut errors = Vec::new();

    for entry in entries {
        if entry.entry_type != "tree" {
            continue;
        }

        let skill_md_path = format!("skills/{}/SKILL.md", entry.name);
        match client.fetch_file(&skill_md_path) {
            Ok(content) => {
                match crate::models::skill::parse_skill_md(&content) {
                    Ok(frontmatter) => {
                        let skill = frontmatter.to_skill();
                        skills.push(skill);
                    }
                    Err(e) => {
                        errors.push((entry.name, format!("Failed to parse SKILL.md: {}", e)));
                    }
                }
            }
            Err(crate::gitlab::GitLabError::NotFound(_)) => {
                errors.push((entry.name, "SKILL.md not found".to_string()));
            }
            Err(e) => {
                errors.push((entry.name, format!("Failed to fetch SKILL.md: {}", e)));
            }
        }
    }

    pb.finish_and_clear();

    if skills.is_empty() {
        if !errors.is_empty() {
            eprintln!("Warning: Could not load any skills:");
            for (name, err) in &errors {
                eprintln!("  {}: {}", name, err);
            }
        }
        println!("No skills available.");
        return Ok(());
    }

    if std::io::stdin().is_terminal() {
        // Interactive mode: fuzzy select with names only, then show details
        loop {
            let items: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();

            let selection = FuzzySelect::new()
                .with_prompt("Select a skill (type to filter)")
                .items(&items)
                .interact_opt()?;

            if let Some(index) = selection {
                let skill = &skills[index];
                println!("\nSkill:        {}", skill.name);
                println!("Description:  {}", skill.description);
                println!("Version:      {}", skill.version);

                let options = vec!["Install this skill", "Back to list", "Quit"];
                let action = Select::new()
                    .with_prompt("What would you like to do?")
                    .items(&options)
                    .default(0)
                    .interact_opt()?;

                match action {
                    Some(0) => {
                        // Install
                        if let Err(e) = install_skill(&client, skill) {
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
        render_table(&skills);
    }

    // Show any errors
    if !errors.is_empty() {
        eprintln!("\nWarning: Some skills could not be loaded:");
        for (name, err) in &errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    Ok(())
}

fn render_table(skills: &[Skill]) {
    let name_header = "Skill";
    let ver_header = "Version";

    let name_width = skills
        .iter()
        .map(|s| s.name.len())
        .chain(std::iter::once(name_header.len()))
        .max()
        .unwrap_or(10)
        .max(10);

    let ver_width = skills
        .iter()
        .map(|s| s.version.len())
        .chain(std::iter::once(ver_header.len()))
        .max()
        .unwrap_or(7)
        .max(7);

    println!("Available Skills");
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

    for skill in skills {
        print!("│ {: <width$} ", skill.name, width = name_width);
        print!("│ {: <width$} ", skill.version, width = ver_width);
        println!("│");
    }

    print!("└");
    print!("{}", "─".repeat(name_width + 2));
    print!("┴");
    print!("{}", "─".repeat(ver_width + 2));
    println!("┘");
}

pub fn install_skill(client: &GitLabClient, skill: &Skill) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Installing {}...", skill.name));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    download::download_and_install(client, skill)?;

    pb.finish_and_clear();

    crate::config::add_skill(skill)?;
    gitignore::ensure_gitignore_entries(&skill.name)?;
    codex::create_symlink(&skill.name)?;

    println!(
        "Successfully installed {} v{}",
        skill.name, skill.version
    );

    Ok(())
}

fn resolve_repo_config() -> Result<(String, String, String)> {
    let env_base_url = std::env::var("strand_GITLAB_URL").ok();

    // Try environment variable first
    if let Ok(project) = std::env::var("strand_SKILLS_REPO") {
        let branch = std::env::var("strand_SKILLS_REPO_BRANCH").unwrap_or_else(|_| "main".to_string());
        let base_url = env_base_url.unwrap_or_else(|| "https://gitlab.com".to_string());
        return Ok((project, base_url, branch));
    }

    // Try config file
    if let Ok(config_str) = std::fs::read_to_string(crate::config::CONFIG_PATH) {
        if let Ok(config) = serde_json::from_str::<Config>(&config_str)
            && !config.skills_repo.project.is_empty()
        {
            let branch = if config.skills_repo.branch.is_empty() {
                "main".to_string()
            } else {
                config.skills_repo.branch
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
            return Ok((config.skills_repo.project, base_url, branch));
        }
    }

    let base_url = env_base_url.unwrap_or_else(|| "https://gitlab.com".to_string());
    Ok((String::new(), base_url, "main".to_string()))
}
