use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("gitlab error: {0}")]
    GitLab(#[from] crate::gitlab::GitLabError),
}
