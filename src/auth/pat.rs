use super::{Auth, AuthError, AuthBackend};

pub struct PatAuth {
    env_var: &'static str,
}

impl PatAuth {
    pub fn new() -> Self {
        Self {
            env_var: "strand_GITLAB_TOKEN",
        }
    }

    #[cfg(test)]
    fn with_env_var(env_var: &'static str) -> Self {
        Self { env_var }
    }

    pub fn authenticate(&self) -> Result<AuthBackend, AuthError> {
        let token = self.get_token()?;
        Ok(AuthBackend::Token { token })
    }
}

impl Default for PatAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl Auth for PatAuth {
    fn get_token(&self) -> Result<String, AuthError> {
        std::env::var(self.env_var).map_err(|_| AuthError::EnvVarNotSet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pat_auth_with_env_var() {
        let test_var = "TEST_SHARD_GITLAB_TOKEN_12345";
        unsafe { std::env::set_var(test_var, "glpat-from-env") };
        let auth = PatAuth::with_env_var(test_var);
        assert_eq!(auth.get_token().unwrap(), "glpat-from-env");
        unsafe { std::env::remove_var(test_var) };
    }

    #[test]
    fn test_pat_auth_without_env_var() {
        let test_var = "TEST_SHARD_GITLAB_TOKEN_MISSING";
        unsafe { std::env::remove_var(test_var) };
        let auth = PatAuth::with_env_var(test_var);
        assert!(matches!(auth.get_token(), Err(AuthError::EnvVarNotSet)));
    }

    #[test]
    fn test_pat_auth_backend() {
        let test_var = "TEST_SHARD_GITLAB_TOKEN_BACKEND";
        unsafe { std::env::set_var(test_var, "glpat-backend") };
        let auth = PatAuth::with_env_var(test_var);
        let backend = auth.authenticate().unwrap();
        assert!(matches!(backend, AuthBackend::Token { token } if token == "glpat-backend"));
        unsafe { std::env::remove_var(test_var) };
    }
}
