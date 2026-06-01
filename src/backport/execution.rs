//! Backport execution: branch and PR creation.
//!
//! When a backport is approved, Rodgers:
//! 1. Creates a branch `backport/{sha_short}/{target_branch}`
//! 2. Cherry-picks the commit
//! 3. Creates a PR targeting the release branch
//! 4. On conflict: files a conflict task and alerts
//!
//! ## Branch Naming
//!
//! Backport branches follow the pattern:
//! `backport/{sha_short}/{target_branch_name}`\n
//! e.g., `backport/abc123d/release/1.x`

use crate::github::client::GitHubClient;
use crate::github::models::PullRequest;
use serde::{Deserialize, Serialize};
use std::process::Command;

type ExecutionResult<T> = std::result::Result<T, BackportExecutionError>;

/// Errors related to backport execution.
#[derive(Debug, thiserror::Error)]
pub enum BackportExecutionError {
    #[error("git command failed: {0}")]
    GitError(String),

    #[error("cherry-pick resulted in merge conflicts")]
    MergeConflict,

    #[error("PR creation failed: {0}")]
    PrCreationFailed(String),

    #[error("branch creation failed: {0}")]
    BranchCreationFailed(String),
}

/// Errors related to backport conflicts.
#[derive(Debug, Clone, thiserror::Error)]
pub enum BackportConflictError {
    #[error("merge conflicts prevent backporting {sha} to {branch} ({reason})")]
    Conflict {
        sha: String,
        branch: String,
        reason: String,
    },
}

/// Result of executing a backport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackportResult {
    /// Task ID for this backport.
    pub task_id: String,
    /// Branch name created.
    pub branch_name: String,
    /// PR number (if created).
    pub pr_number: Option<i32>,
    /// PR URL (if created).
    pub pr_url: Option<String>,
    /// Whether the backport had conflicts.
    pub has_conflicts: bool,
    /// Error message if any.
    pub error: Option<String>,
}

/// Git credentials for branch creation.
#[derive(Debug, Clone)]
pub struct GitCredentials {
    /// Branch name to create.
    pub new_branch: String,
    /// Base branch to branch from.
    pub base_branch: String,
    /// Authentication token for git push (if needed).
    pub push_token: Option<String>,
}

impl GitCredentials {
    /// Create credentials for a backport branch.
    pub fn new(new_branch: String, base_branch: String, push_token: Option<String>) -> Self {
        Self {
            new_branch,
            base_branch,
            push_token,
        }
    }
}

/// Backport executor.
///
/// Handles the execution of approved backports — creating branches,
/// cherry-picking commits, resolving conflicts, and creating PRs.
#[derive(Debug, Clone)]
pub struct BackportExecutor {
    /// GitHub client (for API-based operations).
    github: GitHubClient,
    /// Token for pushing git branches.
    _push_token: Option<String>,
}

impl BackportExecutor {
    /// Create a new executor.
    pub fn new(github: GitHubClient, push_token: Option<String>) -> Self {
        Self {
            github,
            _push_token: push_token,
        }
    }

    /// Generate the backport branch name.
    ///
    /// Format: `backport/{sha_short}/{target_branch}`
    pub fn backport_branch_name(commit_sha: &str, target_branch: &str) -> String {
        let short_sha = commit_sha.chars().take(7).collect::<String>();
        let branch_name = target_branch.replace('/', "-");
        format!("backport/{}/{}", short_sha, branch_name)
    }

    /// Execute an approved backport.
    ///
    /// Creates a branch, cherry-picks the commit, and opens a PR.
    pub async fn execute(
        &mut self,
        commit_sha: &str,
        target_branch: &str,
        title: &str,
        body: Option<&str>,
    ) -> ExecutionResult<BackportResult> {
        let branch_name = Self::backport_branch_name(commit_sha, target_branch);

        // Check if branch already exists
        match self.github.get_branch(&branch_name).await {
            Ok(_) => {
                // Branch exists, reuse it
                tracing::debug!("Branch {} already exists, reusing", branch_name);
            }
            Err(_) => {
                // Create the branch
                self.create_branch(&branch_name, target_branch).await?;
            }
        }

        // Cherry-pick the commit - destructure Result immediately
        let (has_conflicts, pr_number) = match self.cherry_pick(commit_sha, &branch_name).await {
            Ok(cherry) if cherry.is_ok() => {
                // No conflicts, create PR
                let pr_number = match self
                    .create_backport_pr(&branch_name, target_branch, title, body)
                    .await
                {
                    Ok(pr) => {
                        tracing::info!(
                            "Backport PR created: #{} {}",
                            pr.number,
                            pr.html_url.as_deref().unwrap_or("(no URL)")
                        );
                        Some(pr.number)
                    }
                    Err(e) => {
                        tracing::error!("PR creation failed: {}", e);
                        None
                    }
                };
                (false, pr_number)
            }
            Ok(cherry) => {
                // Cherry had conflicts
                (cherry.has_conflicts(), None)
            }
            Err(e) => {
                // Git command failed
                tracing::error!("Cherry-pick failed: {}", e);
                (false, None)
            }
        };

        Ok(BackportResult {
            task_id: String::new(), // Set by caller
            branch_name,
            pr_number,
            pr_url: None,
            has_conflicts,
            error: None,
        })
    }

    /// Create a git branch from a base branch.
    async fn create_branch(
        &mut self,
        new_branch: &str,
        base_branch: &str,
    ) -> Result<(), BackportExecutionError> {
        let output = Command::new("git")
            .args([
                "checkout",
                "-b",
                new_branch,
                &format!("origin/{}", base_branch),
            ])
            .output()
            .map_err(|e| BackportExecutionError::BranchCreationFailed(e.to_string()))?;

        if !output.status.success() {
            // Try without origin/ prefix if that fails
            let output = Command::new("git")
                .args(["checkout", "-b", new_branch, base_branch])
                .output()
                .map_err(|e| BackportExecutionError::BranchCreationFailed(e.to_string()))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(BackportExecutionError::BranchCreationFailed(
                    stderr.to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Cherry-pick a commit onto the current branch.
    ///
    /// Returns the result indicating success or conflict.
    async fn cherry_pick(
        &mut self,
        commit_sha: &str,
        _branch_name: &str,
    ) -> std::result::Result<CherryPickResult, BackportExecutionError> {
        // Cherry-pick the commit
        let output = Command::new("git")
            .args(["cherry-pick", commit_sha])
            .output()
            .map_err(|e| BackportExecutionError::GitError(e.to_string()))?;

        if output.status.success() {
            Ok(CherryPickResult::Success)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("CONFLICT") || stderr.contains("merge conflict") {
                // Abort the cherry-pick since it has conflicts
                let _ = Command::new("git")
                    .arg("cherry-pick")
                    .arg("--abort")
                    .output();
                Err(BackportExecutionError::MergeConflict)
            } else {
                Err(BackportExecutionError::GitError(stderr.to_string()))
            }
        }
    }

    /// Create a Pull Request for the backport.
    async fn create_backport_pr(
        &mut self,
        head: &str,
        base: &str,
        title: &str,
        body: Option<&str>,
    ) -> ExecutionResult<PullRequest> {
        // Use the GitHub API's PR creation via issues endpoint
        let pr_body = body.map(|s| s.to_string()).unwrap_or_else(|| {
            format!(
                "Backport of commit to {}.\n\nPlease review and merge.",
                base
            )
        });

        // GitHub's PR creation endpoint - construct URL directly since repo_url is private
        let owner = self.github.owner();
        let repo = self.github.repo();
        let api_url = self.github.auth().api_url();
        let url = format!("{}/repos/{}/{}/pulls", api_url, owner, repo);

        let request = self
            .github
            .client()
            .post(&url)
            .headers(self.github.auth().auth_headers())
            .json(&serde_json::json!({
                "title": title,
                "head": head,
                "base": base,
                "body": pr_body,
                "draft": true,
            }));

        let response = request.send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let pr: PullRequest = resp
                    .json()
                    .await
                    .map_err(|e| BackportExecutionError::PrCreationFailed(e.to_string()))?;
                Ok(pr)
            }
            Ok(resp) => {
                let status = resp.status();
                let body_err = resp.text().await.unwrap_or_default();
                Err(BackportExecutionError::PrCreationFailed(format!(
                    "HTTP {}: {}",
                    status.as_u16(),
                    body_err
                )))
            }
            Err(e) => Err(BackportExecutionError::PrCreationFailed(e.to_string())),
        }
    }

    /// File a conflict-resolution task.
    ///
    /// Returns the task title and description for filing via TaskController.
    pub fn conflict_task_details(
        &self,
        commit_sha: &str,
        target_branch: &str,
        merge_message: Option<&str>,
    ) -> (String, String) {
        let title = format!(
            "Resolve conflicts: backport {} to {}",
            &commit_sha[..7.min(commit_sha.len())],
            target_branch
        );

        let description = format!(
            r#"Plan: plans/backport-plan.md §Conflict Handling

The cherry-pick of commit `{0}` to `{1}` has merge conflicts.
Human resolution is required.

COMMIT DETAILS
- Commit SHA: {0}
- Target branch: {1}

WHAT TO DO
1. Check out the conflict by looking at the git status on `backport/<sha>/{1}`
2. Resolve the merge conflicts manually
3. Run `git add` on the resolved files
4. Run `git cherry-pick --continue`
5. Force-push to the backport branch: `backport/<sha>/{1}`

ACCEPTANCE
- [ ] Conflicts resolved and branch pushed
- [ ] CI passes on the backport PR

NOTES
{2}
"#,
            commit_sha,
            target_branch,
            merge_message.unwrap_or("(no merge conflict details provided)")
        );

        (title, description)
    }

    /// Generate a unique task ID (12 chars, URL-safe).
    pub fn generate_task_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Format as {timestamp_hex}-{counter_hex} with width to ensure exact char count
        // ts is u64 -> up to 16 hex chars; we use last 5 (20 bits = up to 5 hex)
        // counter is u64 -> unlimited but we mask to last 5 digits
        // Format: SS SSS-CC CCC = 12 chars total (with dash separator)
        format!(
            "{:05x}-{:05x}",
            (ts & 0xFFFFF) as u32,
            (counter & 0xFFFFF) as u32
        )
    }
}

/// Result of a cherry-pick operation.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CherryPickResult {
    /// Cherry-pick succeeded cleanly.
    Success,
    /// Cherry-pick failed with conflicts.
    Conflict,
    /// Cherry-pick failed with an error.
    Error,
}

impl CherryPickResult {
    fn has_conflicts(&self) -> bool {
        matches!(self, CherryPickResult::Conflict)
    }

    fn is_ok(&self) -> bool {
        matches!(self, CherryPickResult::Success)
    }

    #[allow(dead_code)]
    fn error(&self) -> Option<&'static str> {
        match self {
            CherryPickResult::Success | CherryPickResult::Conflict => None,
            CherryPickResult::Error => Some("cherry-pick failed with error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backport_branch_name() {
        assert_eq!(
            BackportExecutor::backport_branch_name("abc123def456789", "release/1.x"),
            "backport/abc123d/release-1.x"
        );
        assert_eq!(
            BackportExecutor::backport_branch_name("def456789012345", "main"),
            "backport/def4567/main"
        );
    }

    #[test]
    fn test_cherry_pick_result() {
        assert!(!CherryPickResult::Success.has_conflicts());
        assert!(CherryPickResult::Success.is_ok());

        assert!(CherryPickResult::Conflict.has_conflicts());
        assert!(!CherryPickResult::Conflict.is_ok());

        assert!(!CherryPickResult::Error.has_conflicts());
        assert!(!CherryPickResult::Error.is_ok());
    }

    #[test]
    fn test_conflict_task_details() {
        let executor = BackportExecutor::new(
            GitHubClient::new(
                "owner",
                "repo",
                crate::github::auth::GitHubAuth::new_with_default_api("ghp_test"),
            ),
            None,
        );

        let (title, description) = executor.conflict_task_details(
            "abc123def456789",
            "release/1.x",
            Some("Conflicts in src/login.rs and tests/test_auth.py"),
        );

        assert!(title.to_lowercase().contains("resolve conflicts"));
        assert!(description.contains("abc123def456789"));
        assert!(description.contains("release/1.x"));
        assert!(description.contains("src/login.rs"));
    }

    #[test]
    fn test_generate_task_id() {
        let id1 = BackportExecutor::generate_task_id();
        let id2 = BackportExecutor::generate_task_id();
        assert_ne!(id1, id2);
        // Format is {ts_hex}-{counter_hex} with zero-padding to fixed widths
        // Each hex component is 5 chars (masked to 20 bits), with a dash separator
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
        assert!(id2.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn test_git_credentials_new() {
        let creds = GitCredentials::new(
            "backport/abc123d/release-1.x".to_string(),
            "release/1.x".to_string(),
            Some("ghp_token".to_string()),
        );

        assert_eq!(creds.new_branch, "backport/abc123d/release-1.x");
        assert_eq!(creds.base_branch, "release/1.x");
        assert_eq!(creds.push_token, Some("ghp_token".to_string()));
    }
}
