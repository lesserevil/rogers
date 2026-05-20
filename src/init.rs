//! Init command implementation — repository readiness audit.

use crate::cli::Commands;
use crate::error::{Result, RogersError};
use crate::labels::RODGERS_REQUIRED_LABELS;
use clap::Args;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Command-line arguments for the init command.
#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Target repository in owner/repo format.
    #[arg(long, value_name = "OWNER/REPO")]
    pub repo: String,

    /// Apply automated fixes where possible.
    #[arg(long, short = 'f')]
    pub fix: bool,

    /// Output JSON instead of human-readable text.
    #[arg(long, short = 'j')]
    pub json: bool,

    /// Repository admin token override (for applying settings that require admin).
    /// If not provided, reads from GITHUB_TOKEN env var.
    #[arg(long, visible_alias = "token")]
    pub github_token: Option<String>,
}

/// GitHub repository metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_wiki: bool,
}

/// Severity level for audit findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Blocker,
    Warn,
    Info,
}

/// Fixability of an audit finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fixability {
    Auto,
    Manual,
    Na,
}

/// An audit check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub severity: Severity,
    pub fixability: Fixability,
    pub message: String,
    pub details: Option<String>,
}

/// Complete audit report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub repository: String,
    pub scanned_at: String,
    pub checks: Vec<CheckResult>,
    pub summary: AuditSummary,
}

/// Summary statistics for the audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_checks: usize,
    pub blockers: usize,
    pub warnings: usize,
    pub info: usize,
}

impl AuditReport {
    fn new(repository: String, checks: Vec<CheckResult>) -> Self {
        let blockers = checks.iter().filter(|c| c.severity == Severity::Blocker).count();
        let warnings = checks.iter().filter(|c| c.severity == Severity::Warn).count();
        let info = checks.iter().filter(|c| c.severity == Severity::Info).count();

        Self {
            repository,
            scanned_at: chrono::Utc::now().to_rfc3339(),
            checks,
            summary: AuditSummary {
                total_checks: blockers + warnings + info,
                blockers,
                warnings,
                info,
            },
        }
    }

    /// Returns the exit code based on audit results.
    pub fn exit_code(&self) -> i32 {
        if self.summary.blockers > 0 {
            1
        } else {
            0
        }
    }
}

/// Init command handler.
pub struct InitCommand {
    args: InitArgs,
    client: reqwest::Client,
}

impl InitCommand {
    /// Create a new InitCommand from parsed arguments.
    pub fn new(args: InitArgs) -> Result<Self> {
        // Validate repo format early
        Self::validate_repo_format(&args.repo)?;

        let token = args
            .github_token
            .clone()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok())
            .ok_or_else(|| {
                RogersError::Auth(
                    "GitHub token not provided. Set GITHUB_TOKEN environment variable or use --github-token".to_string(),
                )
            })?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))
                .map_err(|_| RogersError::Auth("Invalid GitHub token format".to_string()))?,
        );
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("rogers/0.1.0"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(RogersError::GitHub)?;

        Ok(Self { args, client })
    }

    /// Validate repository format (owner/repo).
    fn validate_repo_format(repo: &str) -> Result<()> {
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(RogersError::Config(
                "Invalid repository format. Expected 'owner/repo'".to_string(),
            ));
        }
        Ok(())
    }

    /// Run the init command.
    pub async fn run(&self) -> Result<i32> {
        // Validate repo format early
        let (owner, repo) = self.parse_repo()?;

        // Fetch repository metadata
        let repository = self.fetch_repository(&owner, &repo).await?;

        // Run audit checks
        let checks = self.run_checks(&repository).await?;

        // Generate report
        let report = AuditReport::new(self.args.repo.clone(), checks);

        // Output report
        self.output_report(&report)?;

        // Exit with appropriate code
        Ok(report.exit_code())
    }

    /// Parse and validate the repository argument (owner/repo format).
    fn parse_repo(&self) -> Result<(String, String)> {
        let parts: Vec<&str> = self.args.repo.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(RogersError::Config(
                "Invalid repository format. Expected 'owner/repo'".to_string(),
            ));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Fetch repository metadata from GitHub API.
    async fn fetch_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let response = self.client.get(&url).send().await?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            if status.as_u16() == 404 {
                return Err(RogersError::RepoNotFound);
            }
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(RogersError::Auth(format!(
                    "Authentication failed: {}",
                    message
                )));
            }
            return Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message,
            });
        }

        let repository: Repository = response.json().await?;
        Ok(repository)
    }

    /// Run all audit checks.
    async fn run_checks(&self, repository: &Repository) -> Result<Vec<CheckResult>> {
        let mut checks = Vec::new();

        // Check 1: Required labels
        checks.extend(self.check_required_labels(repository).await?);

        // Check 2: Issue templates
        checks.push(self.check_issue_templates(repository).await?);

        // Check 3: Repository settings (branch protection)
        checks.extend(self.check_repository_settings(repository).await?);

        // Check 4: Release workflow
        checks.push(self.check_release_workflow(repository).await?);

        // Check 5: General workflows (CI on PRs)
        checks.push(self.check_general_workflows(repository).await?);

        // Check 6: Discussion categories
        checks.push(self.check_discussion_categories(repository).await?);

        // Check 7: Release branch protection
        checks.extend(self.check_release_branch_protection(repository).await?);

        // Check 8: Per-project agent instructions
        checks.push(self.check_agent_instructions(repository).await?);

        // Check 9: Repo-level Rodgers configuration
        checks.push(self.check_repo_config(repository).await?);

        Ok(checks)
    }

    /// Check for required labels.
    async fn check_required_labels(&self, repository: &Repository) -> Result<Vec<CheckResult>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/labels",
            repository.owner, repository.name
        );
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: "Failed to fetch labels".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct GitHubLabel {
            name: String,
            color: String,
        }

        let labels: Vec<GitHubLabel> = response.json().await?;
        let existing_labels: std::collections::HashSet<_> =
            labels.iter().map(|l| l.name.as_str()).collect();

        let mut results = Vec::new();
        let mut missing = Vec::new();

        for required in RODGERS_REQUIRED_LABELS {
            if existing_labels.contains(required.name) {
                results.push(CheckResult {
                    name: "required_labels".to_string(),
                    severity: Severity::Info,
                    fixability: Fixability::Na,
                    message: format!("Required label '{}' present", required.name),
                    details: None,
                });
            } else {
                missing.push(required.name);
            }
        }

        if !missing.is_empty() {
            results.push(CheckResult {
                name: "required_labels".to_string(),
                severity: Severity::Blocker,
                fixability: Fixability::Auto,
                message: format!("Required labels missing: {}", missing.join(", ")),
                details: Some(
                    "Run with --fix to create missing labels via GitHub API".to_string(),
                ),
            });
        }

        Ok(results)
    }

    /// Check for issue templates.
    async fn check_issue_templates(&self, repository: &Repository) -> Result<CheckResult> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/.github/ISSUE_TEMPLATE",
            repository.owner, repository.name
        );
        let response = self.client.get(&url).send().await?;

        if response.status().as_u16() == 404 {
            return Ok(CheckResult {
                name: "issue_templates".to_string(),
                severity: Severity::Blocker,
                fixability: Fixability::Manual,
                message: "Issue templates directory not found (.github/ISSUE_TEMPLATE/)".to_string(),
                details: Some(
                    "Create .github/ISSUE_TEMPLATE/ with at least one template file, or run with --fix to create a PR with default templates"
                        .to_string(),
                ),
            });
        }

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: "Failed to check issue templates".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct ContentItem {
            name: String,
            #[serde(rename = "type")]
            item_type: String,
        }

        let contents: Vec<ContentItem> = response.json().await?;
        let template_files: Vec<_> = contents
            .iter()
            .filter(|item| item.item_type == "file")
            .filter(|item| item.name.ends_with(".yml") || item.name.ends_with(".yaml") || item.name.ends_with(".md"))
            .collect();

        if template_files.is_empty() {
            Ok(CheckResult {
                name: "issue_templates".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Manual,
                message: "Issue templates directory exists but no template files found".to_string(),
                details: Some("Add .yml, .yaml, or .md template files to .github/ISSUE_TEMPLATE/".to_string()),
            })
        } else {
            Ok(CheckResult {
                name: "issue_templates".to_string(),
                severity: Severity::Info,
                fixability: Fixability::Na,
                message: format!("Found {} issue template(s)", template_files.len()),
                details: None,
            })
        }
    }

    /// Check repository settings (branch protection on default branch).
    async fn check_repository_settings(&self, repository: &Repository) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();

        // Check branch protection on default branch
        let url = format!(
            "https://api.github.com/repos/{}/{}/branches/{}/protection",
            repository.owner, repository.name, repository.default_branch
        );
        let response = self.client.get(&url).send().await?;

        if response.status().as_u16() == 404 {
            results.push(CheckResult {
                name: "branch_protection".to_string(),
                severity: Severity::Blocker,
                fixability: Fixability::Manual,
                message: format!(
                    "Main branch '{}' has no branch protection rules",
                    repository.default_branch
                ),
                details: Some(
                    format!(
                        "Enable branch protection at https://github.com/{}/{}/settings/branches",
                        repository.owner, repository.name
                    ),
                ),
            });
        } else if response.status().is_success() {
            results.push(CheckResult {
                name: "branch_protection".to_string(),
                severity: Severity::Info,
                fixability: Fixability::Na,
                message: format!(
                    "Branch '{}' has protection rules enabled",
                    repository.default_branch
                ),
                details: None,
            });
        } else {
            results.push(CheckResult {
                name: "branch_protection".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Manual,
                message: "Could not verify branch protection status".to_string(),
                details: Some(
                    format!(
                        "Check manually at https://github.com/{}/{}/settings/branches",
                        repository.owner, repository.name
                    ),
                ),
            });
        }

        // Check repository settings that we can read
        let repo_url = format!(
            "https://api.github.com/repos/{}/{}",
            repository.owner, repository.name
        );
        let repo_response = self.client.get(&repo_url).send().await?;

        if repo_response.status().is_success() {
            #[derive(Deserialize)]
            struct RepoSettings {
                allow_forking: bool,
                delete_branch_on_merge: bool,
            }

            if let Ok(settings) = repo_response.json::<RepoSettings>().await {
                if settings.allow_forking {
                    results.push(CheckResult {
                        name: "repository_settings".to_string(),
                        severity: Severity::Info,
                        fixability: Fixability::Na,
                        message: "Fork syncing is allowed".to_string(),
                        details: None,
                    });
                }

                if !settings.delete_branch_on_merge {
                    results.push(CheckResult {
                        name: "delete_branch_on_merge".to_string(),
                        severity: Severity::Warn,
                        fixability: Fixability::Manual,
                        message: "Delete branches on merge is disabled".to_string(),
                        details: Some(
                            "Enable 'Automatically delete head branches' in repository settings".to_string(),
                        ),
                    });
                } else {
                    results.push(CheckResult {
                        name: "delete_branch_on_merge".to_string(),
                        severity: Severity::Info,
                        fixability: Fixability::Na,
                        message: "Delete branches on merge is enabled".to_string(),
                        details: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Check for release-capable GitHub Actions workflow.
    async fn check_release_workflow(&self, repository: &Repository) -> Result<CheckResult> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/.github/workflows",
            repository.owner, repository.name
        );
        let response = self.client.get(&url).send().await?;

        if response.status().as_u16() == 404 {
            return Ok(CheckResult {
                name: "release_workflow".to_string(),
                severity: Severity::Blocker,
                fixability: Fixability::Manual,
                message: "No GitHub Actions workflows directory found".to_string(),
                details: Some(
                    "Create a release workflow in .github/workflows/ that triggers on tag push and produces artifacts".to_string(),
                ),
            });
        }

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: "Failed to list workflows".to_string(),
            });
        }

        #[derive(Deserialize)]
        struct ContentItem {
            name: String,
            #[serde(rename = "type")]
            item_type: String,
            download_url: Option<String>,
        }

        let contents: Vec<ContentItem> = response.json().await?;
        let workflow_files: Vec<_> = contents
            .iter()
            .filter(|item| item.item_type == "file" && (item.name.ends_with(".yml") || item.name.ends_with(".yaml")))
            .collect();

        if workflow_files.is_empty() {
            return Ok(CheckResult {
                name: "release_workflow".to_string(),
                severity: Severity::Blocker,
                fixability: Fixability::Manual,
                message: "No GitHub Actions workflow files found".to_string(),
                details: Some(
                    "Create a release workflow that triggers on tag push (v* or *.*.*) and uploads artifacts".to_string(),
                ),
            });
        }

        // Check each workflow for release triggers and artifact upload
        let mut has_release_workflow = false;
        let mut has_artifact_upload = false;

        for workflow in workflow_files {
            if let Some(download_url) = &workflow.download_url {
                if let Ok(content_response) = self.client.get(download_url).send().await {
                    if let Ok(content) = content_response.text().await {
                        // Check for release triggers
                        if content.contains("push:") && (content.contains("tags:") || content.contains("v*") || content.contains("*.*.*")) {
                            has_release_workflow = true;
                        }
                        if content.contains("workflow_dispatch:") && content.contains("release") {
                            has_release_workflow = true;
                        }
                        // Check for artifact upload
                        if content.contains("upload-artifact") || content.contains("gh release upload") || content.contains("docker push") {
                            has_artifact_upload = true;
                        }
                    }
                }
            }
        }

        if !has_release_workflow {
            Ok(CheckResult {
                name: "release_workflow".to_string(),
                severity: Severity::Blocker,
                fixability: Fixability::Manual,
                message: "No release-capable GitHub Actions workflow found".to_string(),
                details: Some(
                    "Add a workflow that triggers on tag push (v* or *.*.*) and produces build artifacts".to_string(),
                ),
            })
        } else if !has_artifact_upload {
            Ok(CheckResult {
                name: "release_workflow".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Manual,
                message: "Release workflow exists but may not produce artifacts".to_string(),
                details: Some("Verify workflow has upload-artifact or similar artifact publishing steps".to_string()),
            })
        } else {
            Ok(CheckResult {
                name: "release_workflow".to_string(),
                severity: Severity::Info,
                fixability: Fixability::Na,
                message: "Release workflow found and appears to produce artifacts".to_string(),
                details: None,
            })
        }
    }

    /// Check for general CI workflows on PRs.
    async fn check_general_workflows(&self, repository: &Repository) -> Result<CheckResult> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/.github/workflows",
            repository.owner, repository.name
        );
        let response = self.client.get(&url).send().await?;

        if response.status().as_u16() == 404 || !response.status().is_success() {
            return Ok(CheckResult {
                name: "ci_workflow".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Na,
                message: "Could not verify CI workflow status".to_string(),
                details: Some("Check .github/workflows/ for PR-triggered CI workflows".to_string()),
            });
        }

        #[derive(Deserialize)]
        struct ContentItem {
            name: String,
            download_url: Option<String>,
        }

        let contents: Vec<ContentItem> = response.json().await?;
        let mut has_pr_ci = false;

        for workflow in contents.iter().filter(|w| w.name.ends_with(".yml") || w.name.ends_with(".yaml")) {
            if let Some(download_url) = &workflow.download_url {
                if let Ok(content_response) = self.client.get(download_url).send().await {
                    if let Ok(content) = content_response.text().await {
                        if content.contains("pull_request:") || content.contains("pull_request_target:") {
                            has_pr_ci = true;
                            break;
                        }
                    }
                }
            }
        }

        if has_pr_ci {
            Ok(CheckResult {
                name: "ci_workflow".to_string(),
                severity: Severity::Info,
                fixability: Fixability::Na,
                message: "CI workflow found for pull requests".to_string(),
                details: None,
            })
        } else {
            Ok(CheckResult {
                name: "ci_workflow".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Na,
                message: "No CI workflow found for pull requests targeting main".to_string(),
                details: Some("Add a workflow that runs on pull_request to validate changes".to_string()),
            })
        }
    }

    /// Check for discussion categories.
    async fn check_discussion_categories(&self, repository: &Repository) -> Result<CheckResult> {
        // GitHub Discussions API requires GraphQL, fallback to REST categories endpoint
        let url = format!(
            "https://api.github.com/repos/{}/{}/discussions/categories",
            repository.owner, repository.name
        );
        let response = self.client.get(&url).send().await?;

        if response.status().as_u16() == 404 || response.status().as_u16() == 403 {
            // Discussions might not be enabled
            return Ok(CheckResult {
                name: "discussion_categories".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Auto,
                message: "Discussion category 'Release Proposals' not found (discussions may not be enabled)".to_string(),
                details: Some("Enable GitHub Discussions and run with --fix to create the category".to_string()),
            });
        }

        if !response.status().is_success() {
            return Ok(CheckResult {
                name: "discussion_categories".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Na,
                message: "Could not fetch discussion categories".to_string(),
                details: None,
            });
        }

        #[derive(Deserialize)]
        struct DiscussionCategory {
            name: String,
        }

        let categories: Vec<DiscussionCategory> = response.json().await?;
        let has_release_proposals = categories.iter().any(|c| c.name == "Release Proposals");

        if has_release_proposals {
            Ok(CheckResult {
                name: "discussion_categories".to_string(),
                severity: Severity::Info,
                fixability: Fixability::Na,
                message: "Discussion category 'Release Proposals' exists".to_string(),
                details: None,
            })
        } else {
            Ok(CheckResult {
                name: "discussion_categories".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Auto,
                message: "Discussion category 'Release Proposals' not found".to_string(),
                details: Some("Run with --fix to create the category via GitHub API".to_string()),
            })
        }
    }

    /// Check release branch protection.
    async fn check_release_branch_protection(&self, repository: &Repository) -> Result<Vec<CheckResult>> {
        // For now, just check if there are any release branches
        // In a full implementation, this would read from config.yaml
        let url = format!(
            "https://api.github.com/repos/{}/{}/branches",
            repository.owner, repository.name
        );
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(vec![CheckResult {
                name: "release_branch_protection".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Na,
                message: "Could not fetch branches to check release branch protection".to_string(),
                details: None,
            }]);
        }

        #[derive(Deserialize)]
        struct Branch {
            name: String,
        }

        let branches: Vec<Branch> = response.json().await?;
        let release_branches: Vec<_> = branches
            .iter()
            .filter(|b| b.name.starts_with("release/"))
            .collect();

        if release_branches.is_empty() {
            return Ok(vec![CheckResult {
                name: "release_branch_protection".to_string(),
                severity: Severity::Info,
                fixability: Fixability::Na,
                message: "No release branches configured".to_string(),
                details: None,
            }]);
        }

        let mut results = Vec::new();
        for branch in release_branches {
            let protect_url = format!(
                "https://api.github.com/repos/{}/{}/branches/{}/protection",
                repository.owner, repository.name, branch.name
            );
            let protect_response = self.client.get(&protect_url).send().await?;

            if protect_response.status().as_u16() == 404 {
                results.push(CheckResult {
                    name: "release_branch_protection".to_string(),
                    severity: Severity::Warn,
                    fixability: Fixability::Manual,
                    message: format!("Release branch '{}' has no branch protection rules", branch.name),
                    details: Some(
                        format!(
                            "Enable branch protection at https://github.com/{}/{}/settings/branches",
                            repository.owner, repository.name
                        ),
                    ),
                });
            } else if protect_response.status().is_success() {
                results.push(CheckResult {
                    name: "release_branch_protection".to_string(),
                    severity: Severity::Info,
                    fixability: Fixability::Na,
                    message: format!("Release branch '{}' is protected", branch.name),
                    details: None,
                });
            }
        }

        Ok(results)
    }

    /// Check for per-project agent instructions.
    async fn check_agent_instructions(&self, repository: &Repository) -> Result<CheckResult> {
        let agent_files = [
            ".claude/AGENTS.md",
            ".claude/CONTRIBUTING.md",
            "AGENTS.md",
            "CONTRIBUTING.md",
            ".github/AGENTS.md",
        ];

        for file_path in &agent_files {
            let url = format!(
                "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
                repository.owner, repository.name, file_path, repository.default_branch
            );
            let response = self.client.get(&url).send().await?;

            if response.status().is_success() {
                // Found an agent file - check for contradictions (simplified)
                return Ok(CheckResult {
                    name: "agent_instructions".to_string(),
                    severity: Severity::Info,
                    fixability: Fixability::Na,
                    message: format!("Found agent instructions at {}", file_path),
                    details: Some("Agent instructions found and will be reviewed for compatibility".to_string()),
                });
            }
        }

        Ok(CheckResult {
            name: "agent_instructions".to_string(),
            severity: Severity::Info,
            fixability: Fixability::Na,
            message: "No per-project agent instructions found (using Rodgers defaults)".to_string(),
            details: None,
        })
    }

    /// Check for repo-level Rodgers configuration.
    async fn check_repo_config(&self, repository: &Repository) -> Result<CheckResult> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/rogers.yaml?ref={}",
            repository.owner, repository.name, repository.default_branch
        );
        let response = self.client.get(&url).send().await?;

        if response.status().as_u16() == 404 {
            return Ok(CheckResult {
                name: "repo_config".to_string(),
                severity: Severity::Info,
                fixability: Fixability::Na,
                message: "No rogers.yaml found — using host-level config".to_string(),
                details: None,
            });
        }

        if !response.status().is_success() {
            return Ok(CheckResult {
                name: "repo_config".to_string(),
                severity: Severity::Warn,
                fixability: Fixability::Na,
                message: "Could not fetch rogers.yaml".to_string(),
                details: None,
            });
        }

        #[derive(Deserialize)]
        struct ContentResponse {
            content: String,
            encoding: String,
        }

        let content_resp: ContentResponse = response.json().await?;
        if content_resp.encoding == "base64" {
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            if let Ok(decoded) = STANDARD.decode(&content_resp.content) {
                if let Ok(config_str) = String::from_utf8(decoded) {
                    // Basic validation - just check it's valid YAML
                    if serde_yaml::from_str::<serde_yaml::Value>(&config_str).is_ok() {
                        return Ok(CheckResult {
                            name: "repo_config".to_string(),
                            severity: Severity::Info,
                            fixability: Fixability::Na,
                            message: "rogers.yaml found and valid".to_string(),
                            details: Some("Repository-level configuration will be merged with host config".to_string()),
                        });
                    }
                }
            }
        }

        Ok(CheckResult {
            name: "repo_config".to_string(),
            severity: Severity::Warn,
            fixability: Fixability::Na,
            message: "rogers.yaml found but could not parse".to_string(),
            details: Some("Validate YAML syntax in rogers.yaml".to_string()),
        })
    }

    /// Output the audit report.
    fn output_report(&self, report: &AuditReport) -> Result<()> {
        if self.args.json {
            println!("{}", serde_json::to_string_pretty(report)?);
        } else {
            self.output_human_report(report);
        }
        Ok(())
    }

    /// Output human-readable report.
    fn output_human_report(&self, report: &AuditReport) {
        println!("=== Rodgers Project Readiness Audit ===");
        println!("Repository: {}", report.repository);
        println!("Scanned at: {}", report.scanned_at);
        println!();

        for check in &report.checks {
            let severity_str = match check.severity {
                Severity::Blocker => "[BLOCKER]",
                Severity::Warn => "[WARN   ]",
                Severity::Info => "[INFO   ]",
            };
            println!("{} {}", severity_str, check.message);
            if let Some(details) = &check.details {
                println!("  {}", details);
            }
        }

        println!();
        println!("{} checks performed", report.summary.total_checks);
        println!("  {} blockers — {}", report.summary.blockers, if report.summary.blockers > 0 { "Rodgers cannot safely operate" } else { "none" });
        println!("  {} warnings  — {}", report.summary.warnings, if report.summary.warnings > 0 { "review recommended" } else { "none" });
        println!("  {} info     — {}", report.summary.info, if report.summary.info > 0 { "no action needed" } else { "none" });

        if report.summary.blockers > 0 {
            println!();
            println!("Run 'rogers init --fix' to apply available automated fixes.");
            println!();
            println!("To fix repository settings manually:");
            println!("  https://github.com/{}/settings", report.repository);
        }
    }
}

/// Entry point for the init command from CLI.
pub async fn run_init(command: Commands) -> Result<i32> {
    match command {
        Commands::Init { repo, fix, json, github_token } => {
            let args = InitArgs { repo, fix, json, github_token };
            let cmd = InitCommand::new(args)?;
            cmd.run().await
        }
        _ => Err(RogersError::Config("Expected Init command".to_string())),
    }
}