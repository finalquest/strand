pub mod errors;
pub mod gitlab;
pub mod local;

pub use errors::ResolverError;
pub use gitlab::GitLabSkillSource;
pub use local::LocalSkillSource;

pub trait SkillSource {
    fn read_file(&self, path: &str) -> Result<String, ResolverError>;
}

pub struct Resolver {
    default_project: String,
    default_base_url: String,
}

impl Resolver {
    pub fn new(default_project: impl Into<String>, default_base_url: impl Into<String>) -> Self {
        Self {
            default_project: default_project.into(),
            default_base_url: default_base_url.into(),
        }
    }

    pub fn resolve(&self) -> Result<Box<dyn SkillSource>, ResolverError> {
        // 1. Try local
        if let Ok(path) = std::env::var("strand_SKILLS_REPO_PATH") {
            if let Ok(source) = LocalSkillSource::new(&path) {
                return Ok(Box::new(source));
            }
        }

        // 2. Try GitLab from env
        if let Ok(project) = std::env::var("strand_SKILLS_REPO") {
            let client = crate::gitlab::GitLabClient::for_project(
                self.default_base_url.clone(),
                project,
            )
            .map_err(|e| ResolverError::GitLab(crate::gitlab::GitLabError::AuthError(e.to_string())))?;
            return Ok(Box::new(GitLabSkillSource::new(client)));
        }

        // 3. Try default
        let client = crate::gitlab::GitLabClient::for_project(
            self.default_base_url.clone(),
            self.default_project.clone(),
        )
        .map_err(|e| ResolverError::GitLab(crate::gitlab::GitLabError::AuthError(e.to_string())))?;
        Ok(Box::new(GitLabSkillSource::new(client)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn set_env(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env(key: &str) {
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn test_resolver_local_priority() {
        let _guard = ENV_MUTEX.lock().unwrap();

        let temp_dir = std::env::temp_dir().join("strand_test_local_priority");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let test_file = temp_dir.join("skills/test/skill.json");
        std::fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        let mut file = std::fs::File::create(&test_file).unwrap();
        file.write_all(b"local skill content").unwrap();

        let old_local = std::env::var("strand_SKILLS_REPO_PATH").ok();
        let old_gitlab = std::env::var("strand_SKILLS_REPO").ok();
        let old_token = std::env::var("strand_GITLAB_TOKEN").ok();

        set_env("strand_SKILLS_REPO_PATH", temp_dir.to_str().unwrap());
        set_env("strand_SKILLS_REPO", "some/project");
        set_env("strand_GITLAB_TOKEN", "test-token");

        let resolver = Resolver::new("default/project", "https://gitlab.example.com");
        let source = resolver.resolve().unwrap();
        let content = source.read_file("skills/test/skill.json").unwrap();
        assert_eq!(content, "local skill content");

        match old_local {
            Some(v) => set_env("strand_SKILLS_REPO_PATH", &v),
            None => remove_env("strand_SKILLS_REPO_PATH"),
        }
        match old_gitlab {
            Some(v) => set_env("strand_SKILLS_REPO", &v),
            None => remove_env("strand_SKILLS_REPO"),
        }
        match old_token {
            Some(v) => set_env("strand_GITLAB_TOKEN", &v),
            None => remove_env("strand_GITLAB_TOKEN"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolver_gitlab_fallback() {
        let _guard = ENV_MUTEX.lock().unwrap();

        let old_local = std::env::var("strand_SKILLS_REPO_PATH").ok();
        let old_gitlab = std::env::var("strand_SKILLS_REPO").ok();
        let old_token = std::env::var("strand_GITLAB_TOKEN").ok();

        remove_env("strand_SKILLS_REPO_PATH");
        set_env("strand_SKILLS_REPO", "fallback/project");
        set_env("strand_GITLAB_TOKEN", "test-token");

        let resolver = Resolver::new("default/project", "https://gitlab.example.com");
        let result = resolver.resolve();
        assert!(result.is_ok());

        match old_local {
            Some(v) => set_env("strand_SKILLS_REPO_PATH", &v),
            None => remove_env("strand_SKILLS_REPO_PATH"),
        }
        match old_gitlab {
            Some(v) => set_env("strand_SKILLS_REPO", &v),
            None => remove_env("strand_SKILLS_REPO"),
        }
        match old_token {
            Some(v) => set_env("strand_GITLAB_TOKEN", &v),
            None => remove_env("strand_GITLAB_TOKEN"),
        }
    }

    #[test]
    fn test_resolver_default_fallback() {
        let _guard = ENV_MUTEX.lock().unwrap();

        let old_local = std::env::var("strand_SKILLS_REPO_PATH").ok();
        let old_gitlab = std::env::var("strand_SKILLS_REPO").ok();
        let old_token = std::env::var("strand_GITLAB_TOKEN").ok();

        remove_env("strand_SKILLS_REPO_PATH");
        remove_env("strand_SKILLS_REPO");
        set_env("strand_GITLAB_TOKEN", "test-token");

        let resolver = Resolver::new("default/project", "https://gitlab.example.com");
        let result = resolver.resolve();
        assert!(result.is_ok());

        match old_local {
            Some(v) => set_env("strand_SKILLS_REPO_PATH", &v),
            None => remove_env("strand_SKILLS_REPO_PATH"),
        }
        match old_gitlab {
            Some(v) => set_env("strand_SKILLS_REPO", &v),
            None => remove_env("strand_SKILLS_REPO"),
        }
        match old_token {
            Some(v) => set_env("strand_GITLAB_TOKEN", &v),
            None => remove_env("strand_GITLAB_TOKEN"),
        }
    }
}
