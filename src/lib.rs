//! Rodgers library root — all public modules and re-exports.
#![allow(
    clippy::collapsible_if,
    clippy::derivable_impls,
    clippy::useless_format,
    clippy::needless_borrow,
    clippy::unnecessary_map_or
)]

pub mod backport;
pub mod beads;
pub use beads::client::BeadClient;
pub mod config;
pub mod github;
pub mod triage;

// Re-exports — submodules use `crate::Config`, `crate::RogersError`, etc.
pub use config::schema::Config;
pub use config::schema::GithubConfig;
pub use error::RogersError;

// Submodule error types and results via re-export
pub use error::Result;

pub mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum RogersError {
        #[error("configuration error: {0}")]
        Config(String),

        #[error("GitHub API error: {0}")]
        GitHub(#[from] reqwest::Error),

        #[error("GitHub API returned non-success: {code} — {message}")]
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

        #[error("triage error: {0}")]
        Triage(String),
    }

    impl RogersError {
        pub fn exit_code(&self) -> i32 {
            match self {
                RogersError::Config(_) => 2,
                RogersError::GitHub(_) | RogersError::GitHubStatus { .. } => 3,
                RogersError::Auth(_) | RogersError::RepoNotFound => 3,
                RogersError::Beads(_) => 1,
                RogersError::Plan(_) => 1,
                RogersError::Io(_) => 2,
                RogersError::Yaml(_) => 2,
                RogersError::Triage(_) => 1,
            }
        }
    }

    pub type Result<T> = std::result::Result<T, RogersError>;
}
