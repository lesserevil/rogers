//! GitHub API client for init checks.
//!
//! Centralizes all GitHub API calls, authentication, rate limit handling,
//! and error mapping for the Rodgers init audit system.

use reqwest::Client;
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::{Result, RogersError};
use crate::labels::LabelDefinition;

// ─── Constants ────────────────────────────────────────────────────────────

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_ACCEPT_HEADER: &str = "application/vnd.github+json";
const GITHUB_API_VERSION: &str = "2022-11-28";

// ─── Data Types ───────────────────────────────────────────────────────────

/// Repository metadata returned by the GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub html_url: String,
    pub default_branch: String,
    pub private: bool,
    pub has_issues: bool,
    pub has_wiki: bool,
    pub has_discussions: bool,
    pub size: u64,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: String,
    pub visibility: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_branch_on_merge: Option<bool>,
}

/// A GitHub label (response shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: u64,
    pub name: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub url: String,
}

/// Request body for creating a label via the GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLabelRequest {
    pub name: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Discussion category from the GitHub Discussions API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionCategory {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
    pub color: String,
    pub is_answerable: bool,
    pub created_at: String,
    pub repository_id: u64,
    pub slug: String,
}

/// Branch protection rule (response shape).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BranchProtection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_status_checks: Option<RequiredStatusChecks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_required_status_checks_policy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_pull_request_reviews: Option<RequiredPullRequestReviews>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_admins: Option<EnforceAdmins>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restrictions: Option<Restrictions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_conversation_resolution: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_linear_history: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_force_pushes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_deletions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_response: Option<BlockedResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_branch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_signatures: Option<RequiredSignatures>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredStatusChecks {
    pub strict: bool,
    pub contexts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredPullRequestReviews {
    pub dismiss_stale_reviews: bool,
    pub require_code_owner_reviews: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_approving_review_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dismissal_restrictions: Option<Restrictions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pull_request_review_bypass: Option<BypassAllowances>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforceAdmins {
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Restrictions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps_url: Option<String>,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub teams: Vec<String>,
    #[serde(default)]
    pub apps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedResponse {
    pub link: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredSignatures {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassAllowances {
    pub users_url: String,
    pub teams_url: String,
    pub apps_url: String,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub teams: Vec<String>,
    #[serde(default)]
    pub apps: Vec<String>,
}

/// Workflow from GitHub Actions API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
    pub html_url: String,
    pub badge_url: String,
}

/// Rate limit information from GitHub API response headers.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub reset: u64,
}

// ─── Client ───────────────────────────────────────────────────────────────

/// GitHub API client with connection pooling, authentication, and rate limit tracking.
#[derive(Clone)]
pub struct GitHubClient {
    client: Client,
    base_url: String,
    default_ref: String,
    token: String,
    rate_limit: Arc<Mutex<Option<RateLimitInfo>>>,
}

fn build_client(token: &str) -> Client {
    Client::builder()
        .user_agent("rogers/0.1.0")
        .default_headers({
            let mut headers = header::HeaderMap::new();
            headers.insert(
                header::ACCEPT,
                header::HeaderValue::from_static(GITHUB_ACCEPT_HEADER),
            );
            headers.insert(
                "X-GitHub-Api-Version",
                header::HeaderValue::from_static(GITHUB_API_VERSION),
            );
            if !token.is_empty() {
                headers.insert(
                    header::AUTHORIZATION,
                    header::HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
                );
            }
            headers
        })
        .build()
        .expect("failed to build reqwest client")
}

impl GitHubClient {
    /// Create a new GitHub client.
    ///
    /// # Arguments
    /// * `token` — GitHub personal access token. If empty, the client will
    ///   attempt to read `GITHUB_TOKEN` from the environment at request time.
    pub fn new(token: &str) -> Self {
        let token = if token.is_empty() {
            std::env::var("GITHUB_TOKEN").unwrap_or_default()
        } else {
            token.to_string()
        };

        GitHubClient {
            client: build_client(&token),
            base_url: GITHUB_API_BASE.to_string(),
            default_ref: "main".to_string(),
            token,
            rate_limit: Arc::new(Mutex::new(None)),
        }
    }

    /// Rebuild the client with a new base URL (useful for testing).
    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    /// Set the default git ref (branch/tag/SHA) for requests.
    pub fn with_default_ref(mut self, ref_name: &str) -> Self {
        self.default_ref = ref_name.to_string();
        self
    }

    /// Get the default git ref.
    #[allow(dead_code)]
    pub fn default_ref(&self) -> &str {
        &self.default_ref
    }

    /// Rebuild the client with a new token (e.g., from CLI flag vs env var).
    pub fn with_token(self, token: &str) -> Self {
        let token = if token.is_empty() {
            std::env::var("GITHUB_TOKEN").unwrap_or_default()
        } else {
            token.to_string()
        };

        GitHubClient {
            client: build_client(&token),
            base_url: self.base_url,
            default_ref: self.default_ref,
            token,
            rate_limit: self.rate_limit,
        }
    }

    /// Get the current rate limit info.
    pub async fn get_rate_limit(&self) -> Result<RateLimitInfo> {
        let guard = self.rate_limit.lock().await;
        if let Some(info) = guard.as_ref() {
            return Ok(info.clone());
        }
        drop(guard);

        // Fetch from the rate limit endpoint if not cached
        let resp = self
            .client
            .get(format!("{}/rate_limit", self.base_url))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(RogersError::GitHub)?;

        let rate_info = self.extract_rate_limit_info(&resp);
        // Cache the header-based rate limit info (more reliable than body parsing)
        let mut guard = self.rate_limit.lock().await;
        *guard = Some(rate_info.clone());
        Ok(rate_info)
    }

    /// Parse rate limit info from a JSON value (used in tests).
    #[allow(dead_code)]
    fn parse_rate_limit_json(json: &serde_json::Value) -> Result<RateLimitInfo> {
        let core = json.get("core").ok_or_else(|| {
            RogersError::Config("rate_limit response missing 'core' field".to_string())
        })?;
        let limit = core.get("limit").and_then(|v| v.as_u64()).unwrap_or(0);
        let remaining = core.get("remaining").and_then(|v| v.as_u64()).unwrap_or(0);
        let reset = core.get("reset").and_then(|v| v.as_u64()).unwrap_or(0);
        Ok(RateLimitInfo {
            limit,
            remaining,
            reset,
        })
    }

    /// Parse rate limit headers from any response and cache them.
    fn update_rate_limit_from_headers(&self, resp: &reqwest::Response) {
        let limit = resp
            .headers()
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let remaining = resp
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let reset = resp
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        if limit > 0 {
            let rate_limit = self.rate_limit.clone();
            tokio::spawn(async move {
                let mut guard = rate_limit.lock().await;
                *guard = Some(RateLimitInfo {
                    limit,
                    remaining,
                    reset,
                });
            });
        }
    }

    /// Send a GET request and return the text body, updating rate limit info from headers.
    async fn get_text(&self, url: &str) -> Result<String> {
        let resp = self.get_page(url).await?;
        // Extract rate limit headers before consuming resp (`.text()` takes ownership)
        let rate_info = self.extract_rate_limit_info(&resp);
        let text = resp.text().await.map_err(RogersError::GitHub)?;
        if rate_info.limit > 0 {
            let rl = self.rate_limit.clone();
            tokio::spawn(async move {
                let mut guard = rl.lock().await;
                *guard = Some(rate_info);
            });
        }
        Ok(text)
    }

    /// Extract rate limit info from response headers.
    fn extract_rate_limit_info(&self, resp: &reqwest::Response) -> RateLimitInfo {
        let limit = resp
            .headers()
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let remaining = resp
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let reset = resp
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        RateLimitInfo {
            limit,
            remaining,
            reset,
        }
    }

    /// Send a GET request and parse the text body as JSON.
    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let text = self.get_text(url).await?;
        let value: T = serde_json::from_str(&text)?;
        Ok(value)
    }

    /// Send a request and handle pagination for list endpoints.
    async fn request_with_pagination(&self, url: &str) -> Result<Vec<serde_json::Value>> {
        let mut all_items = Vec::new();
        let mut current_url = Some(url.to_string());

        while let Some(next_url) = current_url.take() {
            let text = self.get_text(&next_url).await?;
            let body: serde_json::Value = serde_json::from_str(&text)?;

            let items = match &body {
                serde_json::Value::Array(arr) => arr.clone(),
                serde_json::Value::Object(map) => {
                    // Check common GitHub API wrapper keys
                    if let Some(val) = map.get("items") {
                        if let Some(arr) = val.as_array() {
                            arr.clone()
                        } else {
                            vec![val.clone()]
                        }
                    } else if let Some(val) = map.get("workflows") {
                        if let Some(arr) = val.as_array() {
                            arr.clone()
                        } else {
                            vec![val.clone()]
                        }
                    } else if let Some(val) = map.get("categories") {
                        if let Some(arr) = val.as_array() {
                            arr.clone()
                        } else {
                            vec![val.clone()]
                        }
                    } else {
                        // Single object — return as single-item vec
                        vec![body]
                    }
                }
                _ => vec![body],
            };

            all_items.extend(items);

            // Check for next page in Link header
            current_url = self.next_page_url(&text);
        }

        Ok(all_items)
    }

    /// Get the next page URL from a Link header string.
    fn next_page_url(&self, link_header: &str) -> Option<String> {
        for part in link_header.split(',') {
            let part = part.trim();
            if part.contains("rel=\"next\"")
                && let Some(start) = part.find('<')
                && let Some(end) = part.find('>')
            {
                return Some(part[start + 1..end].to_string());
            }
        }
        None
    }

    /// Send a GET request and return the Response (caller owns it).
    async fn get_page(&self, url: &str) -> Result<reqwest::Response> {
        let full_url = if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}/{}", self.base_url, url.trim_start_matches('/'))
        };

        let mut req = self.client.get(&full_url);
        if !self.token.is_empty() {
            req = req.bearer_auth(&self.token);
        }

        let resp = req.send().await.map_err(RogersError::GitHub)?;

        // Check for rate limit exhaustion before processing other errors
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let reset = resp
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            return Err(RogersError::GitHubStatus {
                code: resp.status().as_u16(),
                message: format!(
                    "GitHub API rate limit exceeded. Resets at Unix timestamp {}",
                    reset
                ),
            });
        }

        if !resp.status().is_success() {
            let code = resp.status().as_u16();
            let message = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(RogersError::GitHubStatus { code, message });
        }

        Ok(resp)
    }

    // ─── Repository ─────────────────────────────────────────────────────

    /// Fetch repository metadata.
    ///
    /// # Arguments
    /// * `owner` — Repository owner (user or org name)
    /// * `repo` — Repository name
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        self.get_json(&format!("/repos/{}/{}", owner, repo)).await
    }

    // ─── Labels ─────────────────────────────────────────────────────────

    /// List all labels for a repository (handles pagination).
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    pub async fn list_labels(&self, owner: &str, repo: &str) -> Result<Vec<Label>> {
        let items = self
            .request_with_pagination(&format!("/repos/{}/{}/labels", owner, repo))
            .await?;
        let labels: Vec<Label> = items
            .into_iter()
            .filter_map(|item| serde_json::from_value(item).ok())
            .collect();
        Ok(labels)
    }

    /// Create a new label in a repository.
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    /// * `definition` — Label definition with name, color, description
    pub async fn create_label(
        &self,
        owner: &str,
        repo: &str,
        definition: &LabelDefinition,
    ) -> Result<Label> {
        let request = CreateLabelRequest {
            name: definition.name.to_string(),
            color: definition.color.to_string(),
            description: Some(definition.description.to_string()),
        };

        let url = format!("{}/repos/{}/{}/labels", self.base_url, owner, repo);
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .json(&request)
            .send()
            .await
            .map_err(RogersError::GitHub)?;

        self.update_rate_limit_from_headers(&resp);

        if !resp.status().is_success() {
            let code = resp.status().as_u16();
            let message = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(RogersError::GitHubStatus { code, message });
        }

        let text = resp.text().await.map_err(RogersError::GitHub)?;
        let label: Label = serde_json::from_str(&text)?;
        Ok(label)
    }

    // ─── Discussion Categories ──────────────────────────────────────────

    /// List all discussion categories for a repository (handles pagination).
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    pub async fn list_discussion_categories(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<DiscussionCategory>> {
        let items = self
            .request_with_pagination(&format!("/repos/{}/{}/discussion-categories", owner, repo))
            .await?;
        let categories: Vec<DiscussionCategory> = items
            .into_iter()
            .filter_map(|item| serde_json::from_value(item).ok())
            .collect();
        Ok(categories)
    }

    /// Create a new discussion category.
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    /// * `name` — Category name
    pub async fn create_discussion_category(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
    ) -> Result<DiscussionCategory> {
        let request = serde_json::json!({ "name": name });

        let text = self
            .client
            .post(format!("/repos/{}/{}/discussion-categories", owner, repo))
            .bearer_auth(&self.token)
            .json(&request)
            .send()
            .await
            .map_err(RogersError::GitHub)?
            .text()
            .await
            .map_err(RogersError::GitHub)?;

        let category: DiscussionCategory = serde_json::from_str(&text)?;
        Ok(category)
    }

    // ─── Branch Protection ──────────────────────────────────────────────

    /// Get branch protection rules for a specific branch.
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    /// * `branch` — Branch name
    pub async fn get_branch_protection(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<BranchProtection> {
        self.get_json(&format!(
            "/repos/{}/{}/branches/{}/protection",
            owner, repo, branch
        ))
        .await
    }

    // ─── Workflows ──────────────────────────────────────────────────────

    /// List all GitHub Actions workflows (handles pagination).
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    pub async fn list_workflows(&self, owner: &str, repo: &str) -> Result<Vec<Workflow>> {
        let items = self
            .request_with_pagination(&format!("/repos/{}/{}/actions/workflows", owner, repo))
            .await?;
        let workflows: Vec<Workflow> = items
            .into_iter()
            .filter_map(|item| serde_json::from_value(item).ok())
            .collect();
        Ok(workflows)
    }

    // ─── Directory Listing ──────────────────────────────────────────────

    /// List directory contents from a repository.
    ///
    /// Returns a vector of JSON values, each representing a file or directory
    /// entry in the specified path. Each entry contains at least:
    /// - `name`: file/directory name
    /// - `type`: "file" or "dir"
    /// - `path`: full path
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    /// * `path` — Directory path within the repository
    /// * `ref_name` — Git reference (branch, tag, or commit SHA)
    pub async fn list_directory(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let text = self
            .get_text(&format!(
                "/repos/{}/{}/contents/{}?ref={}",
                owner, repo, path, self.default_ref
            ))
            .await?;
        let body: serde_json::Value = serde_json::from_str(&text)?;

        match body {
            serde_json::Value::Array(arr) => Ok(arr),
            serde_json::Value::Object(_) => {
                // Single file returned instead of directory — treat as empty list
                // (the path exists but is a file, not a directory)
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    // ─── File Contents ──────────────────────────────────────────────────

    /// Get file contents from a repository.
    ///
    /// Returns the decoded content as a String.
    ///
    /// # Arguments
    /// * `owner` — Repository owner
    /// * `repo` — Repository name
    /// * `path` — File path within the repository
    /// * `ref` — Git reference (branch, tag, or commit SHA)
    pub async fn get_file_contents(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        ref_name: &str,
    ) -> Result<String> {
        let text = self
            .get_text(&format!(
                "/repos/{}/{}/contents/{}?ref={}",
                owner, repo, path, ref_name
            ))
            .await?;
        let body: serde_json::Value = serde_json::from_str(&text)?;

        match body {
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    return Err(RogersError::Plan(format!(
                        "Path '{}' resolved to empty directory",
                        path
                    )));
                }
                let file_name = path.rsplit('/').next().unwrap_or(path);
                if let Some(file) = arr
                    .iter()
                    .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(file_name))
                {
                    let content = file.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    Ok(base64_decode(content).unwrap_or_default())
                } else {
                    Err(RogersError::Plan(format!(
                        "File not found at path '{}'",
                        path
                    )))
                }
            }
            serde_json::Value::Object(_) => {
                let content = body
                    .get("content")
                    .and_then(|c| c.as_str())
                    .ok_or_else(|| {
                        RogersError::Plan(format!("No content field for path '{}'", path))
                    })?;
                Ok(base64_decode(content).unwrap_or_default())
            }
            _ => Err(RogersError::Plan(format!(
                "Unexpected response type for path '{}'",
                path
            ))),
        }
    }
}

/// Base64 decode helper (standard alphabet, GitHub uses standard base64 for file contents).
fn base64_decode(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return Some(String::new());
    }

    let decode_table: [u8; 256] = {
        let mut table = [255u8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for (i, &c) in chars.iter().enumerate() {
            table[c as usize] = i as u8;
        }
        // '=' is padding
        table[b'=' as usize] = 0;
        table
    };

    let mut bytes = Vec::new();
    let chars: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'\n' && b != b'\r' && b != b' ')
        .collect();
    let len = chars.len();

    if len == 0 {
        return Some(String::new());
    }

    let mut i = 0;
    while i < len {
        let b0 = decode_table[chars[i] as usize];
        let b1 = if i + 1 < len {
            decode_table[chars[i + 1] as usize]
        } else {
            0
        };
        let b2 = if i + 2 < len {
            decode_table[chars[i + 2] as usize]
        } else {
            0
        };
        let b3 = if i + 3 < len {
            decode_table[chars[i + 3] as usize]
        } else {
            0
        };

        if b0 > 63 || b1 > 63 {
            return None;
        }
        bytes.push((b0 << 2) | (b1 >> 4));

        if b2 > 63 {
            return None;
        }
        if chars[i + 2] == b'=' {
            break;
        }
        bytes.push(((b1 & 0x0F) << 4) | (b2 >> 2));

        if b3 > 63 {
            return None;
        }
        if chars[i + 3] == b'=' {
            break;
        }
        bytes.push(((b2 & 0x03) << 6) | b3);

        i += 4;
    }

    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_parsing() {
        let mock_body = serde_json::json!({
            "core": {
                "limit": 5000,
                "remaining": 4999,
                "reset": 1700000000
            }
        });

        let info = GitHubClient::parse_rate_limit_json(&mock_body).unwrap();
        assert_eq!(info.limit, 5000);
        assert_eq!(info.remaining, 4999);
        assert_eq!(info.reset, 1700000000);
    }

    #[tokio::test]
    async fn test_link_header_pagination_parsing() {
        let link_header =
            "<https://api.github.com/repos/test/test/labels?per_page=30&page=2>; rel=\"next\"";

        let next_url = GitHubClient::new("").next_page_url(link_header);

        assert_eq!(
            next_url,
            Some("https://api.github.com/repos/test/test/labels?per_page=30&page=2".to_string())
        );
    }

    #[tokio::test]
    async fn test_link_header_no_next() {
        let link_header =
            "<https://api.github.com/repos/test/test/labels?per_page=30&page=10>; rel=\"last\"";
        let next_url = GitHubClient::new("").next_page_url(link_header);
        assert!(next_url.is_none());
    }

    #[tokio::test]
    async fn test_link_header_multi_part() {
        let link_header = "<https://api.github.com/repos/test/test/labels?per_page=30&page=2>; rel=\"next\", <https://api.github.com/repos/test/test/labels?per_page=30&page=10>; rel=\"last\"";
        let next_url = GitHubClient::new("").next_page_url(link_header);
        assert_eq!(
            next_url,
            Some("https://api.github.com/repos/test/test/labels?per_page=30&page=2".to_string())
        );
    }

    #[tokio::test]
    async fn test_label_serialization() {
        let label_json = r#"{
            "id": 123,
            "name": "bug",
            "color": "d73a4a",
            "default": true,
            "description": "Bug report",
            "url": "https://api.github.com/repos/test/test/labels/bug"
        }"#;

        let label: Label = serde_json::from_str(label_json).expect("valid label JSON");
        assert_eq!(label.id, 123);
        assert_eq!(label.name, "bug");
        assert_eq!(label.color, "d73a4a");
        assert_eq!(label.default, Some(true));
        assert_eq!(label.description, Some("Bug report".to_string()));

        // Test serialization
        let serialized = serde_json::to_string(&label).expect("serialize label");
        assert!(serialized.contains("\"name\":\"bug\""));
    }

    #[tokio::test]
    async fn test_workflow_serialization() {
        let workflow_json = r#"{
            "id": 456,
            "name": "CI",
            "path": ".github/workflows/ci.yml",
            "state": "active",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-02T00:00:00Z",
            "url": "https://api.github.com/repos/test/test/actions/workflows/456",
            "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
            "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
        }"#;

        let workflow: Workflow = serde_json::from_str(workflow_json).expect("valid workflow JSON");
        assert_eq!(workflow.id, 456);
        assert_eq!(workflow.name, "CI");
        assert_eq!(workflow.state, "active");
        assert!(workflow.path.ends_with("ci.yml"));
    }

    #[tokio::test]
    async fn test_discussion_category_serialization() {
        let category_json = r#"{
            "id": 789,
            "name": "Release Proposals",
            "description": "Propose new releases",
            "emoji": "🚀",
            "emoji_name": "rocket",
            "color": "0075ca",
            "is_answerable": false,
            "created_at": "2024-01-01T00:00:00Z",
            "repository_id": 123,
            "slug": "release-proposals"
        }"#;

        let category: DiscussionCategory =
            serde_json::from_str(category_json).expect("valid category JSON");
        assert_eq!(category.id, 789);
        assert_eq!(category.name, "Release Proposals");
        assert_eq!(
            category.description,
            Some("Propose new releases".to_string())
        );
        assert_eq!(category.emoji, Some("🚀".to_string()));
        assert_eq!(category.color, "0075ca");
        assert!(!category.is_answerable);
    }

    #[tokio::test]
    async fn test_label_definition_creation() {
        let definition = LabelDefinition {
            name: "test-label",
            color: "ffffff",
            description: "A test label",
        };

        let request = CreateLabelRequest {
            name: definition.name.to_string(),
            color: definition.color.to_string(),
            description: Some(definition.description.to_string()),
        };

        let json = serde_json::to_string(&request).expect("serialize request");
        assert!(json.contains("\"name\":\"test-label\""));
        assert!(json.contains("\"color\":\"ffffff\""));
        assert!(json.contains("\"description\":\"A test label\""));
    }

    #[tokio::test]
    async fn test_base64_decode() {
        // "hello world" in base64
        let encoded = "aGVsbG8gd29ybGQ=";
        let decoded = base64_decode(encoded).expect("decode base64");
        assert_eq!(decoded, "hello world");
    }

    #[tokio::test]
    async fn test_base64_decode_empty() {
        let decoded = base64_decode("").expect("decode empty string");
        assert_eq!(decoded, "");
    }

    #[tokio::test]
    async fn test_base64_decode_multiline() {
        // Test that base64_decode handles multi-line encoded strings
        // (GitHub may wrap base64 at 76 chars). The encoded string itself
        // may contain newlines which we filter out during decoding.
        let encoded = "aGVsbG8gd29ybGQ=\n"; // "hello world=" with trailing newline
        let decoded = base64_decode(encoded).expect("decode multiline base64");
        assert_eq!(decoded, "hello world");
    }

    #[tokio::test]
    async fn test_repository_serialization() {
        let repo_json = r#"{
            "id": 999,
            "name": "test-repo",
            "full_name": "test-owner/test-repo",
            "description": "A test repository",
            "html_url": "https://github.com/test-owner/test-repo",
            "default_branch": "main",
            "private": false,
            "has_issues": true,
            "has_wiki": true,
            "has_discussions": true,
            "size": 1024,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-02T00:00:00Z",
            "pushed_at": "2024-01-03T00:00:00Z",
            "visibility": "public"
        }"#;

        let repo: Repository = serde_json::from_str(repo_json).expect("valid repo JSON");
        assert_eq!(repo.id, 999);
        assert_eq!(repo.name, "test-repo");
        assert_eq!(repo.full_name, "test-owner/test-repo");
        assert_eq!(repo.default_branch, "main");
        assert!(!repo.private);
        assert!(repo.has_issues);
        assert!(repo.has_discussions);
    }

    #[tokio::test]
    async fn test_create_discussion_category_request() {
        let request = serde_json::json!({ "name": "Release Proposals" });
        let json = serde_json::to_string(&request).expect("serialize request");
        assert!(json.contains("\"name\":\"Release Proposals\""));
    }

    #[tokio::test]
    async fn test_client_creates_with_empty_token() {
        let _client = GitHubClient::new("");
        // Should not panic
    }

    #[tokio::test]
    async fn test_client_creates_with_token() {
        let _client = GitHubClient::new("ghp_testtoken123");
        // Should not panic
    }

    #[tokio::test]
    async fn test_branch_protection_empty() {
        let empty_json = "{}";
        let bp: BranchProtection = serde_json::from_str(empty_json).expect("empty object");
        assert!(bp.url.is_none());
        assert!(bp.required_status_checks.is_none());
        assert!(bp.required_pull_request_reviews.is_none());
    }

    #[tokio::test]
    async fn test_branch_protection_with_data() {
        let json = r#"{
            "url": "https://api.github.com/repos/test/test/branches/main/protection",
            "required_status_checks": {
                "strict": true,
                "contexts": ["ci/build"]
            },
            "enforce_admins": {
                "url": "https://api.github.com/repos/test/test/branches/main/protection/enforce_admins",
                "enabled": true
            },
            "allow_force_pushes": false,
            "allow_deletions": false,
            "required_pull_request_reviews": {
                "dismiss_stale_reviews": true,
                "require_code_owner_reviews": false,
                "required_approving_review_count": 1
            }
        }"#;

        let bp: BranchProtection = serde_json::from_str(json).expect("valid bp JSON");
        assert!(bp.url.is_some());
        let checks = bp.required_status_checks.unwrap();
        assert!(checks.strict);
        assert_eq!(checks.contexts, vec!["ci/build".to_string()]);
        let admins = bp.enforce_admins.unwrap();
        assert!(admins.enabled);
        let reviews = bp.required_pull_request_reviews.unwrap();
        assert!(reviews.dismiss_stale_reviews);
        assert_eq!(reviews.required_approving_review_count, Some(1));
    }

    #[tokio::test]
    async fn test_get_file_contents_parses_single_file() {
        // Simulate a GitHub API response for a single file
        let content = "hello world";
        let encoded = base64_encode(content);
        let json = format!(
            r#"{{"name":"README.md","path":"README.md","sha":"abc123","content":"{}","encoding":"base64"}}"#,
            encoded
        );

        let body: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(body.is_object());
        let file_content = body.get("content").and_then(|c| c.as_str()).unwrap();
        assert_eq!(base64_decode(file_content).unwrap(), content);
    }

    #[tokio::test]
    async fn test_get_file_contents_parses_directory() {
        // Simulate a GitHub API response for a directory
        let content = "init content";
        let encoded = base64_encode(content);
        let json = format!(
            "[{{\"name\":\"INIT\",\"path\":\"INIT\",\"sha\":\"abc123\",\"content\":\"{}\",\"encoding\":\"base64\"}},{{\"name\":\"other\",\"path\":\"other\",\"sha\":\"def456\"}}]",
            encoded
        );

        let body: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(body.is_array());
        let arr = body.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let first = &arr[0];
        let file_content = first.get("content").and_then(|c| c.as_str()).unwrap();
        assert_eq!(base64_decode(file_content).unwrap(), content);
    }

    #[tokio::test]
    async fn test_client_with_token_rebuild() {
        let client1 = GitHubClient::new("token1");
        let client2 = client1.with_token("token2");
        assert_eq!(client2.token, "token2");
    }
}

#[allow(dead_code)]
/// Helper for tests: base64 encode a string.
fn base64_encode(input: &str) -> String {
    let encode_table: [u8; 64] =
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            bytes[i + 2] as u32
        } else {
            0
        };

        result.push(encode_table[(b0 >> 2) as usize] as char);
        result
            .push(encode_table[((b0 & 0x03) << 4) as usize + ((b1 >> 4) & 0x0F) as usize] as char);

        if i + 1 < bytes.len() {
            result.push(
                encode_table[((b1 & 0x0F) << 2) as usize + ((b2 >> 6) & 0x03) as usize] as char,
            );
        } else {
            result.push('=');
        }

        if i + 2 < bytes.len() {
            result.push(encode_table[b2 as usize & 0x3F] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}
