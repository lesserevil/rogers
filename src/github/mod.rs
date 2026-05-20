//! GitHub API client for Rodgers issue and comment operations.
//!
//! This module provides the interface for fetching GitHub issue data needed
//! for epic bead creation. Specifically, it fetches issue comments to extract
//! acceptance criteria that may have been added by Rodgers or by humans.

pub mod client;

pub use client::{
    close_issue, GitHubClient, GitHubComment, GitHubIssue, GitHubLabel, GitHubUser, IssueState,
};

/// Backward compatibility alias
pub type OldGitHubClient = GitHubClient;