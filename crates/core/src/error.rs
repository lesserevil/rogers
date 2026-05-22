//! Error types for Rogers.
//!
//! Shared across all workspace crates via `rogers-core`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RogersError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("GitHub API error: {0}")]
    GitHub(#[from] reqwest::Error),

    #[error("GitHub API returned non-success: status {code}, message: {message}")]
    GitHubStatus { code: u16, message: String },

    #[error("rate limit exceeded: {remaining} remaining, resets at {reset_at}")]
    RateLimitExceeded { remaining: i32, reset_at: i64 },

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("beads database error: {0}")]
    Beads(String),

    #[error("plan file error: {0}")]
    Plan(String),

    #[error("repository not found or not accessible")]
    RepoNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl RogersError {
    pub fn exit_code(&self) -> i32 {
        match self {
            RogersError::Config(_) => 2,
            RogersError::GitHub(_) | RogersError::GitHubStatus { .. } => 3,
            RogersError::RateLimitExceeded { .. } => 3,
            RogersError::Auth(_) | RogersError::RepoNotFound => 3,
            RogersError::Beads(_) => 1,
            RogersError::Plan(_) => 1,
            RogersError::Io(_) => 2,
            RogersError::Yaml(_) => 2,
            RogersError::Json(_) => 2,
        }
    }
}

pub type Result<T> = std::result::Result<T, RogersError>;
