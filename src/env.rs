//! Environment variable configuration for strand.
//!
//! # Agents Repository
//! - `strand_AGENTS_REPO`: Overrides `agentsRepo.project` from config.
//! - `strand_AGENTS_REPO_BRANCH`: Overrides `agentsRepo.branch` from config.
//!
//! # Skills Repository
//! - `strand_SKILLS_REPO`: Overrides `skillsRepo.project` from config.
//! - `strand_SKILLS_REPO_BRANCH`: Overrides `skillsRepo.branch` from config.

/// Read the agents repository project from the environment.
pub fn agents_repo_project() -> Option<String> {
    std::env::var("strand_AGENTS_REPO").ok()
}

/// Read the agents repository branch from the environment.
pub fn agents_repo_branch() -> String {
    std::env::var("strand_AGENTS_REPO_BRANCH").unwrap_or_else(|_| "main".to_string())
}

/// Read the skills repository project from the environment.
pub fn skills_repo_project() -> Option<String> {
    std::env::var("strand_SKILLS_REPO").ok()
}

/// Read the skills repository branch from the environment.
pub fn skills_repo_branch() -> String {
    std::env::var("strand_SKILLS_REPO_BRANCH").unwrap_or_else(|_| "main".to_string())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    fn restore_env(key: &str, old: Option<String>) {
        match old {
            Some(v) => set_env(key, &v),
            None => remove_env(key),
        }
    }

    #[test]
    fn test_agents_repo_project_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let old = std::env::var("strand_AGENTS_REPO").ok();
        set_env("strand_AGENTS_REPO", "my-group/my-agents");
        assert_eq!(agents_repo_project(), Some("my-group/my-agents".to_string()));
        restore_env("strand_AGENTS_REPO", old);
    }

    #[test]
    fn test_agents_repo_branch_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let old = std::env::var("strand_AGENTS_REPO_BRANCH").ok();
        set_env("strand_AGENTS_REPO_BRANCH", "develop");
        assert_eq!(agents_repo_branch(), "develop");
        restore_env("strand_AGENTS_REPO_BRANCH", old);
    }

    #[test]
    fn test_agents_repo_branch_defaults_to_main() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let old = std::env::var("strand_AGENTS_REPO_BRANCH").ok();
        remove_env("strand_AGENTS_REPO_BRANCH");
        assert_eq!(agents_repo_branch(), "main");
        restore_env("strand_AGENTS_REPO_BRANCH", old);
    }

    #[test]
    fn test_skills_repo_project_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let old = std::env::var("strand_SKILLS_REPO").ok();
        set_env("strand_SKILLS_REPO", "my-group/my-skills");
        assert_eq!(skills_repo_project(), Some("my-group/my-skills".to_string()));
        restore_env("strand_SKILLS_REPO", old);
    }

    #[test]
    fn test_skills_repo_branch_from_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let old = std::env::var("strand_SKILLS_REPO_BRANCH").ok();
        set_env("strand_SKILLS_REPO_BRANCH", "develop");
        assert_eq!(skills_repo_branch(), "develop");
        restore_env("strand_SKILLS_REPO_BRANCH", old);
    }

    #[test]
    fn test_skills_repo_branch_defaults_to_main() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let old = std::env::var("strand_SKILLS_REPO_BRANCH").ok();
        remove_env("strand_SKILLS_REPO_BRANCH");
        assert_eq!(skills_repo_branch(), "main");
        restore_env("strand_SKILLS_REPO_BRANCH", old);
    }
}
