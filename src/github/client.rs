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
}
