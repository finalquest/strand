use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::codex;
use crate::config::{Config, CONFIG_PATH, SkillEntry};
use crate::download;
use crate::gitignore;
use crate::gitlab::GitLabClient;
use crate::models::skill::Skill;

pub struct InstallOptions {
    pub dry_run: bool,
}

pub fn execute(options: InstallOptions) -> Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    if !config_path.exists() {
        println!("No configuration found. Run 'strand init' first.");
        return Ok(());
    }

    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
    let config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;

    if config.skills.is_empty() {
        println!("No skills configured.");
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

    let mut updated = 0;
    let mut errors = 0;
    let mut up_to_date = 0;

    for skill_entry in &config.skills {
        match install_skill(&client, &config, skill_entry, &options) {
            Ok(true) => updated += 1,
            Ok(false) => up_to_date += 1,
            Err(e) => {
                eprintln!("Error installing {}: {}", skill_entry.name, e);
                errors += 1;
            }
        }
    }

    if options.dry_run {
        println!("\nDry run complete.");
    } else {
        println!("\nInstall complete.");
    }
    println!("  Up to date: {}", up_to_date);
    println!("  Installed: {}", updated);
    println!("  Errors: {}", errors);

    Ok(())
}

fn install_skill(
    client: &GitLabClient,
    config: &Config,
    skill_entry: &SkillEntry,
    options: &InstallOptions,
) -> Result<bool> {
    let installed_path = Path::new(&skill_entry.installed_path);
    let skill_md_path = installed_path.join("SKILL.md");

    let needs_update = if !installed_path.exists() || !skill_md_path.exists() {
        true
    } else {
        match fs::read_to_string(&skill_md_path) {
            Ok(content) => match crate::models::skill::parse_skill_md(&content) {
                Ok(frontmatter) => frontmatter.metadata.version != skill_entry.version,
                Err(_) => {
                    println!("    {} SKILL.md is corrupted, will reinstall", skill_entry.name);
                    true
                }
            },
            Err(_) => {
                println!("    {} SKILL.md is unreadable, will reinstall", skill_entry.name);
                true
            }
        }
    };

    if !needs_update {
        println!("  {} v{} is up to date", skill_entry.name, skill_entry.version);
        return Ok(false);
    }

    if options.dry_run {
        println!(
            "  {} v{} would be reinstalled",
            skill_entry.name, skill_entry.version
        );
        return Ok(true);
    }

    println!(
        "  Reinstalling {} v{}...",
        skill_entry.name, skill_entry.version
    );

    let remote_skill_md = format!("skills/{}/SKILL.md", skill_entry.name);
    let content = client
        .fetch_file(&remote_skill_md)
        .map_err(|e| anyhow::anyhow!("Failed to fetch SKILL.md: {}", e))?;
    let skill: Skill = crate::models::skill::parse_skill_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse SKILL.md: {}", e))?
        .to_skill();

    if skill.version != skill_entry.version {
        return Err(anyhow::anyhow!(
            "Version mismatch: config pins {} v{} but remote has v{}",
            skill_entry.name,
            skill_entry.version,
            skill.version
        ));
    }

    download::download_and_install(client, &skill)?;
    gitignore::ensure_gitignore_entries(&skill.name)?;

    if config.targets.codex {
        codex::create_symlink(&skill.name)?;
    }

    println!(
        "  Successfully reinstalled {} v{}",
        skill.name, skill.version
    );

    Ok(true)
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

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn dummy_client() -> crate::gitlab::GitLabClient {
        use crate::gitlab::{GitLabClient, Transport};
        struct DummyTransport;
        impl Transport for DummyTransport {
            fn call(&self, _endpoint: &str) -> Result<(u16, String), crate::gitlab::GitLabError> {
                Ok((200, "{}".to_string()))
            }
        }
        GitLabClient::with_transport(Box::new(DummyTransport), "test/project".to_string())
    }

    fn create_test_skill_dir(base: &Path, name: &str, version: &str) {
        let skill_dir = base.join(".agents/skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_md = format!(
            "---\nname: {}\ndescription: test\nmetadata:\n  version: {}\n---\n",
            name, version
        );
        fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();
    }

    fn create_test_config(base: &Path, skills: Vec<(&str, &str)>) {
        let config = Config {
            version: 1,
            targets: crate::config::TargetConfig {
                opencode: true,
                codex: false,
            },
            skills_repo: crate::config::SkillsRepoConfig {
                provider: "gitlab".to_string(),
                project: "test/project".to_string(),
                branch: "main".to_string(),
                base_url: "https://gitlab.com".to_string(),
            },
            skills: skills
                .into_iter()
                .map(|(name, version)| SkillEntry {
                    name: name.to_string(),
                    version: version.to_string(),
                    installed_path: format!(".agents/skills/{}", name),
                })
                .collect(),
        };
        fs::write(
            base.join(".strand/config.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_install_skill_up_to_date() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("strand_test_install_uptodate");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        fs::create_dir_all(".strand").unwrap();
        fs::create_dir_all(".agents/skills").unwrap();
        create_test_config(&temp_dir, vec![("test-skill", "1.0.0")]);
        create_test_skill_dir(&temp_dir, "test-skill", "1.0.0");

        let skill_entry = SkillEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/skills/test-skill".to_string(),
        };

        let result = install_skill(
            &dummy_client(),
            &Config::default(),
            &skill_entry,
            &InstallOptions { dry_run: false },
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_install_skill_missing() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("strand_test_install_missing");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        fs::create_dir_all(".strand").unwrap();
        fs::create_dir_all(".agents/skills").unwrap();
        create_test_config(&temp_dir, vec![("test-skill", "1.0.0")]);

        let skill_entry = SkillEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/skills/test-skill".to_string(),
        };

        let result = install_skill(
            &dummy_client(),
            &Config::default(),
            &skill_entry,
            &InstallOptions { dry_run: true },
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_install_skill_version_mismatch() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("strand_test_install_mismatch");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        fs::create_dir_all(".strand").unwrap();
        fs::create_dir_all(".agents/skills").unwrap();
        create_test_config(&temp_dir, vec![("test-skill", "2.0.0")]);
        create_test_skill_dir(&temp_dir, "test-skill", "1.0.0");

        let skill_entry = SkillEntry {
            name: "test-skill".to_string(),
            version: "2.0.0".to_string(),
            installed_path: ".agents/skills/test-skill".to_string(),
        };

        let result = install_skill(
            &dummy_client(),
            &Config::default(),
            &skill_entry,
            &InstallOptions { dry_run: true },
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_install_idempotent() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("strand_test_install_idempotent");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        fs::create_dir_all(".strand").unwrap();
        fs::create_dir_all(".agents/skills").unwrap();
        create_test_config(&temp_dir, vec![("test-skill", "1.0.0")]);
        create_test_skill_dir(&temp_dir, "test-skill", "1.0.0");

        let skill_entry = SkillEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/skills/test-skill".to_string(),
        };

        // First call
        let result1 = install_skill(
            &dummy_client(),
            &Config::default(),
            &skill_entry,
            &InstallOptions { dry_run: false },
        );

        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), false);

        // Second call should also report up to date
        let result2 = install_skill(
            &dummy_client(),
            &Config::default(),
            &skill_entry,
            &InstallOptions { dry_run: false },
        );

        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), false);

        std::env::set_current_dir(original_dir).unwrap();
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
