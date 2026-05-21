//! GitHub API client module.
//!
//! Provides a thin wrapper around reqwest for GitHub REST API and GraphQL operations.
//! Handles authentication, rate limiting, and error handling consistently.

pub mod auth;
pub mod client;
pub mod models;
pub mod rate_limit;

pub use auth::{AuthError, GitHubAuth};
pub use client::GitHubClient;
pub use models::*;
pub use rate_limit::{RateLimitHandler, DEFAULT_WARNING_THRESHOLD};
