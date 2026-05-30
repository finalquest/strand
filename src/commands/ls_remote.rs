use anyhow::Result;
use dialoguer::{FuzzySelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;

use crate::config::Config;
use crate::download;
use crate::gitignore;
use crate::gitlab::GitLabClient;
use crate::models::pack::Pack;
use crate::models::skill::Skill;

enum RemoteItem {
    Skill(Skill),
    Pack(Pack),
}

pub fn execute() -> Result<()> {
    let (project, base_url, branch) = resolve_repo_config()?;

    if project.is_empty() {
        println!("No skills repository configured.");
        println!(
            "Run 'strand init' to initialize configuration or set the strand_SKILLS_REPO environment variable."
        );
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
    pb.set_message("Fetching remote skills and packs...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut skills = Vec::new();
    let mut packs = Vec::new();
    let mut errors = Vec::new();

    discover_skills(&client, &mut skills, &mut errors);
    discover_packs(&client, &mut packs, &mut errors);

    pb.finish_and_clear();

    if skills.is_empty() && packs.is_empty() {
        if !errors.is_empty() {
            eprintln!("Warning: Could not load any skills or packs:");
            for (name, err) in &errors {
                eprintln!("  {}: {}", name, err);
            }
        }
        println!("No skills or packs available.");
        return Ok(());
    }

    if std::io::stdin().is_terminal() {
        interactive_mode(&client, &skills, &packs)?;
    } else {
        non_interactive_mode(&skills, &packs);
    }

    if !errors.is_empty() {
        eprintln!("\nWarning: Some items could not be loaded:");
        for (name, err) in &errors {
            eprintln!("  {}: {}", name, err);
        }
    }

    Ok(())
}

fn discover_skills(
    client: &GitLabClient,
    skills: &mut Vec<Skill>,
    errors: &mut Vec<(String, String)>,
) {
    let entries = match client.list_tree("skills") {
        Ok(e) => e,
        Err(crate::gitlab::GitLabError::NotFound(_)) => return,
        Err(e) => {
            errors.push(("skills/".to_string(), format!("Failed to list: {}", e)));
            return;
        }
    };

    for entry in entries {
        if entry.entry_type != "tree" {
            continue;
        }

        let standalone_path = format!("skills/{}/SKILL.md", entry.name);
        match client.fetch_file(&standalone_path) {
            Ok(content) => {
                match crate::models::skill::parse_skill_md(&content) {
                    Ok(frontmatter) => {
                        skills.push(frontmatter.to_skill());
                    }
                    Err(e) => {
                        errors.push((entry.name.clone(), format!("Failed to parse SKILL.md: {}", e)));
                    }
                }
                continue;
            }
            Err(crate::gitlab::GitLabError::NotFound(_)) => {}
            Err(e) => {
                errors.push((entry.name.clone(), format!("Failed to fetch SKILL.md: {}", e)));
                continue;
            }
        }

        let sub_entries = match client.list_tree(&format!("skills/{}", entry.name)) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for sub_entry in sub_entries {
            if sub_entry.entry_type != "tree" {
                continue;
            }

            let skill_path = format!("skills/{}/{}", entry.name, sub_entry.name);
            let skill_md_path = format!("{}/SKILL.md", skill_path);
            match client.fetch_file(&skill_md_path) {
                Ok(content) => {
                    match crate::models::skill::parse_skill_md(&content) {
                        Ok(frontmatter) => {
                            skills.push(frontmatter.to_skill());
                        }
                        Err(e) => {
                            errors.push((
                                format!("{}/{}", entry.name, sub_entry.name),
                                format!("Failed to parse SKILL.md: {}", e),
                            ));
                        }
                    }
                }
                Err(crate::gitlab::GitLabError::NotFound(_)) => {
                    errors.push((
                        format!("{}/{}", entry.name, sub_entry.name),
                        "SKILL.md not found".to_string(),
                    ));
                }
                Err(e) => {
                    errors.push((
                        format!("{}/{}", entry.name, sub_entry.name),
                        format!("Failed to fetch SKILL.md: {}", e),
                    ));
                }
            }
        }
    }
}

fn discover_packs(
    client: &GitLabClient,
    packs: &mut Vec<Pack>,
    errors: &mut Vec<(String, String)>,
) {
    let entries = match client.list_tree("packs") {
        Ok(e) => e,
        Err(crate::gitlab::GitLabError::NotFound(_)) => return,
        Err(e) => {
            errors.push(("packs/".to_string(), format!("Failed to list: {}", e)));
            return;
        }
    };

    for entry in entries {
        if entry.entry_type != "tree" {
            continue;
        }

        let pack_md_path = format!("packs/{}/pack.md", entry.name);
        match client.fetch_file(&pack_md_path) {
            Ok(content) => {
                match crate::models::pack::parse_pack_md(&content) {
                    Ok(frontmatter) => {
                        packs.push(frontmatter.to_pack());
                    }
                    Err(e) => {
                        errors.push((entry.name.clone(), format!("Failed to parse pack.md: {}", e)));
                    }
                }
            }
            Err(crate::gitlab::GitLabError::NotFound(_)) => {
                errors.push((entry.name.clone(), "pack.md not found".to_string()));
            }
            Err(e) => {
                errors.push((entry.name.clone(), format!("Failed to fetch pack.md: {}", e)));
            }
        }
    }
}

fn interactive_mode(client: &GitLabClient, skills: &[Skill], packs: &[Pack]) -> Result<()> {
    let mut items: Vec<RemoteItem> = Vec::new();

    for skill in skills {
        items.push(RemoteItem::Skill(skill.clone()));
    }
    for pack in packs {
        items.push(RemoteItem::Pack(pack.clone()));
    }

    loop {
        let labels: Vec<String> = items
            .iter()
            .map(|item| match item {
                RemoteItem::Skill(s) => format!("[skill] {}", s.name),
                RemoteItem::Pack(p) => format!("[pack]  {} ({} skills)", p.name, p.skills.len()),
            })
            .collect();

        let selection = FuzzySelect::new()
            .with_prompt("Select an item (type to filter)")
            .items(&labels)
            .interact_opt()?;

        if let Some(index) = selection {
            match &items[index] {
                RemoteItem::Skill(skill) => {
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
                            if let Err(e) = install_skill(client, skill) {
                                eprintln!("Installation failed: {}", e);
                            }
                            break;
                        }
                        Some(1) => continue,
                        _ => break,
                    }
                }
                RemoteItem::Pack(pack) => {
                    println!("\nPack:         {}", pack.name);
                    println!("Description:  {}", pack.description);
                    println!("Skills ({}):", pack.skills.len());
                    for skill_path in &pack.skills {
                        println!("  - {}", skill_path);
                    }

                    let options = vec!["Install this pack", "Back to list", "Quit"];
                    let action = Select::new()
                        .with_prompt("What would you like to do?")
                        .items(&options)
                        .default(0)
                        .interact_opt()?;

                    match action {
                        Some(0) => {
                            if let Err(e) = install_pack(client, pack) {
                                eprintln!("Pack installation failed: {}", e);
                            }
                            break;
                        }
                        Some(1) => continue,
                        _ => break,
                    }
                }
            }
        } else {
            break;
        }
    }

    Ok(())
}

fn non_interactive_mode(skills: &[Skill], packs: &[Pack]) {
    if !skills.is_empty() {
        render_skills_table(skills);
    }

    if !packs.is_empty() {
        if !skills.is_empty() {
            println!();
        }
        render_packs_table(packs);
    }
}

fn render_skills_table(skills: &[Skill]) {
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

fn render_packs_table(packs: &[Pack]) {
    let name_header = "Pack";
    let count_header = "Skills";

    let name_width = packs
        .iter()
        .map(|p| p.name.len())
        .chain(std::iter::once(name_header.len()))
        .max()
        .unwrap_or(10)
        .max(10);

    let count_width = count_header.len().max(6);

    println!("Available Packs");
    println!();

    print!("┌");
    print!("{}", "─".repeat(name_width + 2));
    print!("┬");
    print!("{}", "─".repeat(count_width + 2));
    println!("┐");

    print!("│ {: <width$} ", name_header, width = name_width);
    print!("│ {: <width$} ", count_header, width = count_width);
    println!("│");

    print!("├");
    print!("{}", "─".repeat(name_width + 2));
    print!("┼");
    print!("{}", "─".repeat(count_width + 2));
    println!("┤");

    for pack in packs {
        print!("│ {: <width$} ", pack.name, width = name_width);
        print!("│ {: <width$} ", pack.skills.len(), width = count_width);
        println!("│");
    }

    print!("└");
    print!("{}", "─".repeat(name_width + 2));
    print!("┴");
    print!("{}", "─".repeat(count_width + 2));
    println!("┘");
}

pub fn install_skill(client: &GitLabClient, skill: &Skill) -> Result<()> {
    {
        let config_str = std::fs::read_to_string(crate::config::CONFIG_PATH).unwrap_or_default();
        let config: crate::config::Config = serde_json::from_str(&config_str).unwrap_or_default();
        let managed: std::collections::HashSet<String> =
            config.skills.into_iter().map(|s| s.name).collect();
        crate::discovery::check_local_skill_conflict(&skill.name, &managed)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }

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
    crate::codex::create_symlink(&skill.name)?;

    println!(
        "Successfully installed {} v{}",
        skill.name, skill.version
    );

    if !skill.agents.is_empty() {
        println!(
            "  {} requires {} agent(s), installing...",
            skill.name,
            skill.agents.len()
        );

        if let Ok(config_str) = std::fs::read_to_string(crate::config::CONFIG_PATH) {
            if let Ok(config) = serde_json::from_str::<Config>(&config_str) {
                let (agents_project, agents_base_url, agents_branch) =
                    config.resolve_agents_repo();

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
        }
    }

    Ok(())
}

fn install_pack(client: &GitLabClient, pack: &Pack) -> Result<()> {
    println!("Installing pack '{}' ({} skills)...", pack.name, pack.skills.len());

    let mut installed = 0;
    let mut _skipped = 0;
    let mut errors = 0;

    for skill_path in &pack.skills {
        let skill_md_path = format!("skills/{}/SKILL.md", skill_path);
        let content = match client.fetch_file(&skill_md_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  Error fetching {}: {}", skill_path, e);
                errors += 1;
                continue;
            }
        };

        let skill = match crate::models::skill::parse_skill_md(&content) {
            Ok(f) => f.to_skill(),
            Err(e) => {
                eprintln!("  Error parsing {}: {}", skill_path, e);
                errors += 1;
                continue;
            }
        };

        println!("  Installing {} v{}...", skill.name, skill.version);

        let skill_path_owned = format!("skills/{}", skill_path);
        download::download_and_install_from_path(client, &skill, &skill_path_owned)?;
        crate::config::add_skill_with_remote_path(&skill, skill_path)?;
        gitignore::ensure_gitignore_entries(&skill.name)?;
        crate::codex::create_symlink(&skill.name)?;

        if !skill.agents.is_empty() {
            if let Ok(config_str) = std::fs::read_to_string(crate::config::CONFIG_PATH) {
                if let Ok(config) = serde_json::from_str::<Config>(&config_str) {
                    let (agents_project, agents_base_url, agents_branch) =
                        config.resolve_agents_repo();

                    if !agents_project.is_empty() {
                        if let Ok(agents_client) =
                            GitLabClient::for_project(agents_base_url, agents_project)
                        {
                            let agents_client = agents_client.with_branch(&agents_branch);
                            crate::commands::agents::helpers::install_skill_agents(
                                &skill.agents,
                                &agents_client,
                                &config.targets,
                            );
                        }
                    }
                }
            }
        }

        installed += 1;
    }

    crate::config::add_pack(pack)?;

    println!(
        "\nPack '{}' installed: {} skills installed, {} errors",
        pack.name, installed, errors
    );

    Ok(())
}

fn resolve_repo_config() -> Result<(String, String, String)> {
    let env_base_url = std::env::var("strand_GITLAB_URL").ok();

    if let Ok(project) = std::env::var("strand_SKILLS_REPO") {
        let branch = std::env::var("strand_SKILLS_REPO_BRANCH").unwrap_or_else(|_| "main".to_string());
        let base_url = env_base_url.unwrap_or_else(|| "https://gitlab.com".to_string());
        return Ok((project, base_url, branch));
    }

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
