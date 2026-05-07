use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitLabError {
    #[error("authentication failed: {0}")]
    AuthError(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },
    #[error("parse error: {0}")]
    ParseError(String),
}
