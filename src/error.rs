#![allow(dead_code)]

//! Error types for Rogers.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RogersError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("GitHub API error: {0}")]
    GitHub(#[from] reqwest::Error),

    #[error("GitHub API returned non-success: code={code} message={message}")]
    GitHubStatus { code: u16, message: String },

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

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl RogersError {
    pub fn exit_code(&self) -> i32 {
        match self {
            RogersError::Config(_) => 2,
            RogersError::GitHub(_) | RogersError::GitHubStatus { .. } => 3,
            RogersError::Auth(_) | RogersError::RepoNotFound => 3,
            RogersError::Beads(_) => 1,
            RogersError::Plan(_) => 1,
            RogersError::Io(_) | RogersError::Yaml(_) | RogersError::Json(_) => 2,
        }
    }
}

impl Clone for RogersError {
    fn clone(&self) -> Self {
        match self {
            RogersError::Config(msg) => RogersError::Config(msg.clone()),
            RogersError::GitHub(_) => {
                // Network errors aren't directly cloneable; wrap in GitHubStatus
                RogersError::GitHubStatus {
                    code: 0,
                    message: "GitHub network error (clone fallback)".to_string(),
                }
            }
            RogersError::GitHubStatus { code, message } => RogersError::GitHubStatus {
                code: *code,
                message: message.clone(),
            },
            RogersError::Auth(msg) => RogersError::Auth(msg.clone()),
            RogersError::Beads(msg) => RogersError::Beads(msg.clone()),
            RogersError::Plan(msg) => RogersError::Plan(msg.clone()),
            RogersError::RepoNotFound => RogersError::RepoNotFound,
            RogersError::Io(_) => {
                RogersError::Io(std::io::Error::other("IO error (clone fallback)"))
            }
            RogersError::Yaml(_) => RogersError::Config("YAML error (clone fallback)".to_string()),
            RogersError::Json(_) => RogersError::Config("JSON error (clone fallback)".to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, RogersError>;
