//! Error types for Rogers.

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

    #[error("Backlog.md task store error: {0}")]
    Backlog(String),

    #[error("plan file error: {0}")]
    Plan(String),

    #[error("repository not found or not accessible")]
    RepoNotFound,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("backport error: {0}")]
    Backport(String),

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
            RogersError::Backlog(_) => 1,
            RogersError::Plan(_) => 1,
            RogersError::Backport(_) => 1,
            RogersError::Io(_) => 2,
            RogersError::Yaml(_) => 2,
            RogersError::Json(_) => 2,
        }
    }
}

/// Convert GitHub auth errors to Rodgers errors.
impl From<crate::github::auth::AuthError> for RogersError {
    fn from(err: crate::github::auth::AuthError) -> Self {
        match err {
            crate::github::auth::AuthError::EmptyToken => {
                RogersError::Auth("Token is empty or missing".to_string())
            }
            crate::github::auth::AuthError::InvalidTokenFormat => {
                RogersError::Auth("Token format is invalid for GitHub".to_string())
            }
            crate::github::auth::AuthError::MissingRequiredScopes { missing } => RogersError::Auth(
                format!("Token missing required scopes: {}", missing.join(", ")),
            ),
            crate::github::auth::AuthError::AuthFailed { message } => {
                RogersError::Auth(format!("Authentication failed: {}", message))
            }
            crate::github::auth::AuthError::TokenExpired => {
                RogersError::Auth("Token is expired or revoked".to_string())
            }
            crate::github::auth::AuthError::InsufficientPermissions { required } => {
                RogersError::Auth(format!(
                    "Token lacks required permissions: {}",
                    required.join(", ")
                ))
            }
        }
    }
}

/// Convert backport execution errors to Rodgers errors.
impl From<crate::backport::execution::BackportExecutionError> for RogersError {
    fn from(err: crate::backport::execution::BackportExecutionError) -> Self {
        RogersError::Backport(format!("{}", err))
    }
}

pub type Result<T> = std::result::Result<T, RogersError>;
