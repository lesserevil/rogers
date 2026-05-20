//! GitHub API client for Rodgers issue and comment operations.
//!
//! This module provides the interface for fetching GitHub issue data needed
//! for epic bead creation. Specifically, it fetches issue comments to extract
//! acceptance criteria that may have been added by Rodgers or by humans.

pub mod client;

pub use client::{GitHubClient, GitHubComment, GitHubIssue, GitHubLabel, GitHubUser};