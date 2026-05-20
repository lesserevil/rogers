//! Init command — orchestrates the project readiness audit.
//!
//! This module provides the entry point for `rogers init`, which audits
//! a GitHub repository for readiness and optionally applies automated fixes
//! via the `--fix` flag.

pub mod fix;
pub mod report;

use crate::checks::{CheckResult, Fixability, InitCheck, ReleaseWorkflowCheck, Severity};
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
    let _repository = github.get_repository(owner, repo).await?;

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

    // Run the release workflow check.
    let release_workflow_check = ReleaseWorkflowCheck;
    let release_workflow_results = release_workflow_check.check(github, owner, repo).await?;
    all_results.extend(release_workflow_results.clone());
    for result in &release_workflow_results {
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

    // Run the general workflows check.
    let general_workflows_check = crate::checks::GeneralWorkflowsCheck;
    let general_workflows_results = general_workflows_check.check(github, owner, repo).await?;
    all_results.extend(general_workflows_results.clone());
    for result in &general_workflows_results {
        println!("[{}] {}", result.severity.as_str(), result.description);
    }

    Ok(all_results)
}

/// Init module - repository initialization and template discovery.
///
/// This module handles:
/// - Repository initialization checks
/// - Template discovery and validation
/// - Bead filing when templates are missing and auto_suggest=true

use crate::templates::{TemplateDiscovery, TEMPLATE_BEAD_TITLE, TEMPLATE_BEAD_TYPE_LABEL};

/// Configuration for templates section.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TemplatesConfig {
    /// Whether to file a bead with suggested templates when none found.
    #[serde(default = "default_auto_suggest")]
    pub auto_suggest: bool,
}

fn default_auto_suggest() -> bool {
    true
}

impl Default for TemplatesConfig {
    fn default() -> Self {
        Self {
            auto_suggest: true,
        }
    }
}

/// Full Rodgers configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RodgersConfig {
    pub templates: TemplatesConfig,
}

impl Default for RodgersConfig {
    fn default() -> Self {
        Self {
            templates: TemplatesConfig::default(),
        }
    }
}

/// Result of running init checks.
#[derive(Debug, Clone)]
pub struct InitCheckResult {
    /// The repository checked.
    pub repository: String,
    /// Template discovery result.
    pub template_discovery: TemplateDiscovery,
    /// Whether a bead was (or would be) filed.
    pub bead_filed: bool,
    /// The bead content if applicable.
    pub bead_body: Option<String>,
}

impl InitCheckResult {
    /// Create a new init check result.
    pub fn new(repository: String) -> Self {
        Self {
            repository: repository.clone(),
            template_discovery: TemplateDiscovery::new(repository),
            bead_filed: false,
            bead_body: None,
        }
    }

    /// Check templates and determine if a bead should be filed.
    ///
    /// In a real implementation, this would query the GitHub API to check
    /// for existing templates. For now, it uses the provided discovery result.
    pub fn with_template_discovery(mut self, discovery: TemplateDiscovery, auto_suggest: bool) -> Self {
        self.template_discovery = discovery;
        
        if self.template_discovery.should_file_bead(auto_suggest) {
            self.bead_filed = true;
            self.bead_body = Some(self.template_discovery.generate_bead_body());
        }
        
        self
    }

    /// Get the bead title for filing.
    pub fn bead_title(&self) -> &'static str {
        TEMPLATE_BEAD_TITLE
    }

    /// Get the bead type label.
    pub fn bead_type_label(&self) -> &'static str {
        TEMPLATE_BEAD_TYPE_LABEL
    }
}

/// Check templates for a repository and file a bead if needed.
///
/// This function is called during `rogers init` to check if the target
/// repository has issue templates. If none are found and auto_suggest is true,
/// a bead is generated with suggested default templates.
///
/// Returns the init check result with bead information if applicable.
pub fn check_and_suggest_templates(
    repository: &str,
    found_templates: Vec<String>,
    auto_suggest: bool,
) -> InitCheckResult {
    let discovery = TemplateDiscovery::new(repository.to_string()).with_templates(found_templates);
    
    let result = InitCheckResult::new(repository.to_string())
        .with_template_discovery(discovery, auto_suggest);
    
    if result.bead_filed {
        tracing::info!(
            repository = result.repository,
            title = result.bead_title(),
            "Filing bead for missing issue templates"
        );
    } else {
        tracing::debug!(
            repository = result.repository,
            "Templates complete or auto_suggest disabled, no bead filed"
        );
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_auto_suggest_is_true() {
        let config = TemplatesConfig::default();
        assert!(config.auto_suggest);
    }

    #[test]
    fn test_init_result_no_bead_when_templates_complete() {
        let result = InitCheckResult::new("owner/repo".to_string())
            .with_template_discovery(
                TemplateDiscovery::new("owner/repo".to_string())
                    .with_templates(vec![
                        "bug_report.md".to_string(),
                        "feature_request.md".to_string(),
                        "question.md".to_string(),
                    ]),
                true,
            );
        
        assert!(!result.bead_filed);
        assert!(result.bead_body.is_none());
    }

    #[test]
    fn test_init_result_bead_when_no_templates_and_auto_suggest() {
        let result = InitCheckResult::new("owner/repo".to_string())
            .with_template_discovery(
                TemplateDiscovery::new("owner/repo".to_string()),
                true,
            );
        
        assert!(result.bead_filed);
        assert!(result.bead_body.is_some());
        let body = result.bead_body.unwrap();
        assert!(body.contains("bug_report.md"));
        assert!(body.contains("feature_request.md"));
        assert!(body.contains("question.md"));
    }

    #[test]
    fn test_init_result_no_bead_when_no_templates_and_auto_suggest_false() {
        let result = InitCheckResult::new("owner/repo".to_string())
            .with_template_discovery(
                TemplateDiscovery::new("owner/repo".to_string()),
                false,
            );
        
        assert!(!result.bead_filed);
        assert!(result.bead_body.is_none());
    }

    #[test]
    fn test_bead_title_is_correct() {
        let result = InitCheckResult::new("owner/repo".to_string());
        assert_eq!(result.bead_title(), "Project missing issue templates - suggested templates available");
    }

    #[test]
    fn test_bead_type_label_is_infra() {
        let result = InitCheckResult::new("owner/repo".to_string());
        assert_eq!(result.bead_type_label(), "infra");
    }

    #[test]
    fn test_check_and_suggest_templates_creates_result() {
        let result = check_and_suggest_templates(
            "owner/repo",
            vec!["bug_report.md".to_string()],
            true,
        );
        
        assert_eq!(result.repository, "owner/repo");
        assert!(result.bead_filed);
        assert!(result.bead_body.is_some());
    }
}
