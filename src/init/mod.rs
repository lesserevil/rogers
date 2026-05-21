//! Init command — orchestrates the project readiness audit.
//!
//! This module provides the entry point for `rogers init`, which audits
//! a GitHub repository for readiness and optionally applies automated fixes
//! via the `--fix` flag.

pub mod fix;
pub mod report;

use crate::checks::{CheckResult, Fixability, InitCheck, Severity};
use crate::error::Result;
use crate::github::GitHubClient;

/// Result of running `rogers init` with optional --fix.
#[derive(Debug, Clone)]
pub struct InitResult {
    /// Whether any blocker-level findings were reported.
    pub has_blockers: bool,
    /// Labels created or skipped by the fix operation.
    pub label_fix: Option<crate::init::fix::FixResult>,
    /// Discussion categories created or skipped by the fix operation.
    pub category_fix: Option<crate::init::fix::CategoryFixResult>,
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

    // Run the required labels check.
    let labels_check = crate::checks::LabelsCheck;
    let labels_results = labels_check.check(github, owner, repo).await?;
    if let Some(first) = labels_results.first() {
        println!("[{}] {}", first.severity.as_str(), first.description);

        // Report fixability for auto-fixable findings.
        if first.fixability == Fixability::Auto
            && let Some(ref instructions) = first.fix_instructions
        {
            for line in instructions.lines() {
                println!("{}", line);
            }
        }
    }

    let mut label_fix = None;
    let mut category_fix = None;
    let has_blockers = labels_results
        .iter()
        .any(|r| r.severity == Severity::Blocker);

    if fix && has_blockers {
        let result = crate::init::fix::ensure_labels(github, owner, repo).await?;
        crate::init::fix::print_fix_report(&result);
        label_fix = Some(result);
    }

    // Always apply discussion category fix when --fix is set (it's a warn, not a blocker).
    if fix {
        let cat_result = crate::init::fix::ensure_discussion_category(github, owner, repo).await?;
        crate::init::fix::print_category_fix_report(&cat_result);
        category_fix = Some(cat_result);
    }

    println!("Repository: {}/{}", repository.full_name, repository.name);
    println!("Default branch: {}", repository.default_branch);
    println!("Has issues: {}", repository.has_issues);
    println!("Has discussions: {}", repository.has_discussions);

    Ok(InitResult {
        has_blockers,
        label_fix,
        category_fix,
    })
}

/// Runs all init audit checks for a repository and prints the results.
///
/// # Arguments
/// * `owner` — Repository owner
/// * `repo` — Repository name
/// * `github` — GitHub API client
pub async fn run_all_checks(
    owner: &str,
    repo: &str,
    github: &GitHubClient,
) -> Result<Vec<CheckResult>> {
    // Fetch repository to verify connectivity.
    let repository = github.get_repository(owner, repo).await?;

    let mut all_results = Vec::new();

    // Run the required labels check.
    let labels_check = crate::checks::LabelsCheck;
    let labels_results = labels_check.check(github, owner, repo).await?;
    all_results.extend(labels_results.clone());
    for result in &labels_results {
        println!("[{}] {}", result.severity.as_str(), result.description);
        if result.fixability == Fixability::Auto
            && let Some(ref instructions) = result.fix_instructions
        {
            for line in instructions.lines() {
                println!("{}", line);
            }
        }
    }

    // Run the issue templates check.
    let issue_templates_check = crate::checks::IssueTemplatesCheck;
    let issue_templates_results = issue_templates_check.check(github, owner, repo).await?;
    all_results.extend(issue_templates_results.clone());
    for result in &issue_templates_results {
        println!("[{}] {}", result.severity.as_str(), result.description);
    }

    // Run the repo settings check.
    let repo_settings_check = crate::checks::RepoSettingsCheck;
    let repo_settings_results = repo_settings_check.check(github, owner, repo).await?;
    all_results.extend(repo_settings_results.clone());
    for result in &repo_settings_results {
        println!("[{}] {}", result.severity.as_str(), result.description);
    }

    // Run the discussion categories check.
    let discussion_categories_check = crate::checks::DiscussionCategoriesCheck;
    let discussion_categories_results = discussion_categories_check
        .check(github, owner, repo)
        .await?;
    all_results.extend(discussion_categories_results.clone());
    for result in &discussion_categories_results {
        println!("[{}] {}", result.severity.as_str(), result.description);
    }

    println!("Repository: {}/{}", repository.full_name, repository.name);
    println!("Default branch: {}", repository.default_branch);
    println!("Has issues: {}", repository.has_issues);
    println!("Has discussions: {}", repository.has_discussions);

    Ok(all_results)
}
