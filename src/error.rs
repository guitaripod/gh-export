use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum GhExportError {
    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limit exceeded. Reset at: {0}")]
    RateLimit(String),

    #[error("Repository download failed: {0}")]
    Download(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Disk space insufficient: need {needed} bytes, have {available} bytes")]
    InsufficientSpace { needed: u64, available: u64 },

    #[error("Dialog error: {0}")]
    Dialog(#[from] dialoguer::Error),
}

pub type Result<T> = std::result::Result<T, GhExportError>;
