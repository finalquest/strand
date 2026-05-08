use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const CONFIG_PATH: &str = ".strand/config.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub version: i32,
    pub targets: TargetConfig,
    #[serde(rename = "skillsRepo")]
    pub skills_repo: SkillsRepoConfig,
    pub skills: Vec<SkillEntry>,
    #[serde(rename = "agentsRepo", default, skip_serializing_if = "AgentsRepoConfig::is_empty")]
    pub agents_repo: AgentsRepoConfig,
    #[serde(default)]
    pub agents: Vec<AgentEntry>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TargetConfig {
    pub opencode: bool,
    #[serde(default)]
    pub codex: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SkillsRepoConfig {
    pub provider: String,
    pub project: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_base_url() -> String {
    "https://gitlab.com".to_string()
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SkillEntry {
    pub name: String,
    pub version: String,
    #[serde(rename = "installedPath")]
    pub installed_path: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AgentsRepoConfig {
    pub provider: String,
    pub project: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
}

impl AgentsRepoConfig {
    pub fn is_empty(&self) -> bool {
        self.provider.is_empty() && self.project.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AgentEntry {
    pub name: String,
    pub version: String,
    #[serde(rename = "installedPath")]
    pub installed_path: String,
}

impl Config {
    /// Resolve agents repository configuration, applying environment variable overrides.
    ///
    /// Environment variables:
    /// - `strand_AGENTS_REPO`: Overrides `agentsRepo.project`
    /// - `strand_AGENTS_REPO_BRANCH`: Overrides `agentsRepo.branch`
    /// - `strand_GITLAB_URL`: Overrides the base URL
    pub fn resolve_agents_repo(&self) -> (String, String, String) {
        let env_base_url = std::env::var("strand_GITLAB_URL").ok();

        if let Some(project) = crate::env::agents_repo_project() {
            let branch = crate::env::agents_repo_branch();
            let base_url = env_base_url.unwrap_or_else(|| "https://gitlab.com".to_string());
            return (project, base_url, branch);
        }

        if !self.agents_repo.project.is_empty() {
            let branch = if std::env::var("strand_AGENTS_REPO_BRANCH").is_ok() {
                crate::env::agents_repo_branch()
            } else if self.agents_repo.branch.is_empty() {
                "main".to_string()
            } else {
                self.agents_repo.branch.clone()
            };
            let base_url = env_base_url
                .or_else(|| {
                    if self.agents_repo.base_url.is_empty() {
                        None
                    } else {
                        Some(self.agents_repo.base_url.clone())
                    }
                })
                .unwrap_or_else(|| "https://gitlab.com".to_string());
            return (self.agents_repo.project.clone(), base_url, branch);
        }

        let base_url = env_base_url.unwrap_or_else(|| "https://gitlab.com".to_string());
        (String::new(), base_url, "main".to_string())
    }
}

pub fn add_skill(skill: &crate::models::skill::Skill) -> Result<()> {
    let config_path = Path::new(CONFIG_PATH);
    if !config_path.exists() {
        return Ok(());
    }

    let config_str = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read {}", CONFIG_PATH))?;
    let mut config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse {}", CONFIG_PATH))?;

    config.skills.retain(|s| s.name != skill.name);

    config.skills.push(SkillEntry {
        name: skill.name.clone(),
        version: skill.version.clone(),
        installed_path: format!(".agents/skills/{}", skill.name),
    });

    let config_json = serde_json::to_string_pretty(&config)
        .with_context(|| format!("Failed to serialize {}", CONFIG_PATH))?;
    fs::write(config_path, config_json)
        .with_context(|| format!("Failed to write {}", CONFIG_PATH))?;

    Ok(())
}

pub fn add_agent(agent: &crate::models::agent::Agent) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    static DIR_LOCK: Mutex<()> = Mutex::new(());

    fn setup_test_config(json: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".strand");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("config.json"), json).unwrap();
        dir
    }

    #[test]
    fn test_config_backward_compatibility_missing_agent_fields() {
        let json = r#"{
            "version": 1,
            "targets": { "opencode": true },
            "skillsRepo": { "provider": "gitlab", "project": "test" },
            "skills": []
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.agents_repo.provider.is_empty());
        assert!(config.agents.is_empty());
    }

    #[test]
    fn test_config_deserialization_with_agent_fields() {
        let json = r#"{
            "version": 1,
            "targets": { "opencode": true },
            "skillsRepo": { "provider": "gitlab", "project": "test" },
            "skills": [],
            "agentsRepo": { "provider": "gitlab", "project": "agents" },
            "agents": [{ "name": "test-agent", "version": "1.0.0", "installedPath": ".agents/agents/test-agent" }]
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.agents_repo.provider, "gitlab");
        assert_eq!(config.agents_repo.project, "agents");
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "test-agent");
        assert_eq!(config.agents[0].version, "1.0.0");
        assert_eq!(config.agents[0].installed_path, ".agents/agents/test-agent");
    }

    #[test]
    fn test_add_agent_inserts_entry() {
        let _guard = DIR_LOCK.lock().unwrap();
        let dir = setup_test_config(r#"{
            "version": 1,
            "targets": { "opencode": true },
            "skillsRepo": { "provider": "gitlab", "project": "test" },
            "skills": []
        }"#);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let agent = crate::models::agent::Agent {
            name: "my-agent".to_string(),
            description: "desc".to_string(),
            version: "1.0.0".to_string(),
        };

        add_agent(&agent).unwrap();

        let config_str = fs::read_to_string(".strand/config.json").unwrap();
        let config: Config = serde_json::from_str(&config_str).unwrap();

        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "my-agent");
        assert_eq!(config.agents[0].version, "1.0.0");
        assert_eq!(config.agents[0].installed_path, ".agents/agents/my-agent");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_add_agent_replaces_existing() {
        let _guard = DIR_LOCK.lock().unwrap();
        let dir = setup_test_config(r#"{
            "version": 1,
            "targets": { "opencode": true },
            "skillsRepo": { "provider": "gitlab", "project": "test" },
            "skills": [],
            "agentsRepo": { "provider": "gitlab", "project": "agents" },
            "agents": [{ "name": "my-agent", "version": "0.0.1", "installedPath": ".agents/agents/my-agent" }]
        }"#);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let agent = crate::models::agent::Agent {
            name: "my-agent".to_string(),
            description: "desc".to_string(),
            version: "2.0.0".to_string(),
        };

        add_agent(&agent).unwrap();

        let config_str = fs::read_to_string(".strand/config.json").unwrap();
        let config: Config = serde_json::from_str(&config_str).unwrap();

        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].version, "2.0.0");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_add_agent_no_config_file() {
        let agent = crate::models::agent::Agent {
            name: "my-agent".to_string(),
            description: "desc".to_string(),
            version: "1.0.0".to_string(),
        };

        let result = add_agent(&agent);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_agents_repo_env_override() {
        let _guard = crate::env::ENV_MUTEX.lock().unwrap();
        let old_project = std::env::var("strand_AGENTS_REPO").ok();
        let old_branch = std::env::var("strand_AGENTS_REPO_BRANCH").ok();

        unsafe {
            std::env::set_var("strand_AGENTS_REPO", "env/project");
            std::env::set_var("strand_AGENTS_REPO_BRANCH", "env_branch");
        }

        let config = Config {
            version: 1,
            agents_repo: AgentsRepoConfig {
                provider: "gitlab".to_string(),
                project: "config/project".to_string(),
                branch: "config_branch".to_string(),
                base_url: "https://gitlab.config.net".to_string(),
            },
            ..Default::default()
        };

        let (project, base_url, branch) = config.resolve_agents_repo();
        assert_eq!(project, "env/project");
        assert_eq!(branch, "env_branch");
        assert_eq!(base_url, "https://gitlab.com");

        match old_project {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO") },
        }
        match old_branch {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO_BRANCH", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO_BRANCH") },
        }
    }

    #[test]
    fn test_resolve_agents_repo_branch_only_override() {
        let _guard = crate::env::ENV_MUTEX.lock().unwrap();
        let old_project = std::env::var("strand_AGENTS_REPO").ok();
        let old_branch = std::env::var("strand_AGENTS_REPO_BRANCH").ok();

        unsafe {
            std::env::remove_var("strand_AGENTS_REPO");
            std::env::set_var("strand_AGENTS_REPO_BRANCH", "env_branch_only");
        }

        let config = Config {
            version: 1,
            agents_repo: AgentsRepoConfig {
                provider: "gitlab".to_string(),
                project: "config/project".to_string(),
                branch: "config_branch".to_string(),
                base_url: "https://gitlab.config.net".to_string(),
            },
            ..Default::default()
        };

        let (project, base_url, branch) = config.resolve_agents_repo();
        assert_eq!(project, "config/project");
        assert_eq!(branch, "env_branch_only");
        assert_eq!(base_url, "https://gitlab.config.net");

        match old_project {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO") },
        }
        match old_branch {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO_BRANCH", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO_BRANCH") },
        }
    }

    #[test]
    fn test_resolve_agents_repo_from_config() {
        let _guard = crate::env::ENV_MUTEX.lock().unwrap();
        let old_project = std::env::var("strand_AGENTS_REPO").ok();
        let old_branch = std::env::var("strand_AGENTS_REPO_BRANCH").ok();

        unsafe {
            std::env::remove_var("strand_AGENTS_REPO");
            std::env::remove_var("strand_AGENTS_REPO_BRANCH");
        }

        let config = Config {
            version: 1,
            agents_repo: AgentsRepoConfig {
                provider: "gitlab".to_string(),
                project: "config/project".to_string(),
                branch: "config_branch".to_string(),
                base_url: "https://gitlab.config.net".to_string(),
            },
            ..Default::default()
        };

        let (project, base_url, branch) = config.resolve_agents_repo();
        assert_eq!(project, "config/project");
        assert_eq!(branch, "config_branch");
        assert_eq!(base_url, "https://gitlab.config.net");

        match old_project {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO") },
        }
        match old_branch {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO_BRANCH", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO_BRANCH") },
        }
    }

    #[test]
    fn test_resolve_agents_repo_defaults() {
        let _guard = crate::env::ENV_MUTEX.lock().unwrap();
        let old_project = std::env::var("strand_AGENTS_REPO").ok();
        let old_branch = std::env::var("strand_AGENTS_REPO_BRANCH").ok();

        unsafe {
            std::env::remove_var("strand_AGENTS_REPO");
            std::env::remove_var("strand_AGENTS_REPO_BRANCH");
        }

        let config = Config::default();

        let (project, base_url, branch) = config.resolve_agents_repo();
        assert!(project.is_empty());
        assert_eq!(branch, "main");
        assert_eq!(base_url, "https://gitlab.com");

        match old_project {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO") },
        }
        match old_branch {
            Some(v) => unsafe { std::env::set_var("strand_AGENTS_REPO_BRANCH", &v) },
            None => unsafe { std::env::remove_var("strand_AGENTS_REPO_BRANCH") },
        }
    }
}
