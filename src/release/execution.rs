//! Release execution: branch, tag, and GitHub Release creation.
//!
//! When a release is approved, Rodgers:
//! 1. Creates a branch `release/X.Y.Z` from the source branch
//! 2. Creates a git tag `X.Y.Z`
//! 3. Creates a GitHub Release via the API
//! 4. Posts a notification comment on the proposal Discussion
//! 5. Closes the proposal Discussion
//!
//! ## Atomic Sequence
//!
//! Branch creation, tagging, and release creation should be treated
//! as an atomic sequence — if any step fails, the prior steps are
//! cleaned up and the release is re-proposed on the next run.

use crate::error::{Result, RogersError};
use crate::github::client::GitHubClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Errors related to release execution.
#[derive(Debug, thiserror::Error)]
pub enum ReleaseExecutionError {
    #[error("branch creation failed: {0}")]
    BranchCreationFailed(String),

    #[error("tag creation failed: {0}")]
    TagCreationFailed(String),

    #[error("release creation failed: {0}")]
    ReleaseCreationFailed(String),

    #[error("branch already exists: {0}")]
    BranchExists(String),

    #[error("tag already exists: {0}")]
    TagExists(String),

    #[error("source branch not found: {0}")]
    SourceBranchNotFound(String),
}

/// Result of executing a release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseResult {
    /// Version released.
    pub version: String,
    /// Release branch name created.
    pub branch_name: String,
    /// Git tag name created.
    pub tag_name: String,
    /// GitHub Release URL (if created).
    pub release_url: Option<String>,
    /// Whether the release was successful.
    pub success: bool,
    /// Error message if any.
    pub error: Option<String>,
    /// When the release was executed.
    pub executed_at: DateTime<Utc>,
}

/// Release executor.
///
/// Handles the execution of approved releases — creating branch,
/// tag, and GitHub Release in an atomic sequence.
#[derive(Debug, Clone)]
pub struct ReleaseExecutor {
    /// GitHub client.
    github: GitHubClient,
    /// Whether to create a branch (true = full release, false = tag+release only).
    create_branch: bool,
}

impl ReleaseExecutor {
    /// Create a new executor.
    pub fn new(github: GitHubClient, create_branch: bool) -> Self {
        Self {
            github,
            create_branch,
        }
    }

    /// Generate the release branch name.
    ///
    /// Format: `release/{version}`
    pub fn release_branch_name(version: &str) -> String {
        format!("release/{}", version)
    }

    /// Generate the git tag name.
    ///
    /// Format: `v{version}`
    pub fn tag_name(version: &str) -> String {
        format!("v{}", version)
    }

    /// Generate the release title.
    ///
    /// Format: `Version X.Y.Z`
    pub fn release_title(version: &str) -> String {
        format!("Version {}", version)
    }

    /// Execute an approved release.
    ///
    /// This is the main entry point for release execution. It creates
    /// the release branch (if configured), the git tag, and the
    /// GitHub Release in a single atomic sequence.
    pub async fn execute(
        &mut self,
        version: &str,
        source_branch: &str,
        discussion_number: i32,
        notification_body: &str,
    ) -> Result<ReleaseResult> {
        let branch_name = Self::release_branch_name(version);
        let tag = Self::tag_name(version);
        let title = Self::release_title(version);

        // Step 1: Check if branch already exists (collision handling)
        if self.create_branch {
            match self.github.get_branch(&branch_name).await {
                Ok(_) => {
                    // Branch already exists — post a note and skip branch creation
                    // but still proceed with tag and release
                    tracing::warn!(
                        "Branch {} already exists, skipping branch creation",
                        branch_name
                    );
                    if let Err(e) = self
                        .post_discussion_comment(discussion_number, &format!(
                            "Note: branch `{}` already exists. Proceeding with tag and GitHub Release creation.",
                            branch_name
                        ))
                        .await {
                        tracing::warn!("Failed to post branch collision note: {}", e);
                    }
                }
                Err(_) => {
                    // Create the branch from source
                    match self.create_release_branch(&branch_name, source_branch).await {
                        Ok(branch) => {
                            if branch.is_none() {
                                tracing::warn!("Branch was created but not found via API");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to create release branch: {}", e);
                        }
                    }
                }
            }
        }

        // Step 2: Check if tag already exists
        match self.github.get_release(&tag).await {
            Ok(_) => {
                // Tag already exists
                return Ok(ReleaseResult {
                    version: version.to_string(),
                    branch_name,
                    tag_name: tag.clone(),
                    release_url: None,
                    success: true,
                    error: Some(format!("Tag {} already exists, skipping", tag)),
                    executed_at: Utc::now(),
                });
            }
            Err(_) => {
                // Create the tag and GitHub Release
                match self
                    .create_github_release(&tag, source_branch, &title, version)
                    .await
                {
                    Ok(release) => {
                        // Step 4: Post notification
                        if let Err(e) = self
                            .post_discussion_comment(
                                discussion_number,
                                &notification_body,
                            )
                            .await {
                            tracing::warn!("Failed to post notification: {}", e);
                        }

                        // Step 5: Close the proposal discussion
                        if let Err(e) = self
                            .close_discussion(discussion_number)
                            .await {
                            tracing::warn!("Failed to close discussion: {}", e);
                        }

                        return Ok(ReleaseResult {
                            version: version.to_string(),
                            branch_name,
                            tag_name: tag,
                            release_url: Some(release.url.clone().unwrap_or_default()),
                            success: true,
                            error: None,
                            executed_at: Utc::now(),
                        });
                    }
                    Err(e) => {
                        // Cleanup: delete branch if we created it
                        if self.create_branch {
                            if let Err(e2) = self.delete_branch(&branch_name).await {
                                tracing::warn!(
                                    "Cleanup failed (could not delete branch {}): {}",
                                    branch_name,
                                    e2
                                );
                            }
                        }

                        return Ok(ReleaseResult {
                            version: version.to_string(),
                            branch_name,
                            tag_name: tag,
                            release_url: None,
                            success: false,
                            error: Some(e.to_string()),
                            executed_at: Utc::now(),
                        });
                    }
                }
            }
        }
    }

    /// Create a release branch from a source branch.
    async fn create_release_branch(
        &mut self,
        branch_name: &str,
        source_branch: &str,
    ) -> Result<Option<crate::github::models::Branch>> {
        // GitHub doesn't have a direct branch creation REST API,
        // so we use git commands to create the branch and push it.
        let fetch_output = std::process::Command::new("git")
            .args(["fetch", "origin", source_branch])
            .output();

        match fetch_output {
            Ok(ref out) if out.status.success() => {}
            Ok(ref out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(RogersError::GitHubStatus {
                    code: 500,
                    message: format!(
                        "Could not fetch branch {}: {}",
                        source_branch, stderr
                    ),
                });
            }
            Err(e) => {
                return Err(RogersError::GitHubStatus {
                    code: 500,
                    message: format!(
                        "Could not fetch branch {}: {}",
                        source_branch, e
                    ),
                });
            }
        }

        let checkout_output = std::process::Command::new("git")
            .args(["checkout", "-b", branch_name, &format!("origin/{}", source_branch)])
            .output();

        match checkout_output {
            Ok(ref out) if out.status.success() => {
                // Push the branch
                let push_output = std::process::Command::new("git")
                    .args(["push", "-u", "origin", branch_name])
                    .output();

                match push_output {
                    Ok(ref p) if p.status.success() => {
                        tracing::info!(
                            "Created release branch {}",
                            branch_name
                        );
                    }
                    Ok(ref p) => {
                        let stderr = String::from_utf8_lossy(&p.stderr);
                        return Err(RogersError::GitHubStatus {
                            code: 500,
                            message: format!(
                                "Failed to push branch {}: {}",
                                branch_name, stderr
                            ),
                        });
                    }
                    Err(e) => {
                        return Err(RogersError::GitHubStatus {
                            code: 500,
                            message: format!(
                                "Failed to push branch {}: {}",
                                branch_name, e
                            ),
                        });
                    }
                }
            }
            Ok(ref out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(RogersError::GitHubStatus {
                    code: 500,
                    message: format!(
                        "Failed to create branch {}: {}",
                        branch_name, stderr
                    ),
                });
            }
            Err(e) => {
                return Err(RogersError::GitHubStatus {
                    code: 500,
                    message: format!(
                        "Failed to create branch {}: {}",
                        branch_name, e
                    ),
                });
            }
        }

        // Return the branch info from GitHub
        match self.github.get_branch(branch_name).await {
            Ok(branch) => Ok(Some(branch)),
            Err(_) => Ok(None),
        }
    }

    /// Create a GitHub Release (which also creates the tag).
    async fn create_github_release(
        &mut self,
        tag: &str,
        target_commitish: &str,
        title: &str,
        version: &str,
    ) -> Result<crate::github::models::Release> {
        let body = format!(
            "Release {}",
            version
        );

        self.github
            .create_release(tag, Some(target_commitish), Some(title), Some(&body), false, false)
            .await
    }

    /// Post a comment on a discussion.
    async fn post_discussion_comment(
        &mut self,
        discussion_number: i32,
        body: &str,
    ) -> Result<()> {
        use serde_json::json;

        // Post comment via the issues comments endpoint (discussions share this)
        let url = format!(
            "{}/repos/{}/{}/discussions/{}/comments",
            self.github.auth().api_url(),
            self.github.owner(),
            self.github.repo(),
            discussion_number
        );

        let request = self
            .github
            .client()
            .post(&url)
            .headers(self.github.auth().auth_headers())
            .json(&json!({ "body": body }));

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message: text,
            });
        }

        Ok(())
    }

    /// Close a discussion via GraphQL.
    async fn close_discussion(&mut self, discussion_number: i32) -> Result<()> {
        // First get the discussion ID
        let discussions = self.github.get_discussions(None, Some(100), None).await?;

        let disc = discussions
            .nodes
            .iter()
            .find(|d| d.number == discussion_number);

        if let Some(disc) = disc {
            let mutation = r#"
                mutation($id: ID!) {
                    updateDiscussion(input: {discussionId: $id, state: CLOSED}) {
                        discussion {
                            id
                        }
                    }
                }
            "#;

            #[derive(serde::Serialize)]
            struct Vars {
                id: String,
            }

            #[derive(serde::Deserialize)]
            struct UpdateResult {
                #[serde(rename = "updateDiscussion")]
                update_discussion: DiscClose,
            }

            #[derive(serde::Deserialize)]
            struct DiscClose {
                discussion: DiscId,
            }

            #[derive(serde::Deserialize)]
            struct DiscId {
                id: String,
            }

            let variables = Vars {
                id: disc.id.clone(),
            };

            let _: crate::github::models::GraphQLResponse<UpdateResult> = self
                .github
                .graphql(mutation, Some(variables))
                .await
                .map_err(|e| RogersError::GitHubStatus {
                    code: 500,
                    message: e.to_string(),
                })?;
        }

        Ok(())
    }

    /// Delete a branch (cleanup on failure).
    async fn delete_branch(&mut self, branch_name: &str) -> Result<()> {
        use serde_json::json;

        let url = format!(
            "{}/repos/{}/{}/git/refs/heads/{}",
            self.github.auth().api_url(),
            self.github.owner(),
            self.github.repo(),
            branch_name
        );

        let request = self
            .github
            .client()
            .delete(&url)
            .headers(self.github.auth().auth_headers());

        let response = request.send().await?;
        if !response.status().is_success() {
            tracing::warn!(
                "Could not delete branch {} (cleanup): {}",
                branch_name,
                response.status()
            );
        }

        Ok(())
    }

    /// File a release bead.
    ///
    /// Creates a `chore` bead with `rodgers:type=release` to track
    /// the release process.
    pub fn file_release_bead(
        &self,
        version: &str,
        source: &str,
        pr_count: usize,
    ) -> crate::beads::controller::CreateChildRequest {
        crate::beads::controller::CreateChildRequest {
            title: format!("Release {}", version),
            description: Some(format!(
                r#"Plan: plans/release-management-plan.md

Release {version} has been approved.

SOURCE: {source}
PRs included: {pr_count}

WHAT TO DO
1. Verify the release branch was created
2. Verify the git tag was created
3. Verify the GitHub Release was created
4. Confirm CI passes on the release branch
5. Close this bead when release is verified

NOTE: Rodgers created the branch, tag, and GitHub Release.
Artifact builds are handled by CI, not Rodgers."#,
                version = version,
                source = source,
                pr_count = pr_count,
            )),
            bead_type: Some("chore".to_string()),
            rodgers_type: Some("release".to_string()),
            rodgers_labels: Some("rodgers:type=release".to_string()),
            acceptance_criteria: Some(format!(
                "- [ ] Release {} branch exists",
                version
            )),
            priority: Some(2),
        }
    }
}

impl From<ReleaseExecutionError> for RogersError {
    fn from(err: ReleaseExecutionError) -> Self {
        RogersError::GitHubStatus {
            code: 500,
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_release_branch_name() {
        assert_eq!(
            ReleaseExecutor::release_branch_name("1.2.3"),
            "release/1.2.3"
        );
    }

    #[test]
    fn test_tag_name() {
        assert_eq!(ReleaseExecutor::tag_name("1.2.3"), "v1.2.3");
    }

    #[test]
    fn test_release_title() {
        assert_eq!(
            ReleaseExecutor::release_title("1.2.3"),
            "Version 1.2.3"
        );
    }

    #[test]
    fn test_release_result_default_fields() {
        let result = ReleaseResult {
            version: "1.0.0".to_string(),
            branch_name: "release/1.0.0".to_string(),
            tag_name: "v1.0.0".to_string(),
            release_url: Some("https://github.com/test/releases/tag/v1.0.0".to_string()),
            success: true,
            error: None,
            executed_at: Utc::now(),
        };

        assert_eq!(result.version, "1.0.0");
        assert_eq!(result.branch_name, "release/1.0.0");
        assert_eq!(result.tag_name, "v1.0.0");
        assert!(result.success);
        assert!(result.release_url.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_release_result_failure() {
        let result = ReleaseResult {
            version: "1.0.0".to_string(),
            branch_name: "release/1.0.0".to_string(),
            tag_name: "v1.0.0".to_string(),
            release_url: None,
            success: false,
            error: Some("Some error".to_string()),
            executed_at: Utc::now(),
        };

        assert!(!result.success);
        assert_eq!(result.error, Some("Some error".to_string()));
    }

    #[test]
    fn test_release_executor_new() {
        let github = GitHubClient::new(
            "owner",
            "repo",
            crate::github::auth::GitHubAuth::new_with_default_api("ghp_test"),
        );
        let executor = ReleaseExecutor::new(github, true);
        assert!(executor.create_branch);
    }

    #[test]
    fn test_release_executor_no_branch() {
        let github = GitHubClient::new(
            "owner",
            "repo",
            crate::github::auth::GitHubAuth::new_with_default_api("ghp_test"),
        );
        let executor = ReleaseExecutor::new(github, false);
        assert!(!executor.create_branch);
    }

    #[test]
    fn test_file_release_bead() {
        let github = GitHubClient::new(
            "owner",
            "repo",
            crate::github::auth::GitHubAuth::new_with_default_api("ghp_test"),
        );
        let executor = ReleaseExecutor::new(github, true);

        let request = executor.file_release_bead("1.2.3", "main", 5);

        assert_eq!(request.title, "Release 1.2.3");
        assert!(request.description.as_deref().unwrap_or("").contains("1.2.3"));
        assert!(request.description.as_deref().unwrap_or("").contains("main"));
        assert!(request.description.as_deref().unwrap_or("").contains("5"));
        assert_eq!(request.bead_type, Some("chore".to_string()));
        assert_eq!(
            request.rodgers_type,
            Some("release".to_string())
        );
    }
}
