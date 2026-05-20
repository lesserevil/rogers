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

/// GitHub issue state (backward compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

    /// Backward compatibility: Create a GitHub client with token and API URL.
    ///
    /// This signature matches the old interface used by doctor module.
    pub fn compat_new(token: String, api_url: Option<&str>) -> Self {
        Self {
            api_base: api_url
                .map(String::from)
                .unwrap_or_else(|| "https://api.github.com".to_string()),
            owner: String::new(),
            repo: String::new(),
            token: Some(token),
        }
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

    // ===== Backward compatibility methods (for doctor module) =====

    /// Backward compatibility: Get the state of a GitHub issue
    ///
    /// Returns `Ok(Some(IssueState))` if the issue exists.
    /// Returns `Ok(None)` if the issue is not found (deleted).
    /// Returns `Err` on API errors.
    pub async fn get_issue_state(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Option<IssueState>> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.api_base, owner, repo, issue_number
        );

        let response = self
            .fetch_json_raw(&url)
            .await?;

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

        let body: serde_json::Value = response.json().await?;
        let state_str = body.get("state").and_then(|s| s.as_str()).unwrap_or("open");
        let state = match state_str {
            "closed" => IssueState::Closed,
            _ => IssueState::Open,
        };

        Ok(Some(state))
    }

    /// Backward compatibility: Parse a GitHub issue URL and extract owner, repo, and issue number
    pub fn parse_issue_url(url: &str) -> Option<(String, String, u64)> {
        let url = url.split('#').next().unwrap_or(url);

        if let Ok(parsed) = url::Url::parse(url) {
            if parsed.host_str() == Some("github.com") {
                let segments: Vec<&str> = parsed.path_segments()?.collect();
                if segments.len() >= 4 && segments[2] == "issues" {
                    let owner = segments[0].to_string();
                    let repo = segments[1].to_string();
                    let num_str = segments[3];
                    let num_str = num_str.strip_suffix(".git").unwrap_or(num_str);
                    if let Ok(num) = num_str.parse() {
                        return Some((owner, repo, num));
                    }
                }
            }
        }

        let parts: Vec<&str> = url.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 4 && parts[2] == "issues" {
            let owner = parts[1].to_string();
            let repo = parts[3].to_string();
            let num_str = parts[4];
            let num_str = num_str.strip_suffix(".git").unwrap_or(num_str);
            if let Ok(num) = num_str.parse() {
                return Some((owner, repo, num));
            }
        }

        None
    }

    /// Backward compatibility: Extract issue number from a GitHub issue URL
    pub fn extract_issue_number(url: &str) -> Option<u64> {
        let url = url.split('#').next().unwrap_or(url);

        if let Ok(parsed) = url::Url::parse(url) {
            if parsed.host_str() == Some("github.com") {
                let segments: Vec<&str> = parsed.path_segments()?.collect();
                if segments.len() >= 4 && segments[2] == "issues" {
                    return segments[3].parse().ok();
                }
            }
        }

        if let Some(last_segment) = url.rsplit('/').next() {
            if last_segment.starts_with("issues/") {
                let num_str = last_segment.strip_prefix("issues/")?;
                return num_str.parse().ok();
            }
        }

        None
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

    /// Raw fetch for backward compatibility (returns response for status checking)
    async fn fetch_json_raw(&self, url: &str) -> Result<reqwest::Response> {
        let client = reqwest::Client::new();

        let mut request = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "RogersDoctor/0.1");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        Ok(response)
    }
}

/// Backward compatibility: Attempt to close an issue via the GitHub API
pub async fn close_issue(
    owner: &str,
    repo: &str,
    issue_number: u64,
    token: &str,
    api_url: Option<&str>,
) -> Result<()> {
    let base_url = api_url
        .map(String::from)
        .unwrap_or_else(|| "https://api.github.com".to_string());

    let client = reqwest::Client::new();
    let url = format!(
        "{}/repos/{}/{}/issues/{}",
        base_url, owner, repo, issue_number
    );

    let response = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "RogersDoctor/0.1")
        .json(&serde_json::json!({ "state": "closed" }))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body: serde_json::Value = response.json().await.unwrap_or_default();
        let msg = body
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(RogersError::GitHubStatus {
            code: status.as_u16(),
            message: msg.to_string(),
        });
    }

    Ok(())
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

    // Backward compatibility tests
    #[test]
    fn test_parse_issue_url() {
        let result = GitHubClient::parse_issue_url("https://github.com/test-owner/test-repo/issues/42");
        assert_eq!(result, Some(("test-owner".into(), "test-repo".into(), 42)));
    }

    #[test]
    fn test_parse_issue_url_with_anchor() {
        let result = GitHubClient::parse_issue_url(
            "https://github.com/owner/repo/issues/123#issuecomment-456",
        );
        assert_eq!(result, Some(("owner".into(), "repo".into(), 123)));
    }

    #[test]
    fn test_extract_issue_number() {
        assert_eq!(
            GitHubClient::extract_issue_number("https://github.com/owner/repo/issues/123"),
            Some(123)
        );
        assert_eq!(
            GitHubClient::extract_issue_number("https://github.com/owner/repo/issues/456#comment"),
            Some(456)
        );
    }

    #[test]
    fn test_issue_state_display() {
        assert_eq!(IssueState::Open.to_string(), "open");
        assert_eq!(IssueState::Closed.to_string(), "closed");
    }
}