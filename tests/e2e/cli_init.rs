use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

static INIT_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct CwdGuard(std::path::PathBuf);
impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

fn setup_init_test() -> (TempDir, CwdGuard, std::sync::MutexGuard<'static, ()>) {
    let guard = INIT_MUTEX.lock().unwrap();
    let temp = TempDir::new().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp).unwrap();
    (temp, CwdGuard(original_dir), guard)
}

#[test]
fn test_init_creates_config() {
    let (_temp, _cwd, _guard) = setup_init_test();

    // Clean env
    unsafe {
        std::env::remove_var("strand_INIT_PROJECT");
        std::env::remove_var("strand_INIT_BRANCH");
        std::env::remove_var("strand_INIT_CODEX");
    }

    strand::commands::init::init().unwrap();

    assert!(Path::new(".strand/config.json").exists());
    let config_str = fs::read_to_string(".strand/config.json").unwrap();
    let config: strand::config::Config = serde_json::from_str(&config_str).unwrap();
    assert_eq!(config.version, 1);
    assert!(config.targets.opencode);
    assert!(!config.targets.codex);
    assert_eq!(config.skills_repo.provider, "gitlab");
    assert!(config.skills_repo.project.is_empty());
    assert_eq!(config.skills_repo.branch, "main");
    assert!(
        !config.skills_repo.base_url.is_empty(),
        "base_url should be set (default or inferred)"
    );
    assert!(config.skills.is_empty());
}

#[test]
fn test_init_rewrites_config_preserving_skills() {
    let (_temp, _cwd, _guard) = setup_init_test();

    fs::create_dir_all(".strand").unwrap();
    let old_config = strand::config::Config {
        version: 1,
        targets: strand::config::TargetConfig {
            opencode: true,
            codex: false,
        },
        skills_repo: strand::config::SkillsRepoConfig {
            provider: "gitlab".to_string(),
            project: "old/project".to_string(),
            branch: "old_branch".to_string(),
            base_url: "https://gitlab.example.com".to_string(),
        },
        skills: vec![strand::config::SkillEntry {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/skills/test-skill".to_string(),
        }],
        ..Default::default()
    };
    fs::write(
        ".strand/config.json",
        serde_json::to_string_pretty(&old_config).unwrap(),
    )
    .unwrap();

    // Run init with new data via env vars
    unsafe {
        std::env::set_var("strand_INIT_PROJECT", "new/project");
        std::env::set_var("strand_INIT_BRANCH", "new_branch");
        std::env::set_var("strand_INIT_CODEX", "true");
        std::env::set_var("strand_INIT_BASE_URL", "https://gitlab.newhost.net");
    }

    strand::commands::init::init().unwrap();

    let config_str = fs::read_to_string(".strand/config.json").unwrap();
    let config: strand::config::Config = serde_json::from_str(&config_str).unwrap();

    // New data should be written
    assert_eq!(config.skills_repo.project, "new/project");
    assert_eq!(config.skills_repo.branch, "new_branch");
    assert_eq!(config.skills_repo.base_url, "https://gitlab.newhost.net");
    assert!(config.targets.codex);
    // Skills should be preserved
    assert_eq!(config.skills.len(), 1);
    assert_eq!(config.skills[0].name, "test-skill");

    // Clean up env vars
    unsafe {
        std::env::remove_var("strand_INIT_PROJECT");
        std::env::remove_var("strand_INIT_BRANCH");
        std::env::remove_var("strand_INIT_CODEX");
        std::env::remove_var("strand_INIT_BASE_URL");
    }
}

#[test]
fn test_init_creates_agents_directory() {
    let (_temp, _cwd, _guard) = setup_init_test();

    unsafe {
        std::env::remove_var("strand_INIT_PROJECT");
        std::env::remove_var("strand_INIT_BRANCH");
        std::env::remove_var("strand_INIT_CODEX");
        std::env::remove_var("strand_INIT_AGENTS_PROJECT");
        std::env::remove_var("strand_INIT_AGENTS_BRANCH");
    }

    strand::commands::init::init().unwrap();

    assert!(Path::new(".agents/agents").exists());
}

#[test]
fn test_init_creates_agent_target_directories() {
    let (_temp, _cwd, _guard) = setup_init_test();

    unsafe {
        std::env::remove_var("strand_INIT_PROJECT");
        std::env::remove_var("strand_INIT_BRANCH");
        std::env::set_var("strand_INIT_CODEX", "true");
        std::env::remove_var("strand_INIT_AGENTS_PROJECT");
        std::env::remove_var("strand_INIT_AGENTS_BRANCH");
    }

    strand::commands::init::init().unwrap();

    assert!(Path::new(".opencode/agents").exists());
    assert!(Path::new(".codex/agents").exists());

    unsafe {
        std::env::remove_var("strand_INIT_CODEX");
    }
}

#[test]
fn test_init_agents_repo_not_written_when_empty() {
    let (_temp, _cwd, _guard) = setup_init_test();

    unsafe {
        std::env::remove_var("strand_INIT_PROJECT");
        std::env::remove_var("strand_INIT_BRANCH");
        std::env::remove_var("strand_INIT_CODEX");
        std::env::remove_var("strand_INIT_AGENTS_PROJECT");
        std::env::remove_var("strand_INIT_AGENTS_BRANCH");
    }

    strand::commands::init::init().unwrap();

    let config_str = fs::read_to_string(".strand/config.json").unwrap();
    assert!(!config_str.contains("agentsRepo"));
}

#[test]
fn test_init_agents_repo_written_when_provided() {
    let (_temp, _cwd, _guard) = setup_init_test();

    unsafe {
        std::env::remove_var("strand_INIT_PROJECT");
        std::env::remove_var("strand_INIT_BRANCH");
        std::env::remove_var("strand_INIT_CODEX");
        std::env::set_var("strand_INIT_AGENTS_PROJECT", "my/agents");
        std::env::set_var("strand_INIT_AGENTS_BRANCH", "develop");
    }

    strand::commands::init::init().unwrap();

    let config_str = fs::read_to_string(".strand/config.json").unwrap();
    assert!(config_str.contains("agentsRepo"));
    let config: strand::config::Config = serde_json::from_str(&config_str).unwrap();
    assert_eq!(config.agents_repo.project, "my/agents");
    assert_eq!(config.agents_repo.branch, "develop");

    unsafe {
        std::env::remove_var("strand_INIT_AGENTS_PROJECT");
        std::env::remove_var("strand_INIT_AGENTS_BRANCH");
    }
}

#[test]
fn test_init_preserves_agents_and_creates_symlinks() {
    let (_temp, _cwd, _guard) = setup_init_test();

    fs::create_dir_all(".strand").unwrap();
    let old_config = strand::config::Config {
        version: 1,
        targets: strand::config::TargetConfig {
            opencode: true,
            codex: false,
        },
        skills_repo: strand::config::SkillsRepoConfig {
            provider: "gitlab".to_string(),
            project: "old/project".to_string(),
            branch: "old_branch".to_string(),
            base_url: "https://gitlab.example.com".to_string(),
        },
        skills: vec![],
        agents_repo: strand::config::AgentsRepoConfig {
            provider: "gitlab".to_string(),
            project: "old/agents".to_string(),
            branch: "main".to_string(),
            base_url: "https://gitlab.example.com".to_string(),
        },
        agents: vec![strand::config::AgentEntry {
            name: "test-agent".to_string(),
            version: "1.0.0".to_string(),
            installed_path: ".agents/agents/test-agent".to_string(),
        }],
    };
    fs::write(
        ".strand/config.json",
        serde_json::to_string_pretty(&old_config).unwrap(),
    )
    .unwrap();

    unsafe {
        std::env::set_var("strand_INIT_PROJECT", "new/project");
        std::env::set_var("strand_INIT_BRANCH", "new_branch");
        std::env::remove_var("strand_INIT_CODEX");
        std::env::set_var("strand_INIT_AGENTS_PROJECT", "new/agents");
        std::env::set_var("strand_INIT_AGENTS_BRANCH", "new_agent_branch");
    }

    strand::commands::init::init().unwrap();

    let opencode_link = Path::new(".opencode/agents/test-agent");
    assert!(
        opencode_link.symlink_metadata().is_ok(),
        "Symlink should be created for existing agent"
    );

    let config_str = fs::read_to_string(".strand/config.json").unwrap();
    let config: strand::config::Config = serde_json::from_str(&config_str).unwrap();
    assert_eq!(config.agents.len(), 1);
    assert_eq!(config.agents[0].name, "test-agent");
    assert_eq!(config.agents_repo.project, "new/agents");

    unsafe {
        std::env::remove_var("strand_INIT_PROJECT");
        std::env::remove_var("strand_INIT_BRANCH");
        std::env::remove_var("strand_INIT_AGENTS_PROJECT");
        std::env::remove_var("strand_INIT_AGENTS_BRANCH");
    }
}

#[test]
fn test_cli_init_uses_env_vars() {
    let temp = TempDir::new().unwrap();
    let output = Command::new(crate::strand_bin())
        .current_dir(temp.path())
        .env("strand_INIT_PROJECT", "env/project")
        .env("strand_INIT_BRANCH", "env_branch")
        .env("strand_INIT_CODEX", "true")
        .env("strand_INIT_BASE_URL", "https://gitlab.env.net")
        .env("strand_INIT_AGENTS_PROJECT", "env/agents")
        .env("strand_INIT_AGENTS_BRANCH", "env_agents_branch")
        .arg("init")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute strand init");

    assert!(
        output.status.success(),
        "strand init failed. stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let config_path = temp.path().join(".strand/config.json");
    assert!(config_path.exists());
    let config_str = fs::read_to_string(&config_path).unwrap();
    let config: strand::config::Config = serde_json::from_str(&config_str).unwrap();
    assert_eq!(config.skills_repo.project, "env/project");
    assert_eq!(config.skills_repo.branch, "env_branch");
    assert_eq!(config.skills_repo.base_url, "https://gitlab.env.net");
    assert!(config.targets.codex);
    assert_eq!(config.agents_repo.project, "env/agents");
    assert_eq!(config.agents_repo.branch, "env_agents_branch");
}
