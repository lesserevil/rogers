#![allow(dead_code)]

//! GitHub API client for issue and comment operations.
//!
//! This module provides the interface for interacting with the GitHub API.
//! The actual HTTP calls are delegated to the CLI layer, which manages
//! authentication and request handling.
//!
//! ## Design
//!
//! GitHub issue comments are needed for CRIT-6 (epic bead description) because
//! acceptance criteria can appear in both the issue body AND in comments on the
//! issue. Rodgers-generated criteria are posted in comments, and humans may
//! also modify or add criteria in their own comments.

use serde::{Deserialize, Serialize};

use crate::error::{Result, RogersError};

/// GitHub API client for Rodgers.
///
/// This client provides methods for fetching GitHub issue data needed for
/// epic bead creation. The actual HTTP transport is handled by the CLI layer.
pub struct GitHubClient {
    /// GitHub API base URL (default: https://api.github.com)
    api_base: String,
    /// Repository owner
    owner: String,
    /// Repository name
    repo: String,
    /// Authentication token
    token: Option<String>,
}

impl GitHubClient {
    /// Create a new GitHub client configured for a repository.
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            api_base: "https://api.github.com".to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            token: None,
        }
    }

    /// Create a client configured from environment variables.
    ///
    /// Reads GITHUB_OWNER, GITHUB_REPO, and GITHUB_TOKEN from environment.
    pub fn from_env() -> Result<Self> {
        let owner = std::env::var("GITHUB_OWNER")
            .map_err(|_| RogersError::Config("GITHUB_OWNER not set".to_string()))?;
        let repo = std::env::var("GITHUB_REPO")
            .map_err(|_| RogersError::Config("GITHUB_REPO not set".to_string()))?;
        let token = std::env::var("GITHUB_TOKEN").ok();

        Ok(Self {
            api_base: "https://api.github.com".to_string(),
            owner,
            repo,
            token,
        })
    }

    /// Set a custom API base URL.
    pub fn with_api_base(mut self, api_base: &str) -> Self {
        self.api_base = api_base.to_string();
        self
    }

    /// Set an authentication token.
    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    /// Build the API URL for fetching issue comments.
    fn comments_url(&self, issue_number: u64) -> String {
        format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.api_base, self.owner, self.repo, issue_number
        )
    }

    /// Build the API URL for fetching a single issue.
    fn issue_url(&self, issue_number: u64) -> String {
        format!(
            "{}/repos/{}/{}/issues/{}",
            self.api_base, self.owner, self.repo, issue_number
        )
    }

    /// Fetch all comments for a GitHub issue.
    ///
    /// Comments are returned in chronological order (oldest first).
    /// Returns an empty vector if the issue has no comments.
    ///
    /// The returned comments include Rodgers-generated criteria and
    /// any human-modified criteria that appear in comments.
    pub async fn fetch_issue_comments(&self, issue_number: u64) -> Result<Vec<GitHubComment>> {
        let url = self.comments_url(issue_number);
        let comments = self.fetch_json::<Vec<GitHubComment>>(&url).await?;
        Ok(comments)
    }

    /// Fetch a GitHub issue.
    pub async fn fetch_issue(&self, issue_number: u64) -> Result<GitHubIssue> {
        let url = self.issue_url(issue_number);
        let issue = self.fetch_json::<GitHubIssue>(&url).await?;
        Ok(issue)
    }

    /// Generic JSON fetch with auth headers.
    async fn fetch_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let client = reqwest::Client::new();

        let mut request = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message: body,
            });
        }

        let data = response.json::<T>().await?;
        Ok(data)
    }
}

/// A GitHub issue comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubComment {
    /// Comment ID
    pub id: u64,
    /// Comment body (the text content)
    pub body: String,
    /// Author username
    pub user: GitHubUser,
    /// When the comment was created
    pub created_at: String,
    /// When the comment was last updated
    pub updated_at: String,
}

/// A GitHub user (comment or issue author).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    /// Username
    pub login: String,
    /// User ID
    pub id: u64,
}

/// A GitHub issue (minimal representation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    /// Issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body (description)
    pub body: Option<String>,
    /// Author username
    pub user: GitHubUser,
    /// Labels applied to the issue
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
}

/// A GitHub label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    /// Label name
    pub name: String,
    /// Label color
    #[serde(default)]
    pub color: String,
}

/// GitHub issue state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

impl std::fmt::Display for IssueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueState::Open => write!(f, "open"),
            IssueState::Closed => write!(f, "closed"),
        }
    }
}

/// Fetch the state of a GitHub issue (open/closed) without fetching full data.
pub async fn issue_state(client: &GitHubClient, issue_number: u64) -> Result<Option<IssueState>> {
    let url = format!(
        "{}/repos/{}/{}/issues/{}",
        client.api_base, client.owner, client.repo, issue_number
    );

    let http_client = reqwest::Client::new();
    let mut request = http_client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if let Some(ref token) = client.token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await?;
    let status = response.status();

    if status.as_u16() == 404 {
        return Ok(None);
    }

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(RogersError::GitHubStatus {
            code: status.as_u16(),
            message: body,
        });
    }

    let issue_body = response.text().await?;
    let is_closed = issue_body.contains("\"state\":\"closed\"");

    Ok(Some(if is_closed {
        IssueState::Closed
    } else {
        IssueState::Open
    }))
}

/// Close a GitHub issue.
pub async fn close_issue(client: &GitHubClient, issue_number: u64) -> Result<()> {
    let url = format!(
        "{}/repos/{}/{}/issues/{}",
        client.api_base, client.owner, client.repo, issue_number
    );

    let http_client = reqwest::Client::new();
    let mut request = http_client
        .patch(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .body(r#"{"state":"closed"}"#);

    if let Some(ref token) = client.token {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(RogersError::GitHubStatus {
            code: status.as_u16(),
            message: body,
        });
    }

    Ok(())
}

// ============================================================================
// Release candidacy detection: merged PRs, tags, check runs
// ============================================================================

/// A GitHub repository tag (from the API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTag {
    /// Tag name (e.g. "v1.2.3")
    pub name: String,
    /// Commit SHA the tag points to
    pub commit: GitTagCommit,
    /// Full tag object URL
    pub zipball_url: String,
    #[serde(default)]
    pub tarball_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitTagCommit {
    pub sha: String,
    #[serde(default)]
    pub url: String,
}

/// A merged pull request (minimal fields needed for changelog/release).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedPR {
    /// PR number
    pub number: u64,
    /// PR title (parsed for conventional commit type)
    pub title: String,
    /// PR state
    pub state: String,
    /// Merge commit SHA
    pub merge_commit_sha: Option<String>,
    /// When the PR was merged (RFC3339)
    pub merged_at: Option<String>,
    /// Labels on the PR
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    /// PR author
    pub user: Option<GitHubUser>,
    /// Base branch (e.g. "main")
    pub base: GitHubPRRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPRRef {
    #[serde(rename = "ref")]
    pub ref_field: String,
    pub sha: String,
}

impl GitHubPRRef {
    /// Return the branch name.
    pub fn name(&self) -> &str {
        &self.ref_field
    }
}

/// A check run from the GitHub Checks API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRun {
    /// Check run ID
    pub id: u64,
    /// Human-readable name (e.g. "ci/cd")
    pub name: String,
    /// Current conclusion: "success", "failure", "skipped", "neutral", etc.
    pub conclusion: Option<String>,
    /// Current status: "completed", "in_progress", "queued"
    pub status: String,
}

/// A commit status from the GitHub commit statuses API (fallback for Checks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStatus {
    /// Status state: "success", "failure", "error", "pending"
    pub state: String,
    /// Context label (e.g. "ci/github-actions")
    pub context: String,
}

impl GitHubClient {
    // ---- Tags ----

    /// Build the API URL for listing repository tags.
    fn tags_url(&self) -> String {
        format!(
            "{}/repos/{}/{}/tags?per_page=100",
            self.api_base, self.owner, self.repo
        )
    }

    /// Fetch repository tags from the GitHub API.
    ///
    /// Returns tags sorted by most recently created first.
    /// Handles pagination by fetching up to 100 tags per page.
    pub async fn fetch_tags(&self) -> Result<Vec<GitTag>> {
        let url = self.tags_url();
        let tags = self.fetch_json::<Vec<GitTag>>(&url).await?;
        Ok(tags)
    }

    // ---- Merged Pull Requests ----

    /// Build the API URL for listing pull requests.
    fn pull_requests_url(
        &self,
        base_branch: &str,
        state: &str,
        sort: &str,
        direction: &str,
    ) -> String {
        format!(
            "{}/repos/{}/{}/pulls?base={}&state={}&sort={}&direction={}&per_page=100",
            self.api_base, self.owner, self.repo, base_branch, state, sort, direction
        )
    }

    /// Fetch merged pull requests for a given base branch.
    ///
    /// Returns PRs that have been merged, sorted by most recently merged first.
    /// Handles pagination automatically.
    pub async fn fetch_merged_prs(&self, base_branch: &str) -> Result<Vec<MergedPR>> {
        let mut all_prs = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/repos/{}/{}/pulls?base={}&state=closed&sort=updated&direction=desc&per_page=100&page={}",
                self.api_base, self.owner, self.repo, base_branch, page
            );
            let prs: Vec<MergedPR> = match self.fetch_json(&url).await {
                Ok(p) => p,
                Err(e) => {
                    if all_prs.is_empty() {
                        return Err(e);
                    }
                    break;
                }
            };

            if prs.is_empty() {
                break;
            }

            // Check if we've reached the last page before consuming prs
            let is_last_page = prs.len() < 100;

            // Filter to only merged PRs
            let merged: Vec<MergedPR> = prs
                .into_iter()
                .filter(|p| p.merge_commit_sha.is_some())
                .collect();

            all_prs.extend(merged);

            // If we got fewer than 100, we've reached the last page
            if is_last_page {
                break;
            }

            page += 1;

            // Safety valve: limit to 20 pages to avoid excessive API calls
            if page > 20 {
                break;
            }
        }

        Ok(all_prs)
    }

    // ---- Check Runs (GitHub Checks API) ----

    /// Fetch the check runs for a specific commit SHA.
    ///
    /// Uses the GitHub Checks API: GET /repos/{owner}/{repo}/commits/{sha}/check-runs
    pub async fn fetch_check_runs(&self, sha: &str) -> Result<Vec<CheckRun>> {
        let url = format!(
            "{}/repos/{}/{}/commits/{}/check-runs?per_page=100",
            self.api_base, self.owner, self.repo, sha
        );
        let wrapper: CheckRunsWrapper = self.fetch_json(&url).await?;
        Ok(wrapper.check_runs)
    }

    /// Fetch the commit statuses for a specific SHA (fallback for Checks API).
    ///
    /// Uses the GitHub commit statuses API: GET /repos/{owner}/{repo}/commits/{sha}/status
    pub async fn fetch_commit_statuses(&self, sha: &str) -> Result<Vec<CommitStatus>> {
        // The combined status endpoint returns a single CombinedStatus object
        let url = format!(
            "{}/repos/{}/{}/commits/{}/status",
            self.api_base, self.owner, self.repo, sha
        );
        let combined: CombinedStatus = self.fetch_json(&url).await?;
        Ok(combined.statuses)
    }

    // ---- Latest commit for a branch ----

    /// Fetch the latest commit SHA for a given branch.
    ///
    /// Uses: GET /repos/{owner}/{repo}/branches/{branch}
    pub async fn fetch_branch_head(&self, branch: &str) -> Result<BranchHead> {
        let url = format!(
            "{}/repos/{}/{}/branches/{}",
            self.api_base, self.owner, self.repo, branch
        );
        let branch_info: BranchHead = self.fetch_json(&url).await?;
        Ok(branch_info)
    }

    /// Check if CI is green on the given branch.
    ///
    /// Uses the GitHub Checks API as primary, with commit statuses as fallback.
    /// CI is considered "green" if:
    /// - There are completed check runs (not in_progress or queued)
    /// - All completed checks have a non-failure conclusion
    /// - No pending checks remain
    ///
    /// If no checks exist at all, returns `Ok(false)` (nothing to verify = not green).
    pub async fn is_ci_green(&self, branch: &str) -> Result<bool> {
        // First get the latest commit SHA for the branch
        let branch_head = match self.fetch_branch_head(branch).await {
            Ok(bh) => bh,
            Err(_) => return Ok(false),
        };

        let sha = &branch_head.commit.sha;

        // Try the Checks API first
        match self.fetch_check_runs(sha).await {
            Ok(checks) => {
                if checks.is_empty() {
                    // No check runs — fall back to commit statuses
                    return self.check_commit_statuses_green(sha).await;
                }

                // Check all runs
                for check in &checks {
                    if check.status == "completed" {
                        // A completed check with a failure conclusion means CI is red
                        if let Some(ref conclusion) = check.conclusion {
                            if conclusion == "failure"
                                || conclusion == "timed_out"
                                || conclusion == "startup_failure"
                            {
                                return Ok(false);
                            }
                        }
                        // If conclusion is None but status is completed, that's unusual
                        // — treat as not green to be safe
                        if check.conclusion.is_none() {
                            return Ok(false);
                        }
                    } else {
                        // Still in_progress or queued — CI hasn't finished yet
                        return Ok(false);
                    }
                }

                // All completed checks passed (success, skipped, neutral, cancelled)
                Ok(true)
            }
            Err(_) => {
                // Checks API failed — fall back to commit statuses
                self.check_commit_statuses_green(sha).await
            }
        }
    }

    /// Check commit statuses for green status (fallback method).
    async fn check_commit_statuses_green(&self, sha: &str) -> Result<bool> {
        let statuses = self.fetch_commit_statuses(sha).await?;

        if statuses.is_empty() {
            return Ok(false);
        }

        for status in &statuses {
            if status.state == "failure" || status.state == "error" {
                return Ok(false);
            }
            if status.state == "pending" {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Wrapper for the check runs API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckRunsWrapper {
    #[serde(rename = "check_runs")]
    check_runs: Vec<CheckRun>,
}

/// Combined commit status (from the combined status API).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CombinedStatus {
    #[serde(default)]
    statuses: Vec<CommitStatus>,
}

/// Branch head information (from the branches API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchHead {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "commit")]
    pub commit: BranchCommit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCommit {
    #[serde(rename = "sha")]
    pub sha: String,
}

// ============================================================================
// End of release candidacy additions
// ============================================================================

/// Parse an issue URL to extract owner, repo, and issue number.
pub fn parse_issue_url(url: &str) -> Option<(String, String, u64)> {
    let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();

    if parts.len() >= 5
        && let (Some(&owner), Some(&repo), Some(&issue_str), Some(&"issues")) = (
            parts.get(parts.len() - 4),
            parts.get(parts.len() - 3),
            parts.last(),
            parts.get(parts.len() - 2),
        )
        && let Ok(number) = issue_str.parse::<u64>()
    {
        return Some((owner.to_string(), repo.to_string(), number));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_client_new() {
        let client = GitHubClient::new("owner", "repo");
        assert_eq!(client.owner, "owner");
        assert_eq!(client.repo, "repo");
        assert_eq!(client.api_base, "https://api.github.com");
    }

    #[test]
    fn test_github_client_with_api_base() {
        let client =
            GitHubClient::new("owner", "repo").with_api_base("https://github.example.com/api/v3");
        assert_eq!(client.api_base, "https://github.example.com/api/v3");
    }

    #[test]
    fn test_github_client_with_token() {
        let client = GitHubClient::new("owner", "repo").with_token("ghp_test_token_12345");
        assert_eq!(client.token, Some("ghp_test_token_12345".to_string()));
    }

    #[test]
    fn test_comments_url_format() {
        let client = GitHubClient::new("myorg", "myrepo");
        let url = client.comments_url(42);
        assert!(url.contains("myorg"));
        assert!(url.contains("myrepo"));
        assert!(url.contains("42"));
        assert!(url.contains("/issues/"));
        assert!(url.contains("/comments"));
    }

    #[test]
    fn test_issue_url_format() {
        let client = GitHubClient::new("myorg", "myrepo");
        let url = client.issue_url(123);
        assert!(url.contains("myorg"));
        assert!(url.contains("myrepo"));
        assert!(url.contains("123"));
        assert!(url.contains("/issues/"));
    }

    #[test]
    fn test_github_comment_deserialization() {
        let json = r#"{
            "id": 1,
            "body": "Test comment body",
            "user": {
                "login": "testuser",
                "id": 123
            },
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let comment: GitHubComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, 1);
        assert_eq!(comment.body, "Test comment body");
        assert_eq!(comment.user.login, "testuser");
    }

    #[test]
    fn test_github_issue_deserialization() {
        let json = r#"{
            "number": 42,
            "title": "Test Issue",
            "body": "Issue body content",
            "user": {
                "login": "author",
                "id": 456
            },
            "labels": [
                {"name": "bug", "color": "ff0000"}
            ]
        }"#;

        let issue: GitHubIssue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 42);
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.body, Some("Issue body content".to_string()));
        assert_eq!(issue.labels.len(), 1);
        assert_eq!(issue.labels[0].name, "bug");
    }

    #[test]
    fn test_github_issue_with_no_body() {
        let json = r#"{
            "number": 42,
            "title": "Test Issue",
            "body": null,
            "user": {
                "login": "author",
                "id": 456
            },
            "labels": []
        }"#;

        let issue: GitHubIssue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.body, None);
    }

    #[test]
    fn test_github_user_deserialization() {
        let json = r#"{"login": "cli-user", "id": 999}"#;
        let user: GitHubUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "cli-user");
        assert_eq!(user.id, 999);
    }

    #[test]
    fn test_github_label_deserialization() {
        let json = r#"{"name": "enhancement", "color": "84b6eb"}"#;
        let label: GitHubLabel = serde_json::from_str(json).unwrap();
        assert_eq!(label.name, "enhancement");
        assert_eq!(label.color, "84b6eb");
    }

    // =============================================================================
    // Release candidacy: MergedPR deserialization tests
    // =============================================================================

    #[test]
    fn test_merged_pr_deserialization() {
        let json = r#"{
            "number": 42,
            "title": "feat: add login",
            "state": "closed",
            "merge_commit_sha": "abc123",
            "merged_at": "2024-01-15T10:00:00Z",
            "labels": [{"name": "enhancement", "color": "84b6eb"}],
            "base": {"ref": "main", "sha": "def456"}
        }"#;

        let pr: MergedPR = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "feat: add login");
        assert_eq!(pr.state, "closed");
        assert_eq!(pr.merge_commit_sha, Some("abc123".to_string()));
        assert_eq!(pr.base.name(), "main");
        assert_eq!(pr.labels.len(), 1);
    }

    #[test]
    fn test_merged_pr_no_merge_sha() {
        let json = r#"{
            "number": 43,
            "title": "fix: something",
            "state": "closed",
            "labels": [],
            "base": {"ref": "main", "sha": "def456"}
        }"#;

        let pr: MergedPR = serde_json::from_str(json).unwrap();
        assert!(pr.merge_commit_sha.is_none());
    }

    #[test]
    fn test_git_tag_deserialization() {
        let json = r#"{
            "name": "v1.2.3",
            "commit": {"sha": "abc123", "url": "https://api.github.com/repos/o/r/git/refs/tags/v1.2.3"},
            "zipball_url": "https://api.github.com/repos/o/r/zipball/v1.2.3",
            "tarball_url": "https://api.github.com/repos/o/r/tarball/v1.2.3"
        }"#;

        let tag: GitTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "v1.2.3");
        assert_eq!(tag.commit.sha, "abc123");
    }

    #[test]
    fn test_check_run_deserialization() {
        let json = r#"{
            "id": 12345,
            "name": "ci/cd",
            "conclusion": "success",
            "status": "completed"
        }"#;

        let check: CheckRun = serde_json::from_str(json).unwrap();
        assert_eq!(check.id, 12345);
        assert_eq!(check.name, "ci/cd");
        assert_eq!(check.conclusion, Some("success".to_string()));
        assert_eq!(check.status, "completed");
    }

    #[test]
    fn test_check_run_no_conclusion() {
        let json = r#"{
            "id": 12345,
            "name": "ci/cd",
            "status": "in_progress"
        }"#;

        let check: CheckRun = serde_json::from_str(json).unwrap();
        assert_eq!(check.conclusion, None);
        assert_eq!(check.status, "in_progress");
    }

    #[test]
    fn test_branch_head_deserialization() {
        let json = r#"{
            "name": "main",
            "commit": {"sha": "latest123"}
        }"#;

        let head: BranchHead = serde_json::from_str(json).unwrap();
        assert_eq!(head.name, "main");
        assert_eq!(head.commit.sha, "latest123");
    }

    #[test]
    fn test_tags_url_format() {
        let client = GitHubClient::new("myorg", "myrepo");
        let url = client.tags_url();
        assert!(url.contains("myorg"));
        assert!(url.contains("myrepo"));
        assert!(url.contains("/tags"));
        assert!(url.contains("per_page=100"));
    }
}
