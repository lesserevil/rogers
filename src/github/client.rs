//! Thin GitHub REST API client wrapping `reqwest`.
//!
//! All GitHub communication flows through this module.

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::GithubConfig;
use crate::RogersError;

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct GithubClient {
    client: Client,
    config: Arc<GithubConfig>,
}

impl GithubClient {
    pub fn new(config: GithubConfig) -> Self {
        let client = Client::builder()
            .user_agent("rodgers/0.1")
            .build()
            .expect("reqwest client must build");
        Self {
            client,
            config: Arc::new(config),
        }
    }

    /// Return a reference to the GitHub configuration.
    #[allow(dead_code)]
    pub fn config(&self) -> &GithubConfig {
        &self.config
    }

    /// Return a reference to the reqwest client (for GraphQL requests).
    #[allow(dead_code)]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Return the Authorization header value.
    #[allow(dead_code)]
    pub fn auth_header(&self) -> String {
        format!(
            "Bearer {}",
            self.config.token.as_ref().unwrap_or(&String::new())
        )
    }

    fn base_url(&self) -> String {
        format!(
            "{}/repos/{}/{}",
            self.config.api_url, self.config.owner, self.config.repo
        )
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, RogersError> {
        let url = format!("{}{}", self.base_url(), path);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header())
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        let v: T = resp.json().await.map_err(RogersError::GitHub)?;
        Ok(v)
    }

    async fn get_paginated<T: DeserializeOwned>(&self, path: &str) -> Result<Vec<T>, RogersError> {
        let url = format!("{}{}", self.base_url(), path);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header())
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }
        let v: Vec<T> = resp.json().await.map_err(RogersError::GitHub)?;
        Ok(v)
    }

    // ---------------------------------------------------------------------------
    // Merged PRs
    // ---------------------------------------------------------------------------

    /// Fetch merged pull requests since the given timestamp (ISO 8601).
    /// Used by the triage loop to find candidate commits.
    pub async fn merged_prs_since(&self, since: &str) -> Result<Vec<MergedPr>, RogersError> {
        let path = "/pulls?state=closed&sort=updated&direction=desc&per_page=100";
        #[derive(Deserialize)]
        struct PrWrapper {
            merged_at: Option<String>,
            #[serde(flatten)]
            inner: MergedPr,
        }
        let wrappers: Vec<PrWrapper> = self.get_paginated(&path).await?;
        let merged: Vec<MergedPr> = wrappers
            .into_iter()
            .filter(|w| {
                if let Some(ref at) = w.merged_at {
                    at.as_str() >= since
                } else {
                    false
                }
            })
            .map(|w| w.inner)
            .collect();
        Ok(merged)
    }

    /// Fetch a single merged pull request by number.
    pub async fn merged_pr(&self, number: u64) -> Result<MergedPr, RogersError> {
        let path = format!("/pulls/{number}");
        self.get(&path).await
    }

    // ---------------------------------------------------------------------------
    // Issues
    // ---------------------------------------------------------------------------

    /// Fetch a single issue.
    pub async fn issue(&self, number: u64) -> Result<GithubIssue, RogersError> {
        let path = format!("/issues/{number}");
        self.get(&path).await
    }

    /// Fetch all labels on an issue.
    pub async fn issue_labels(&self, number: u64) -> Result<Vec<GithubLabel>, RogersError> {
        let path = format!("/issues/{number}/labels");
        self.get_paginated(&path).await
    }

    // ---------------------------------------------------------------------------
    // Advisories (for security patch detection)
    // ---------------------------------------------------------------------------

    /// Fetch repository security advisories (GHSA).
    pub async fn advisories(&self) -> Result<Vec<GithubAdvisory>, RogersError> {
        let path = "/security-advisories?per_page=100";
        self.get_paginated(&path).await
    }

    // ---------------------------------------------------------------------------
    // Branches
    // ---------------------------------------------------------------------------

    /// List all branches.
    pub async fn branches(&self) -> Result<Vec<GithubBranch>, RogersError> {
        let path = "/branches?per_page=100";
        self.get_paginated(path).await
    }

    /// Check if a branch exists.
    pub async fn branch_exists(&self, name: &str) -> Result<bool, RogersError> {
        let path = format!("/branches/{name}");
        let result: Result<GithubBranch, _> = self.get(&path).await;
        match result {
            Ok(_) => Ok(true),
            Err(RogersError::GitHubStatus { code: 404, .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    // ---------------------------------------------------------------------------
    // Discussions
    // ---------------------------------------------------------------------------

    /// Create a GitHub Discussion for backport approval.
    ///
    /// Uses the category configured in `config.release.approval_discussion_category`.
    /// The discussion body is the approval prompt; reactions (👍/👎) are the vote.
    ///
    /// Returns the URL to the created discussion.
    pub async fn create_discussion(
        &self,
        category_id: &str,
        title: &str,
        body: &str,
    ) -> Result<Discussion, RogersError> {
        #[derive(Serialize)]
        struct CreateDiscussionRequest {
            category_id: String,
            title: String,
            body: String,
        }

        #[derive(Deserialize)]
        struct DiscussionWrapper {
            #[serde(flatten)]
            inner: Discussion,
        }

        let path = "/discussions";
        let request = CreateDiscussionRequest {
            category_id: category_id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
        };

        let url = format!("{}{}", self.base_url(), path);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.auth_header())
            .header("Accept", "application/vnd.github+json")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let code = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(RogersError::GitHubStatus { code, message });
        }

        let wrapper: DiscussionWrapper = resp.json().await.map_err(RogersError::GitHub)?;
        Ok(wrapper.inner)
    }

    /// List discussion categories to find the category ID by name.
    pub async fn discussion_categories(&self) -> Result<Vec<DiscussionCategory>, RogersError> {
        let path = "/discussions/categories";
        self.get_paginated(path).await
    }
}

// ---------------------------------------------------------------------------
// API data types
// ---------------------------------------------------------------------------

/// A merged pull request — backport candidate source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergedPr {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub merged_at: Option<String>,
    pub merge_commit_sha: Option<String>,
    pub user: GithubUser,
    pub labels: Vec<GithubLabel>,
    pub state: String,
}

/// A GitHub issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubIssue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<GithubLabel>,
    pub assignees: Vec<GithubUser>,
    pub user: GithubUser,
    pub created_at: String,
    pub updated_at: String,
}

/// A GitHub user (could be human or bot).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubUser {
    pub login: String,
    #[serde(rename = "type")]
    pub user_type: String,
}

/// A GitHub label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubLabel {
    pub name: String,
    pub color: String,
}

/// A security advisory (GHSA).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubAdvisory {
    pub ghsa_id: String,
    pub severity: Option<String>,
    pub cve_id: Option<String>,
    pub summary: Option<String>,
}

/// A GitHub branch reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubBranch {
    pub name: String,
    pub sha: String,
    pub protected: bool,
}

/// A GitHub Discussion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discussion {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub url: String,
    pub html_url: String,
    pub category: DiscussionCategory,
    /// When the discussion was created (ISO 8601).
    pub created_at: String,
}

/// A GitHub Discussion category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionCategory {
    pub id: u64,
    pub name: String,
    pub slug: String,
}
