//! GitHub API client module.
//!
//! Provides a thin wrapper around reqwest for GitHub REST API and GraphQL operations.
//! Handles authentication, rate limiting, and error handling consistently.

pub mod auth;
pub mod client;
pub mod init_client;
pub mod models;
pub mod rate_limit;

pub use auth::{AuthError, GitHubAuth};
pub use init_client::GitHubClient;
pub use models::*;
pub use rate_limit::{RateLimitHandler, DEFAULT_WARNING_THRESHOLD};

pub use client::{
    BranchHead, CheckRun, CommitStatus, GitTag, MergedPR, close_issue,
};
