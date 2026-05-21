//! Init command — orchestrates the project readiness audit.
//!
//! This module provides the entry point for `rogers init`, which audits
//! a GitHub repository for readiness and optionally applies automated fixes
//! via the `--fix` flag.

pub mod fix;

use crate::error::Result;
use crate::github::GitHubClient;

/// Result of running `rogers init` with optional --fix.
#[derive(Debug, Clone)]
pub struct InitResult {
    /// Labels created or skipped by the fix operation.
    pub label_fix: Option<crate::init::fix::FixResult>,
}

/// Runs the init audit for a repository.
///
/// When `fix` is true, also runs auto-fix for available fixes (currently labels).
///
/// # Arguments
/// * `owner` — Repository owner
/// * `repo` — Repository name
/// * `fix` — Whether to apply automated fixes
/// * `github` — GitHub API client
pub async fn run_init(
    owner: &str,
    repo: &str,
    fix: bool,
    github: &GitHubClient,
) -> Result<InitResult> {
    // Fetch repository to verify connectivity.
    let repository = github.get_repository(owner, repo).await?;

    let mut label_fix = None;
    if fix {
        let result = crate::init::fix::ensure_labels(github, owner, repo).await?;
        crate::init::fix::print_fix_report(&result);
        label_fix = Some(result);
    }

    println!("Repository: {}/{}", repository.full_name, repository.name);
    println!("Default branch: {}", repository.default_branch);
    println!("Has issues: {}", repository.has_issues);
    println!("Has discussions: {}", repository.has_discussions);

    Ok(InitResult { label_fix })
}
