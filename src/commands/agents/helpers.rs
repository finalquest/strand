use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::config::{AgentEntry, Config, CONFIG_PATH, TargetConfig};
use crate::gitlab::GitLabClient;
use crate::models::agent::Agent;

/// Download and install an agent from the remote repository.
/// Fetches the agent directory from `agents/{name}` and installs it to `.agents/agents/{name}`.
pub fn download_and_install_agent(client: &GitLabClient, agent: &Agent) -> Result<()> {
    let install_dir = Path::new(".agents/agents").join(&agent.name);
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("Failed to create directory {}", install_dir.display()))?;

    download_directory(client, &format!("agents/{}", agent.name), &install_dir)?;

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

/// Update config.json to include the given agent entry.
/// Removes any existing entry with the same name and appends the new one.
pub fn update_config_with_agent(agent: &Agent) -> Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    if !config_path.exists() {
        return Ok(());
    }

    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
    let mut config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;

    config.agents.retain(|a| a.name != agent.name);

    config.agents.push(AgentEntry {
        name: agent.name.clone(),
        version: agent.version.clone(),
        installed_path: format!(".agents/agents/{}", agent.name),
    });

    let config_json = serde_json::to_string_pretty(&config)
        .with_context(|| format!("Failed to serialize {}", CONFIG_PATH))?;
    fs::write(config_path, config_json)
        .with_context(|| format!("Failed to write {}", CONFIG_PATH))?;

    Ok(())
}

/// Ensure .gitignore contains entries for the agent in:
/// - `.agents/agents/{name}`
/// - `.opencode/agents/{name}`
/// - `.codex/agents/{name}`
pub fn ensure_gitignore_entries_for_agent(agent_name: &str) -> Result<()> {
    let gitignore_path = Path::new(".gitignore");
    let agents_path = format!(".agents/agents/{}", agent_name);
    let opencode_path = format!(".opencode/agents/{}", agent_name);
    let codex_path = format!(".codex/agents/{}", agent_name);

    let mut lines: Vec<String> = if gitignore_path.exists() {
        fs::read_to_string(gitignore_path)
            .with_context(|| "Failed to read .gitignore")?
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut changed = false;

    if !lines.contains(&agents_path) {
        lines.push(agents_path);
        changed = true;
    }

    if !lines.contains(&opencode_path) {
        lines.push(opencode_path);
        changed = true;
    }

    if !lines.contains(&codex_path) {
        lines.push(codex_path);
        changed = true;
    }

    if changed {
        let content = lines.join("\n");
        let content = if content.ends_with('\n') {
            content
        } else {
            format!("{}\n", content)
        };
        fs::write(gitignore_path, content).with_context(|| "Failed to write .gitignore")?;
    }

    Ok(())
}

/// Create symlinks for the agent in target directories based on the target configuration.
/// - `.opencode/agents/{name}` -> `.agents/agents/{name}` (if opencode target enabled)
/// - `.codex/agents/{name}` -> `.agents/agents/{name}` (if codex target enabled)
pub fn create_agent_symlinks(agent_name: &str, targets: &TargetConfig) -> Result<()> {
    let source_path = Path::new(".agents/agents").join(agent_name);

    if targets.opencode {
        create_symlink_for_target(agent_name, &source_path, ".opencode/agents")?;
    }

    if targets.codex {
        create_symlink_for_target(agent_name, &source_path, ".codex/agents")?;
    }

    Ok(())
}

pub struct AgentInstallResult {
    pub succeeded: Vec<(String, String)>,
    pub failed: Vec<(String, String)>,
}

impl AgentInstallResult {
    pub fn is_ok(&self) -> bool {
        self.failed.is_empty()
    }

    pub fn total(&self) -> usize {
        self.succeeded.len() + self.failed.len()
    }
}

/// Install all agents declared by a skill.
/// For each agent: fetch AGENT.md, parse, download, update config, gitignore, symlinks.
/// Errors are non-blocking: failures are collected but don't stop other agents.
pub fn install_skill_agents(
    agent_names: &[String],
    agents_client: &GitLabClient,
    targets: &TargetConfig,
) -> AgentInstallResult {
    let mut succeeded: Vec<(String, String)> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();

    for agent_name in agent_names {
        println!("    Installing agent {}...", agent_name);
        match install_single_agent_for_skill(agents_client, agent_name, targets) {
            Ok(version) => {
                println!("    Installed agent {} v{}", agent_name, version);
                succeeded.push((agent_name.clone(), version));
            }
            Err(e) => {
                eprintln!("    Failed to install agent {}: {}", agent_name, e);
                failed.push((agent_name.clone(), e.to_string()));
            }
        }
    }

    AgentInstallResult {
        succeeded,
        failed,
    }
}

fn install_single_agent_for_skill(
    client: &GitLabClient,
    agent_name: &str,
    targets: &TargetConfig,
) -> Result<String> {
    {
        let config_str = std::fs::read_to_string(CONFIG_PATH).unwrap_or_default();
        let config: Config = serde_json::from_str(&config_str).unwrap_or_default();
        let managed: std::collections::HashSet<String> =
            config.agents.into_iter().map(|a| a.name).collect();
        crate::discovery::check_local_agent_conflict(agent_name, &managed)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    let remote_agent_md = format!("agents/{}/AGENT.md", agent_name);
    let content = client
        .fetch_file(&remote_agent_md)
        .map_err(|e| anyhow::anyhow!("Failed to fetch AGENT.md for {}: {}", agent_name, e))?;
    let agent: Agent = crate::models::agent::parse_agent_md(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse AGENT.md for {}: {}", agent_name, e))?
        .to_agent();

    download_and_install_agent(client, &agent)?;
    update_config_with_agent(&agent)?;
    ensure_gitignore_entries_for_agent(&agent.name)?;
    create_agent_symlinks(&agent.name, targets)?;

    Ok(agent.version)
}

fn create_symlink_for_target(
    agent_name: &str,
    source_path: &Path,
    link_dir: &str,
) -> Result<()> {
    // source_path is like .agents/agents/test-agent
    // We need to pass .agents/agents as target_dir and test-agent as name
    // to create_symlink so it creates link_dir/test-agent -> .agents/agents/test-agent
    let target_dir = source_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Source path has no parent directory"))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Source path is not valid UTF-8"))?;

    crate::symlinks::create_symlink(target_dir, link_dir, agent_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".strand")).unwrap();
        dir
    }

    fn create_test_config(dir: &tempfile::TempDir) {
        let config = Config {
            version: 1,
            targets: TargetConfig {
                opencode: true,
                codex: false,
            },
            skills_repo: crate::config::SkillsRepoConfig {
                provider: "gitlab".to_string(),
                project: "test/project".to_string(),
                branch: "main".to_string(),
                base_url: "https://gitlab.com".to_string(),
            },
            ..Default::default()
        };
        fs::write(
            dir.path().join(".strand/config.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_update_config_with_agent_adds_entry() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();
        create_test_config(&dir);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let agent = Agent {
            name: "test-agent".to_string(),
            description: "A test agent".to_string(),
            version: "1.0.0".to_string(),
        };

        update_config_with_agent(&agent).unwrap();

        let config_str = fs::read_to_string(".strand/config.json").unwrap();
        let config: Config = serde_json::from_str(&config_str).unwrap();

        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "test-agent");
        assert_eq!(config.agents[0].version, "1.0.0");
        assert_eq!(
            config.agents[0].installed_path,
            ".agents/agents/test-agent"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_config_with_agent_replaces_existing() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();
        create_test_config(&dir);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let agent1 = Agent {
            name: "test-agent".to_string(),
            description: "A test agent".to_string(),
            version: "1.0.0".to_string(),
        };
        update_config_with_agent(&agent1).unwrap();

        let agent2 = Agent {
            name: "test-agent".to_string(),
            description: "Updated agent".to_string(),
            version: "2.0.0".to_string(),
        };
        update_config_with_agent(&agent2).unwrap();

        let config_str = fs::read_to_string(".strand/config.json").unwrap();
        let config: Config = serde_json::from_str(&config_str).unwrap();

        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].version, "2.0.0");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_update_config_with_agent_no_config() {
        let agent = Agent {
            name: "test-agent".to_string(),
            description: "A test agent".to_string(),
            version: "1.0.0".to_string(),
        };

        let result = update_config_with_agent(&agent);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_gitignore_entries_for_agent_creates_file() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        ensure_gitignore_entries_for_agent("test-agent").unwrap();

        let content = fs::read_to_string(".gitignore").unwrap();
        assert!(content.contains(".agents/agents/test-agent"));
        assert!(content.contains(".opencode/agents/test-agent"));
        assert!(content.contains(".codex/agents/test-agent"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_ensure_gitignore_entries_for_agent_idempotent() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // First call
        ensure_gitignore_entries_for_agent("test-agent").unwrap();
        let content1 = fs::read_to_string(".gitignore").unwrap();

        // Second call should not duplicate entries
        ensure_gitignore_entries_for_agent("test-agent").unwrap();
        let content2 = fs::read_to_string(".gitignore").unwrap();

        assert_eq!(content1, content2);

        // Count occurrences - should be exactly one per entry
        let lines: Vec<&str> = content2.lines().collect();
        assert_eq!(
            lines.iter().filter(|&&l| l == ".agents/agents/test-agent").count(),
            1
        );
        assert_eq!(
            lines.iter().filter(|&&l| l == ".opencode/agents/test-agent").count(),
            1
        );
        assert_eq!(
            lines.iter().filter(|&&l| l == ".codex/agents/test-agent").count(),
            1
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_ensure_gitignore_entries_for_agent_appends_to_existing() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(".gitignore", "node_modules/\n.env\n").unwrap();

        ensure_gitignore_entries_for_agent("test-agent").unwrap();

        let content = fs::read_to_string(".gitignore").unwrap();
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".env"));
        assert!(content.contains(".agents/agents/test-agent"));
        assert!(content.contains(".opencode/agents/test-agent"));
        assert!(content.contains(".codex/agents/test-agent"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_agent_symlinks_opencode_only() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // Create the source agent directory
        fs::create_dir_all(".agents/agents/test-agent").unwrap();

        let targets = TargetConfig {
            opencode: true,
            codex: false,
        };

        create_agent_symlinks("test-agent", &targets).unwrap();

        let opencode_link = Path::new(".opencode/agents/test-agent");
        assert!(opencode_link.symlink_metadata().is_ok());
        let opencode_target = fs::read_link(opencode_link).unwrap();
        assert_eq!(opencode_target, Path::new(".agents/agents/test-agent"));

        let codex_link = Path::new(".codex/agents/test-agent");
        assert!(!codex_link.exists() && !codex_link.symlink_metadata().is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_agent_symlinks_both_targets() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // Create the source agent directory
        fs::create_dir_all(".agents/agents/test-agent").unwrap();

        let targets = TargetConfig {
            opencode: true,
            codex: true,
        };

        create_agent_symlinks("test-agent", &targets).unwrap();

        let opencode_link = Path::new(".opencode/agents/test-agent");
        assert!(opencode_link.symlink_metadata().is_ok());
        let opencode_target = fs::read_link(opencode_link).unwrap();
        assert_eq!(opencode_target, Path::new(".agents/agents/test-agent"));

        let codex_link = Path::new(".codex/agents/test-agent");
        assert!(codex_link.symlink_metadata().is_ok());
        let codex_target = fs::read_link(codex_link).unwrap();
        assert_eq!(codex_target, Path::new(".agents/agents/test-agent"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_agent_symlinks_no_targets() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // Create the source agent directory
        fs::create_dir_all(".agents/agents/test-agent").unwrap();

        let targets = TargetConfig {
            opencode: false,
            codex: false,
        };

        create_agent_symlinks("test-agent", &targets).unwrap();

        let opencode_link = Path::new(".opencode/agents/test-agent");
        assert!(!opencode_link.exists() && !opencode_link.symlink_metadata().is_ok());

        let codex_link = Path::new(".codex/agents/test-agent");
        assert!(!codex_link.exists() && !codex_link.symlink_metadata().is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_agent_symlinks_idempotent() {
        let _guard = TEST_MUTEX.lock().unwrap();
        let dir = setup_test_dir();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        // Create the source agent directory
        fs::create_dir_all(".agents/agents/test-agent").unwrap();

        let targets = TargetConfig {
            opencode: true,
            codex: true,
        };

        // First call
        create_agent_symlinks("test-agent", &targets).unwrap();

        // Second call should not fail
        create_agent_symlinks("test-agent", &targets).unwrap();

        let opencode_link = Path::new(".opencode/agents/test-agent");
        assert!(opencode_link.symlink_metadata().is_ok());
        let opencode_target = fs::read_link(opencode_link).unwrap();
        assert_eq!(opencode_target, Path::new(".agents/agents/test-agent"));

        let codex_link = Path::new(".codex/agents/test-agent");
        assert!(codex_link.symlink_metadata().is_ok());
        let codex_target = fs::read_link(codex_link).unwrap();
        assert_eq!(codex_target, Path::new(".agents/agents/test-agent"));

        std::env::set_current_dir(original_dir).unwrap();
    }
}
