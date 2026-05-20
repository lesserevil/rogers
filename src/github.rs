//! GitHub API client for Rodgers.
//!
//! Thin wrapper around the GitHub REST API for reading and writing
//! issues, comments, labels, and other GitHub resources.

use crate::error::{Result, RogersError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// GitHub API configuration.
#[derive(Debug, Clone)]
pub struct GitHubConfig {
    /// GitHub API base URL.
    pub api_url: String,
    /// Repository owner (organization or user).
    pub owner: String,
    /// Repository name.
    pub repo: String,
    /// Personal access token.
    pub token: String,
}

/// GitHub issue data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: User,
    pub labels: Vec<Label>,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
}

/// GitHub user data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
}

/// GitHub label data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
}

/// GitHub comment data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub body: String,
    pub user: User,
    #[serde(rename = "created_at")]
    pub created_at: String,
}

/// GitHub API client.
#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
    config: GitHubConfig,
}

impl GitHubClient {
    /// Create a new GitHub client from configuration.
    pub fn new(config: GitHubConfig) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
            config,
        }
    }

    /// Get a webhook URL for GitHub App (if needed).
    /// Returns the base API URL for constructing full URLs.
    pub fn api_base(&self) -> String {
        format!(
            "{}/repos/{}/{}",
            self.config.api_url, self.config.owner, self.config.repo
        )
    }

    /// Get an issue by number.
    pub async fn get_issue(&self, number: u64) -> Result<Issue> {
        let url = format!("{}/issues/{}", self.api_base(), number);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Rodgers/0.1.0")
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(RogersError::RepoNotFound);
        }

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let issue: Issue = response.json().await?;
        Ok(issue)
    }

    /// Get comments on an issue.
    pub async fn get_comments(&self, issue_number: u64) -> Result<Vec<Comment>> {
        let url = format!("{}/issues/{}/comments", self.api_base(), issue_number);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Rodgers/0.1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let comments: Vec<Comment> = response.json().await?;
        Ok(comments)
    }

    /// Post a comment on an issue.
    pub async fn post_comment(&self, issue_number: u64, body: &str) -> Result<Comment> {
        let url = format!("{}/issues/{}/comments", self.api_base(), issue_number);

        #[derive(Serialize)]
        struct PostCommentRequest<'a> {
            body: &'a str,
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Rodgers/0.1.0")
            .json(&PostCommentRequest { body })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let comment: Comment = response.json().await?;
        Ok(comment)
    }

    /// Apply labels to an issue.
    pub async fn apply_labels(&self, issue_number: u64, labels: &[String]) -> Result<Vec<Label>> {
        let url = format!("{}/issues/{}/labels", self.api_base(), issue_number);

        #[derive(Serialize)]
        struct ApplyLabelsRequest<'a> {
            labels: &'a [String],
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Rodgers/0.1.0")
            .json(&ApplyLabelsRequest { labels })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let labels: Vec<Label> = response.json().await?;
        Ok(labels)
    }

    /// Remove labels from an issue.
    pub async fn remove_label(&self, issue_number: u64, label: &str) -> Result<()> {
        let url = format!(
            "{}/issues/{}/labels/{}",
            self.api_base(),
            issue_number,
            urlencoding::encode(label)
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Rodgers/0.1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(())
    }

    /// Close an issue.
    pub async fn close_issue(&self, issue_number: u64) -> Result<Issue> {
        let url = format!("{}/issues/{}", self.api_base(), issue_number);

        #[derive(Serialize)]
        struct CloseIssueRequest {
            state: String,
        }

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Rodgers/0.1.0")
            .json(&CloseIssueRequest {
                state: "closed".to_string(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let issue: Issue = response.json().await?;
        Ok(issue)
    }

    /// List issues with a specific label.
    pub async fn list_issues_with_label(&self, label: &str) -> Result<Vec<Issue>> {
        let url = format!(
            "{}/issues?labels={}",
            self.api_base(),
            urlencoding::encode(label)
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "Rodgers/0.1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        // GitHub returns both issues and PRs; filter to just issues
        #[derive(Deserialize)]
        struct IssueListResponse {
            #[serde(rename = "pull_request", default)]
            pull_request: Option<Value>,
            #[serde(flatten)]
            issue: Issue,
        }

        let items: Vec<IssueListResponse> = response.json().await?;
        let issues: Vec<Issue> = items
            .into_iter()
            .filter(|item| item.pull_request.is_none())
            .map(|item| item.issue)
            .collect();

        Ok(issues)
    }
}

// URL encoding helper
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut encoded = String::new();
        for c in input.chars() {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    encoded.push(c);
                }
                _ => {
                    for b in c.to_string().as_bytes() {
                        encoded.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        encoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encoding() {
        assert_eq!(urlencoding::encode("hello world"), "hello%20world");
        assert_eq!(urlencoding::encode("bug,feature"), "bug%2Cfeature");
        assert_eq!(
            urlencoding::encode("needs-documentation"),
            "needs-documentation"
        );
    }

    #[test]
    fn test_github_config() {
        let config = GitHubConfig {
            api_url: "https://api.github.com".to_string(),
            owner: "testorg".to_string(),
            repo: "testrepo".to_string(),
            token: "test-token".to_string(),
        };

        let client = GitHubClient::new(config);
        assert_eq!(
            client.api_base(),
            "https://api.github.com/repos/testorg/testrepo"
        );
    }

    #[test]
    fn test_issue_deserialization() {
        let json = r#"{
            "number": 123,
            "title": "Test issue",
            "body": "Issue body",
            "state": "open",
            "user": {"login": "testuser"},
            "labels": [{"name": "bug"}, {"name": "question"}],
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-02T00:00:00Z"
        }"#;

        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 123);
        assert_eq!(issue.title, "Test issue");
        assert_eq!(issue.user.login, "testuser");
        assert_eq!(issue.labels.len(), 2);
    }
}
