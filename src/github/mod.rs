//! GitHub API client for Rodgers
//!
//! Provides methods for interacting with GitHub issues and other API endpoints.

use crate::error::{Result, RogersError};
use serde::{Deserialize, Serialize};

/// GitHub issue state
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

/// GitHub issue information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub number: u64,
    pub state: IssueState,
    pub title: String,
    pub html_url: String,
}

/// GitHub API client
pub struct GitHubClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl GitHubClient {
    /// Create a new GitHub API client
    pub fn new(token: String, api_url: Option<&str>) -> Self {
        let base_url = api_url
            .map(String::from)
            .unwrap_or_else(|| "https://api.github.com".to_string());

        Self {
            client: reqwest::Client::new(),
            base_url,
            token,
        }
    }

    /// Get the state of a GitHub issue
    ///
    /// Returns `Ok(Some(IssueState))` if the issue exists.
    /// Returns `Ok(None)` if the issue is not found (deleted).
    /// Returns `Err` on API errors (after retries).
    pub async fn get_issue_state(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
    ) -> Result<Option<IssueState>> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.base_url, owner, repo, issue_number
        );

        // Retry up to 3 times on transient failures
        let mut last_error = None;
        for attempt in 0..3 {
            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "RogersDoctor/0.1")
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let body: serde_json::Value = match resp.json().await {
                        Ok(b) => b,
                        Err(e) => {
                            // If we can't parse the response, try again
                            last_error = Some(RogersError::GitHub(e));
                            if attempt < 2 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(
                                    100 * (attempt + 1) as u64,
                                ))
                                .await;
                                continue;
                            }
                            return Err(last_error.unwrap());
                        }
                    };

                    // Issue deleted (404) - treat as closed
                    if status.as_u16() == 404 {
                        return Ok(None);
                    }

                    if !status.is_success() {
                        let msg = body
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error");
                        return Err(RogersError::GitHubStatus {
                            code: status.as_u16(),
                            message: msg.to_string(),
                        });
                    }

                    // Parse the issue state
                    let state_str = body.get("state").and_then(|s| s.as_str()).unwrap_or("open");

                    let state = match state_str {
                        "closed" => IssueState::Closed,
                        _ => IssueState::Open,
                    };

                    return Ok(Some(state));
                }
                Err(e) => {
                    last_error = Some(RogersError::GitHub(e));
                    if attempt < 2 {
                        // Retry on network errors
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * (attempt + 1) as u64,
                        ))
                        .await;
                        continue;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Extract issue number from a GitHub issue URL
    ///
    /// Supports URLs like:
    /// - https://github.com/owner/repo/issues/123
    /// - https://github.com/owner/repo/issues/123#comment
    pub fn extract_issue_number(url: &str) -> Option<u64> {
        // Strip trailing stuff first
        let url = url.split('#').next().unwrap_or(url);

        // Try to parse as URL first for better handling
        if let Ok(parsed) = url::Url::parse(url) {
            if parsed.host_str() == Some("github.com") {
                let segments: Vec<&str> = parsed.path_segments()?.collect();
                // Path is /owner/repo/issues/123 → segments = ["owner", "repo", "issues", "123"]
                if segments.len() >= 4 && segments[2] == "issues" {
                    return segments[3].parse().ok();
                }
            }
        }

        // Fall back to simple parsing: look for /issues/ followed by digits at end
        // Pattern: any/path/issues/NUMBER
        if let Some(last_segment) = url.rsplit('/').next() {
            if last_segment.starts_with("issues/") {
                let num_str = last_segment.strip_prefix("issues/")?;
                return num_str.parse().ok();
            }
        }

        None
    }

    /// Parse a GitHub issue URL and extract owner, repo, and issue number
    pub fn parse_issue_url(url: &str) -> Option<(String, String, u64)> {
        // Strip trailing anchor/comment
        let url = url.split('#').next().unwrap_or(url);

        // Parse URL or just extract from path
        // Expected formats:
        // - https://github.com/owner/repo/issues/123
        // - owner/repo/issues/123

        // Try to parse as full URL first
        if let Ok(parsed) = url::Url::parse(url) {
            if parsed.host_str() == Some("github.com") {
                let segments: Vec<&str> = parsed.path_segments()?.collect();
                // Path is /owner/repo/issues/123 → segments = ["owner", "repo", "issues", "123"]
                if segments.len() >= 4 && segments[2] == "issues" {
                    let owner = segments[0].to_string();
                    let repo = segments[1].to_string();
                    let num_str = segments[3];
                    // Remove .git suffix if present
                    let num_str = num_str.strip_suffix(".git").unwrap_or(num_str);
                    if let Ok(num) = num_str.parse() {
                        return Some((owner, repo, num));
                    }
                }
            }
        }

        // Fall back to path-only parsing
        let parts: Vec<&str> = url.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 4 && parts[2] == "issues" {
            // URL like: github.com/owner/repo/issues/123
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
}

/// Attempt to close an issue via the GitHub API
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

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(
            GitHubClient::extract_issue_number("https://github.com/owner/repo/issues/789.diff"),
            None
        );

        assert_eq!(
            GitHubClient::extract_issue_number("not-a-url/issues/123"),
            None
        );
    }

    #[test]
    fn test_parse_issue_url_github_https() {
        let result =
            GitHubClient::parse_issue_url("https://github.com/test-owner/test-repo/issues/42");
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
    fn test_parse_issue_url_invalid() {
        assert_eq!(GitHubClient::parse_issue_url("https://google.com/"), None);
        assert_eq!(
            GitHubClient::parse_issue_url("https://github.com/owner/repo/pull/123"),
            None
        );
    }

    #[test]
    fn test_issue_state_display() {
        assert_eq!(IssueState::Open.to_string(), "open");
        assert_eq!(IssueState::Closed.to_string(), "closed");
    }
}
