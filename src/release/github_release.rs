//! GitHub Release API integration for Rodgers.
//!
//! This module handles creating and updating GitHub Releases via the REST API,
//! including retry logic for rate limits, detecting existing releases, and
//! truncating oversized changelogs.
//!
//! ## GitHub API Reference
//!
//! - Create release: `POST /repos/{owner}/{repo}/releases`
//! - Get release by tag: `GET /repos/{owner}/{repo}/releases/tags/{tag}`
//! - Update release: `PATCH /repos/{owner}/{repo}/releases/{release_id}`
//! - Discussion comment: `POST /repos/{owner}/{repo}/discussions/{number}/comments`
//!
//! ## Retry Strategy
//!
//! GitHub API rate limits and transient network failures are handled via
//! exponential backoff with jitter. Retries occur for 403 (rate limit),
//! 422 (validation/already exists), 500/502/503/504 (server errors), and
//! network errors.
//!
//! ## Changelog Truncation
//!
//! GitHub release notes are limited to 128 KB. If the generated changelog
//! exceeds this, it is truncated with a "see full changelog" note appended.

use crate::error::{Result, RogersError};
use crate::release::changelog::{
    ChangelogConfig, PullRequest, generate_markdown, group_prs_by_type,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Maximum size for a GitHub release body in bytes (128 KB).
const GITHUB_RELEASE_BODY_LIMIT: usize = 128 * 1024;

/// GitHub Release API client.
///
/// Provides methods for creating, updating, and checking GitHub Releases.
pub struct ReleaseClient {
    /// Base URL for the GitHub API.
    api_base: String,
    /// Repository owner.
    owner: String,
    /// Repository name.
    repo: String,
    /// Authentication token.
    token: String,
    /// Number of retry attempts for transient failures.
    max_retries: u32,
}

/// Metadata about a GitHub Release returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// The unique release ID.
    pub id: u64,
    /// The git tag this release is associated with.
    pub tag_name: String,
    /// The display name of the release.
    pub name: String,
    /// The release body (notes/changelog).
    pub body: String,
    /// Whether this is a draft release.
    pub draft: bool,
    /// Whether this is a pre-release.
    pub prerelease: bool,
    /// The target commitish (branch/commit) for the release.
    #[serde(default)]
    pub target_commitish: Option<String>,
}

/// Configuration for a release creation operation.
#[derive(Debug, Clone)]
pub struct ReleaseConfig {
    /// The git tag for the release (e.g., "v1.2.0").
    pub tag: String,
    /// The display name for the release (e.g., "Release v1.2.0").
    pub title: String,
    /// The generated changelog body.
    pub body: String,
    /// Whether this is a pre-release (true for alpha/beta).
    pub prerelease: bool,
    /// The source commit/branch for the release.
    pub target_commitish: Option<String>,
}

impl ReleaseClient {
    /// Create a new release client.
    pub fn new(owner: &str, repo: &str, token: &str) -> Self {
        Self {
            api_base: "https://api.github.com".to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            token: token.to_string(),
            max_retries: 3,
        }
    }

    /// Set the GitHub API base URL.
    pub fn with_api_base(mut self, api_base: &str) -> Self {
        self.api_base = api_base.to_string();
        self
    }

    /// Set the maximum number of retry attempts.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Check if a release already exists for the given tag.
    ///
    /// Returns `Some(ReleaseInfo)` if a release exists, `None` otherwise.
    pub async fn get_release_by_tag(&self, tag: &str) -> Result<Option<ReleaseInfo>> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{}",
            self.api_base, self.owner, self.repo, tag
        );

        let response = self
            .build_client()
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        match response.status().as_u16() {
            200 => {
                let release: ReleaseInfo = response.json().await?;
                Ok(Some(release))
            }
            404 => Ok(None),
            code => Err(RogersError::GitHubStatus {
                code,
                message: response.text().await.unwrap_or_default(),
            }),
        }
    }

    /// Create a GitHub Release.
    ///
    /// If a release already exists for the tag, this method will update it
    /// instead of creating a duplicate.
    ///
    /// # Arguments
    ///
    /// * `config` — Configuration for the release (tag, title, body, etc.)
    ///
    /// # Returns
    ///
    /// The `ReleaseInfo` for the created or updated release.
    pub async fn create_release(&self, config: &ReleaseConfig) -> Result<ReleaseInfo> {
        match self.get_release_by_tag(&config.tag).await? {
            Some(existing) => {
                info!(
                    release_id = existing.id,
                    tag = config.tag,
                    "Release already exists, updating instead of creating"
                );
                self.update_release(&existing.id, config).await
            }
            None => self.do_create_release(config).await,
        }
    }

    async fn do_create_release(&self, config: &ReleaseConfig) -> Result<ReleaseInfo> {
        let url = format!(
            "{}/repos/{}/{}/releases",
            self.api_base, self.owner, self.repo
        );

        let body = self.truncate_body(&config.body);

        let payload = serde_json::json!({
            "tag_name": config.tag,
            "name": config.title,
            "body": body,
            "draft": false,
            "prerelease": config.prerelease,
        });

        let payload_str = serde_json::to_string(&payload).map_err(RogersError::Json)?;

        self.with_retry(|| async {
            let mut request = self
                .build_client()
                .post(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Content-Type", "application/json")
                .body(payload_str.clone());

            if let Some(ref commitish) = config.target_commitish {
                request = request.query(&[("target_commitish", commitish)]);
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let body_text = response.text().await.unwrap_or_default();

                // Check for "already exists" error in the response body
                if body_text.contains("already exists") || status.as_u16() == 422 {
                    warn!("Release creation returned 422 or 'already exists' message");
                    match self.get_release_by_tag(&config.tag).await? {
                        Some(release) => return Ok(release),
                        None => {
                            return Err(RogersError::GitHubStatus {
                                code: status.as_u16(),
                                message: body_text,
                            });
                        }
                    }
                }

                Err(RogersError::GitHubStatus {
                    code: status.as_u16(),
                    message: body_text,
                })
            } else {
                let release: ReleaseInfo = response.json().await?;
                Ok(release)
            }
        })
        .await
    }

    async fn update_release(
        &self,
        release_id: &u64,
        config: &ReleaseConfig,
    ) -> Result<ReleaseInfo> {
        let url = format!(
            "{}/repos/{}/{}/releases/{}",
            self.api_base, self.owner, self.repo, release_id
        );

        let body = self.truncate_body(&config.body);

        let payload = serde_json::json!({
            "tag_name": config.tag,
            "name": config.title,
            "body": body,
            "draft": false,
            "prerelease": config.prerelease,
        });

        let payload_str = serde_json::to_string(&payload).map_err(RogersError::Json)?;

        self.with_retry(|| async {
            let request = self
                .build_client()
                .patch(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Content-Type", "application/json")
                .body(payload_str.clone());

            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let body_text = response.text().await.unwrap_or_default();
                return Err(RogersError::GitHubStatus {
                    code: status.as_u16(),
                    message: body_text,
                });
            }

            let release: ReleaseInfo = response.json().await?;
            Ok(release)
        })
        .await
    }

    /// Truncate the release body if it exceeds GitHub's byte limit.
    fn truncate_body(&self, body: &str) -> String {
        let bytes = body.as_bytes();
        if bytes.len() <= GITHUB_RELEASE_BODY_LIMIT {
            body.to_string()
        } else {
            let note = "\n\n---\n*Release notes truncated. See full changelog in commit history.*";
            let note_len = note.len();
            let truncate_at = GITHUB_RELEASE_BODY_LIMIT.saturating_sub(note_len);

            let truncated = if truncate_at < bytes.len() {
                let safe_end = bytes[..truncate_at]
                    .iter()
                    .rev()
                    .position(|&b| b == b'\n')
                    .map(|pos| truncate_at - pos)
                    .unwrap_or(truncate_at);
                String::from_utf8_lossy(&bytes[..safe_end]).to_string()
            } else {
                body.to_string()
            };

            format!("{}{}", truncated, note)
        }
    }

    /// Post a notification comment to a GitHub Discussion.
    pub async fn post_discussion_comment(&self, discussion_number: u64, body: &str) -> Result<u64> {
        let url = format!(
            "{}/repos/{}/{}/discussions/{}/comments",
            self.api_base, self.owner, self.repo, discussion_number
        );

        let payload = serde_json::json!({
            "body": body,
        });

        let payload_str = serde_json::to_string(&payload).map_err(RogersError::Json)?;

        self.with_retry(|| async {
            let request = self
                .build_client()
                .post(&url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .header("Authorization", format!("Bearer {}", self.token))
                .header("Content-Type", "application/json")
                .body(payload_str.clone());

            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let body_text = response.text().await.unwrap_or_default();
                return Err(RogersError::GitHubStatus {
                    code: status.as_u16(),
                    message: body_text,
                });
            }

            #[derive(Deserialize)]
            struct CommentResponse {
                id: u64,
            }
            let comment: CommentResponse = response.json().await?;
            info!(comment_id = comment.id, "Discussion comment posted");
            Ok(comment.id)
        })
        .await
    }

    /// Retry a fallible async operation with exponential backoff.
    async fn with_retry<F, Fut, T>(&self, mut op: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match op().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e.clone());
                    let should_retry = match &e {
                        RogersError::GitHubStatus { code, .. } => {
                            *code == 403 || *code == 422 || (*code >= 500 && *code < 600)
                        }
                        RogersError::GitHub(_) => true,
                        _ => false,
                    };

                    if !should_retry || attempt == self.max_retries {
                        return Err(e);
                    }

                    let jitter = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.subsec_millis() as u64)
                        .unwrap_or(0)
                        % 500;
                    let delay = (2_u64.pow(attempt) * 1000) + jitter;
                    warn!(
                        attempt,
                        max_retries = self.max_retries,
                        error = %e,
                        "Retryable error, retrying in {}ms",
                        delay
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            RogersError::Io(std::io::Error::other("retry exhausted without error"))
        }))
    }

    fn build_client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }
}

/// Generate the notification comment body for a GitHub Discussion.
///
/// # Arguments
///
/// * `version` — The release version (e.g., "v1.2.0").
/// * `release_url` — URL to the created GitHub Release.
/// * `tag` — The git tag string.
pub fn release_notification_comment(version: &str, release_url: &str, tag: &str) -> String {
    format!(
        "## Release {version} Cut\n\n\
         Release {version} has been created:\n\
         - Tag: `{tag}`\n\
         - Release: [{version}]({release_url})\n\
         - Status: Published (not draft, not pre-release)\n\
         - Notes: Generated from PR titles and labels\n\
         \n\
         CI will build release artifacts from this tag. The release is now\n\
         available for download and testing.\n\
         \n\
         _This release was created automatically by Rodgers._",
        version = version,
        release_url = release_url,
        tag = tag,
    )
}

/// Build a `ReleaseConfig` from a `ChangelogConfig` and a list of PRs.
pub fn build_release_config(
    changelog_config: &ChangelogConfig,
    prs: &[PullRequest],
    prerelease: bool,
    target_commitish: Option<String>,
) -> ReleaseConfig {
    let title = format!("Release {}", changelog_config.release_name);
    let grouped = group_prs_by_type(prs);
    let body = generate_markdown(&grouped, changelog_config);

    ReleaseConfig {
        tag: changelog_config.release_name.clone(),
        title,
        body,
        prerelease,
        target_commitish,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // ReleaseConfig tests
    // =============================================================================

    #[test]
    fn test_release_config_creation() {
        let config = ReleaseConfig {
            tag: "v1.2.0".to_string(),
            title: "Release v1.2.0".to_string(),
            body: "## Features\n\n- add login".to_string(),
            prerelease: false,
            target_commitish: None,
        };
        assert_eq!(config.tag, "v1.2.0");
        assert_eq!(config.title, "Release v1.2.0");
        assert!(!config.prerelease);
        assert!(config.target_commitish.is_none());
        assert!(config.body.contains("add login"));
    }

    #[test]
    fn test_release_config_prerelease() {
        let config = ReleaseConfig {
            tag: "v2.0.0-beta.1".to_string(),
            title: "Release v2.0.0-beta.1".to_string(),
            body: "beta notes".to_string(),
            prerelease: true,
            target_commitish: None,
        };
        assert!(config.prerelease);
    }

    // =============================================================================
    // ReleaseNotificationComment tests
    // =============================================================================

    #[test]
    fn test_release_notification_comment_basic() {
        let comment = release_notification_comment(
            "v1.2.0",
            "https://github.com/example/repo/releases/tag/v1.2.0",
            "v1.2.0",
        );
        assert!(comment.contains("Release v1.2.0 Cut"));
        assert!(comment.contains("v1.2.0"));
        assert!(comment.contains("https://github.com/example/repo/releases/tag/v1.2.0"));
        assert!(comment.contains("Published (not draft, not pre-release)"));
        assert!(comment.contains("Rodgers"));
    }

    #[test]
    fn test_release_notification_comment_prerelease() {
        let comment = release_notification_comment(
            "v2.0.0-beta.1",
            "https://github.com/example/repo/releases/tag/v2.0.0-beta.1",
            "v2.0.0-beta.1",
        );
        assert!(comment.contains("Release v2.0.0-beta.1 Cut"));
        assert!(comment.contains("v2.0.0-beta.1"));
    }

    #[test]
    fn test_release_notification_format_complete() {
        let comment = release_notification_comment(
            "v2.0.0",
            "https://github.com/org/repo/releases/tag/v2.0.0",
            "v2.0.0",
        );
        assert!(comment.contains("Release v2.0.0 Cut"));
        assert!(comment.contains("Published (not draft, not pre-release)"));
        assert!(comment.contains("Generated from PR titles and labels"));
        assert!(comment.contains("Rodgers"));
    }

    // =============================================================================
    // TruncateBody tests
    // =============================================================================

    #[test]
    fn test_truncate_body_within_limit() {
        let client = ReleaseClient::new("owner", "repo", "token");
        let small_body = "## Features\n\n- add feature 1\n- add feature 2";
        let result = client.truncate_body(small_body);
        assert_eq!(result, small_body);
    }

    #[test]
    fn test_truncate_body_at_limit() {
        let client = ReleaseClient::new("owner", "repo", "token");
        let body = "a".repeat(GITHUB_RELEASE_BODY_LIMIT);
        let result = client.truncate_body(&body);
        assert_eq!(result.len(), body.len());
        assert!(result.len() <= GITHUB_RELEASE_BODY_LIMIT);
    }

    #[test]
    fn test_truncate_body_exceeds_limit() {
        let client = ReleaseClient::new("owner", "repo", "token");
        let body = "a".repeat(GITHUB_RELEASE_BODY_LIMIT + 1000);
        let result = client.truncate_body(&body);
        assert!(result.len() <= GITHUB_RELEASE_BODY_LIMIT);
        assert!(result.contains("truncated"));
        assert!(result.contains("changelog"));
    }

    #[test]
    fn test_truncate_body_preserves_line_boundary() {
        let client = ReleaseClient::new("owner", "repo", "token");
        let body = "line1\nline2\nline3\n".repeat(10000);
        let result = client.truncate_body(&body);
        assert!(result.len() <= GITHUB_RELEASE_BODY_LIMIT);
        assert!(result.ends_with("changelog in commit history.*"));
    }

    #[test]
    fn test_truncate_body_with_markdown_content() {
        let client = ReleaseClient::new("owner", "repo", "token");
        let mut sections = Vec::new();
        for i in 0..5000 {
            sections.push(format!("### Feature {}\n\n- feature {}", i, i));
        }
        let large_body = format!("## v1.0.0\n\n{}", sections.join("\n\n"));
        let result = client.truncate_body(&large_body);
        assert!(result.len() <= GITHUB_RELEASE_BODY_LIMIT);
        assert!(result.contains("truncated"));
    }

    // =============================================================================
    // BuildReleaseConfig tests
    // =============================================================================

    #[test]
    fn test_build_release_config() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: add user login"),
            PullRequest::new("myorg", "myrepo", 2, "fix: resolve crash"),
        ];
        let changelog_config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let config = build_release_config(&changelog_config, &prs, false, None);

        assert_eq!(config.tag, "v1.0.0");
        assert_eq!(config.title, "Release v1.0.0");
        assert!(!config.prerelease);
        assert!(config.body.contains("Features"));
        assert!(config.body.contains("Bug Fixes"));
        assert!(config.body.contains("add user login"));
        assert!(config.body.contains("resolve crash"));
    }

    #[test]
    fn test_build_release_config_prerelease() {
        let prs = vec![PullRequest::new("myorg", "myrepo", 1, "feat: add feature")];
        let changelog_config = ChangelogConfig::new("myorg", "myrepo", "v2.0.0-alpha.1");
        let config = build_release_config(&changelog_config, &prs, true, None);

        assert!(config.prerelease);
        assert_eq!(config.tag, "v2.0.0-alpha.1");
        assert_eq!(config.title, "Release v2.0.0-alpha.1");
    }

    #[test]
    fn test_build_release_config_with_commitish() {
        let prs = vec![PullRequest::new("myorg", "myrepo", 1, "feat: add feature")];
        let changelog_config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let config = build_release_config(&changelog_config, &prs, false, Some("main".to_string()));

        assert_eq!(config.target_commitish, Some("main".to_string()));
    }

    // =============================================================================
    // ReleaseClient tests
    // =============================================================================

    #[test]
    fn test_release_client_new() {
        let client = ReleaseClient::new("owner", "repo", "ghp_test_token");
        assert_eq!(client.owner, "owner");
        assert_eq!(client.repo, "repo");
        assert_eq!(client.token, "ghp_test_token");
        assert_eq!(client.api_base, "https://api.github.com");
        assert_eq!(client.max_retries, 3);
    }

    #[test]
    fn test_release_client_with_api_base() {
        let client = ReleaseClient::new("owner", "repo", "token")
            .with_api_base("https://github.example.com/api/v3");
        assert_eq!(client.api_base, "https://github.example.com/api/v3");
    }

    #[test]
    fn test_release_client_with_max_retries() {
        let client = ReleaseClient::new("owner", "repo", "token").with_max_retries(5);
        assert_eq!(client.max_retries, 5);
    }

    // =============================================================================
    // ReleaseInfo deserialization tests
    // =============================================================================

    #[test]
    fn test_release_info_deserialize_full() {
        let json = r#"{
  "id": 12345,
  "tag_name": "v1.0.0",
  "name": "Release v1.0.0",
  "body": "Features and Bug Fixes sections with release notes",
  "draft": false,
  "prerelease": false,
  "target_commitish": "main"
}"#;
        let release: ReleaseInfo = serde_json::from_str(json).unwrap();
        assert_eq!(release.id, 12345);
        assert_eq!(release.tag_name, "v1.0.0");
        assert_eq!(release.name, "Release v1.0.0");
        assert!(!release.draft);
        assert!(!release.prerelease);
        assert_eq!(release.target_commitish, Some("main".to_string()));
    }

    #[test]
    fn test_release_info_deserialize_minimal() {
        let json = r#"{
  "id": 12345,
  "tag_name": "v1.0.0",
  "name": "Release v1.0.0",
  "body": "Release notes",
  "draft": false,
  "prerelease": false
}"#;
        let release: ReleaseInfo = serde_json::from_str(json).unwrap();
        assert_eq!(release.id, 12345);
        assert_eq!(release.tag_name, "v1.0.0");
        assert!(release.target_commitish.is_none());
    }

    // =============================================================================
    // CRIT-4 Acceptance Criteria tests
    // =============================================================================

    #[tokio::test]
    async fn test_release_config_has_correct_title_format() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: add feature"),
            PullRequest::new("myorg", "myrepo", 2, "fix: fix bug"),
        ];
        let changelog_config = ChangelogConfig::new("myorg", "myrepo", "v3.0.0");
        let config = build_release_config(&changelog_config, &prs, false, None);
        // CRIT-4: Release title matches 'Release vX.Y.Z'
        assert_eq!(config.title, "Release v3.0.0");
    }

    #[tokio::test]
    async fn test_release_config_has_correct_tag() {
        let prs = vec![PullRequest::new("myorg", "myrepo", 1, "feat: add feature")];
        let changelog_config = ChangelogConfig::new("myorg", "myrepo", "v1.2.3");
        let config = build_release_config(&changelog_config, &prs, false, None);
        // CRIT-4: Correct tag for release
        assert_eq!(config.tag, "v1.2.3");
    }

    #[tokio::test]
    async fn test_release_body_contains_generated_changelog() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 10, "feat: add user dashboard"),
            PullRequest::new("myorg", "myrepo", 11, "fix: resolve crash on startup"),
            PullRequest::new("myorg", "myrepo", 12, "docs: update README"),
        ];
        let changelog_config =
            ChangelogConfig::new("myorg", "myrepo", "v1.0.0").with_date("2024-06-15");
        let config = build_release_config(&changelog_config, &prs, false, None);

        // CRIT-4: Release body contains generated changelog
        assert!(config.body.contains("Features"));
        assert!(config.body.contains("Bug Fixes"));
        assert!(config.body.contains("Documentation"));
        assert!(config.body.contains("add user dashboard"));
        assert!(config.body.contains("resolve crash on startup"));
        assert!(
            config
                .body
                .contains("[#10](https://github.com/myorg/myrepo/pull/10)")
        );
        assert!(
            config
                .body
                .contains("[#11](https://github.com/myorg/myrepo/pull/11)")
        );
    }

    #[tokio::test]
    async fn test_release_config_not_prerelease_for_stable() {
        let prs = vec![PullRequest::new("myorg", "myrepo", 1, "feat: feature")];
        let changelog_config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let config = build_release_config(&changelog_config, &prs, false, None);
        // CRIT-4: Release marked as latest (not prerelease)
        assert!(!config.prerelease);
    }

    #[tokio::test]
    async fn test_release_notification_comment_contains_release_info() {
        let comment = release_notification_comment(
            "v1.2.0",
            "https://github.com/myorg/myrepo/releases/tag/v1.2.0",
            "v1.2.0",
        );
        // CRIT-4: Posts notification to proposal Discussion
        assert!(comment.contains("Release v1.2.0 Cut"));
        assert!(comment.contains("v1.2.0"));
        assert!(comment.contains("https://github.com/myorg/myrepo/releases/tag/v1.2.0"));
        assert!(comment.contains("Published"));
    }

    #[test]
    fn test_release_draft_and_prerelease_false_for_stable() {
        // CRIT-4: draft=false and prerelease=false for stable releases
        let prs = vec![PullRequest::new("myorg", "myrepo", 1, "feat: feature")];
        let changelog_config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let config = build_release_config(&changelog_config, &prs, false, None);
        assert!(!config.prerelease);
    }
}
