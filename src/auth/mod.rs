use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("glab not found or not authenticated: {0}")]
    GlabError(String),
    #[error("environment variable strand_GITLAB_TOKEN not set")]
    EnvVarNotSet,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse glab output")]
    ParseError,
    #[error("interactive prompt failed: {0}")]
    InteractiveError(String),
    #[error("glab binary not found in PATH")]
    GlabNotInstalled,
}

pub trait Auth {
    fn get_token(&self) -> Result<String, AuthError>;
}

pub mod glab;
pub mod interactive;
pub mod pat;

#[allow(unused_imports)]
pub(crate) use glab::GlabAuth;
#[allow(unused_imports)]
pub(crate) use interactive::InteractiveAuth;
pub use pat::PatAuth;

#[derive(Debug, Clone)]
pub enum AuthBackend {
    Glab { hostname: String },
    Token { token: String },
}

pub struct TokenAuth {
    token: String,
}

impl TokenAuth {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl Auth for TokenAuth {
    fn get_token(&self) -> Result<String, AuthError> {
        Ok(self.token.clone())
    }
}

/// Attempts authentication using available methods in order:
/// 1. glab CLI (for the given hostname)
/// 2. Personal Access Token from environment
/// 3. Interactive prompt
pub fn authenticate(hostname: &str) -> Result<AuthBackend, AuthError> {
    let glab = GlabAuth::new();
    if glab.is_installed() && glab.is_authenticated_for(hostname) {
        return Ok(AuthBackend::Glab {
            hostname: hostname.to_string(),
        });
    }

    // Try PAT from environment
    let pat = PatAuth::new();
    match pat.get_token() {
        Ok(token) => return Ok(AuthBackend::Token { token }),
        Err(AuthError::EnvVarNotSet) => {}
        Err(e) => return Err(e),
    }

    // Fall back to interactive prompt
    let interactive = InteractiveAuth::new();
    let token = interactive.get_token()?;
    Ok(AuthBackend::Token { token })
}
