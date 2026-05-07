use super::{Auth, AuthError, AuthBackend};

pub struct InteractiveAuth {
    prompt: Box<dyn Fn(&str) -> Result<String, std::io::Error>>,
}

impl InteractiveAuth {
    pub fn new() -> Self {
        Self {
            prompt: Box::new(|msg| {
                use std::io::{self, Write};
                print!("{}", msg);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                Ok(input.trim().to_string())
            }),
        }
    }

    #[cfg(test)]
    pub fn with_prompt<F>(prompt: F) -> Self
    where
        F: Fn(&str) -> Result<String, std::io::Error> + 'static,
    {
        Self {
            prompt: Box::new(prompt),
        }
    }

    pub fn authenticate(&self) -> Result<AuthBackend, AuthError> {
        let token = self.get_token()?;
        Ok(AuthBackend::Token { token })
    }
}

impl Default for InteractiveAuth {
    fn default() -> Self {
        Self::new()
    }
}

impl Auth for InteractiveAuth {
    fn get_token(&self) -> Result<String, AuthError> {
        let token = (self.prompt)("Enter your GitLab Personal Access Token: ")?;
        if token.is_empty() {
            return Err(AuthError::InteractiveError(
                "Token cannot be empty".to_string(),
            ));
        }
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_auth_success() {
        let auth = InteractiveAuth::with_prompt(|_msg| Ok("glpat-interactive".to_string()));
        assert_eq!(auth.get_token().unwrap(), "glpat-interactive");
    }

    #[test]
    fn test_interactive_auth_empty_token() {
        let auth = InteractiveAuth::with_prompt(|_msg| Ok("".to_string()));
        assert!(matches!(
            auth.get_token(),
            Err(AuthError::InteractiveError(_))
        ));
    }

    #[test]
    fn test_interactive_auth_io_error() {
        let auth = InteractiveAuth::with_prompt(|_msg| {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "read failed"))
        });
        assert!(matches!(auth.get_token(), Err(AuthError::Io(_))));
    }

    #[test]
    fn test_interactive_auth_backend() {
        let auth = InteractiveAuth::with_prompt(|_msg| Ok("glpat-backend".to_string()));
        let backend = auth.authenticate().unwrap();
        assert!(matches!(backend, AuthBackend::Token { token } if token == "glpat-backend"));
    }
}
