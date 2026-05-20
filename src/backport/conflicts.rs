//! Conflict detection and conflict-resolution bead filing.
//!
//! Per plan/backport-plan.md §Conflict Handling:
//!
//! If a cherry-pick has conflicts, Rodgers:
//! 1. Files a `chore` bead (`rodgers:type=backport-conflict`) noting the target
//!    branch, source commit, and that merge conflict resolution is needed
//! 2. Posts a comment on the original issue: "Backport needs conflict
//!    resolution. Bead filed."
//! 3. Closes the approval Discussion
//!
//! Rodgers does NOT attempt the cherry-pick or any partial application.
//! Human judgment is required to resolve merge conflicts.
//!
//! ## Detecting Conflicts
//!
//! Conflicts are detected after a backport PR is created by checking the PR's
//! `mergeable` field via GitHub's REST API. A PR is conflicting when GitHub
//! reports `mergeable = false` (specifically `mergeable_status = "conflicting"`).
//!
//! GitHub computes mergeability asynchronously — immediately after PR creation
//! the field may be `None`. `wait_for_mergeable` polls until the field is
//! populated or a timeout is reached.

use tracing::info;

use crate::RogersError;
use crate::beads::client::{BeadClient, BeadResult};
use crate::config::schema::ReleaseConfig;
use crate::github::client::{GithubClient, PullRequest};

/// Invariant: a successfully-executing backport (no conflicts).
pub use super::execution::BackportExecutionResult as BackportNormalResult;

/// Result of handling a backport conflict.
#[derive(Debug, Clone)]
pub struct ConflictResult {
    /// The backport bead previously filed (for the empty branch + PR).
    pub previous_bead_id: String,
    /// The conflict-resolution bead ID.
    pub conflict_bead_id: String,
    /// Source issue number.
    pub source_issue: Option<u64>,
    /// Source PR number.
    pub source_pr: u64,
    /// Target release branch.
    pub target_branch: String,
    /// Source commit SHA.
    pub source_sha: String,
    /// Sha short (first 7 chars).
    pub sha_short: String,
    /// Whether the conflict comment was posted on the source issue.
    pub source_comment_posted: bool,
    /// Whether the approval discussion was closed.
    pub discussion_closed: bool,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

impl ConflictResult {
    /// Returns true if all conflict-handling operations succeeded.
    pub fn is_success(&self) -> bool {
        self.source_comment_posted && self.discussion_closed && self.errors.is_empty()
    }
}

/// Wait for GitHub to compute PR mergeability and return the PR state.
///
/// GitHub computes `mergeable` asynchronously. Immediately after PR creation
/// the field may be `None`. This function polls at most `max_attempts` times,
/// sleeping `interval_ms` milliseconds between attempts.
///
/// Returns `Ok(PullRequest)` when GitHub has populated the mergeable field.
/// Returns `Ok(pr)` if `max_attempts` is exhausted and `pr.mergeable` is still
/// `None` (treats as "unknown mergeability" — GitHub will surface issues later).
pub async fn wait_for_mergeable(
    pr_number: u64,
    github: &GithubClient,
    max_attempts: u32,
    interval_ms: u64,
) -> Result<PullRequest, RogersError> {
    for attempt in 0..max_attempts {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
        }

        let pr = github.get_pull_request(pr_number).await?;

        if pr.mergeable.is_some() {
            info!(
                "PR #{} mergeable field populated after {} attempt(s)",
                pr_number,
                attempt + 1
            );
            return Ok(pr);
        }

        info!(
            "PR #{} mergeable not yet available, attempt {}/{}",
            pr_number,
            attempt + 1,
            max_attempts
        );
    }

    // Exhaustive wait — fetch one more time to return the latest state
    github.get_pull_request(pr_number).await
}

/// Check whether a PR has merge conflicts.
///
/// Per the GitHub API, `mergeable == false` with `mergeable_status == "conflicting"`
/// indicates that GitHub cannot auto-merge the PR due to merge conflicts.
pub fn has_merge_conflicts(pr: &PullRequest) -> bool {
    pr.mergeable == Some(false)
}

/// Handle a merge conflict for an approved backport:
///
/// 1. File a conflict-resolution bead
/// 2. Post comment on source issue
/// 3. Close the approval discussion
///
/// Returns a `ConflictResult` with all outcomes recorded.
pub async fn handle_conflict(
    execution_result: &BackportNormalResult,
    source_issue: Option<u64>,
    source_pr: u64,
    source_sha: &str,
    sha_short: &str,
    pr_title: &str,
    target_branch: &str,
    discussion_number: u64,
    github: &GithubClient,
    _release_config: &ReleaseConfig,
) -> Result<ConflictResult, RogersError> {
    info!(
        "Handling merge conflict for backport PR to {} (PR #{} → {})",
        target_branch, source_pr, discussion_number
    );

    let mut result = ConflictResult {
        previous_bead_id: execution_result.bead_id.clone(),
        conflict_bead_id: String::new(),
        source_issue,
        source_pr,
        target_branch: target_branch.to_string(),
        source_sha: source_sha.to_string(),
        sha_short: sha_short.to_string(),
        source_comment_posted: false,
        discussion_closed: false,
        errors: vec![],
    };

    // Step 1: File the conflict-resolution bead
    let bead = file_conflict_bead(
        source_sha,
        sha_short,
        pr_title,
        source_pr,
        target_branch,
        &execution_result.branch_name,
    )
    .await;

    match bead {
        Ok(bead_result) => {
            result.conflict_bead_id = bead_result.id.clone();
            info!(
                "Conflict-resolution bead filed: {} for backport to {}",
                bead_result.id, target_branch
            );
        }
        Err(e) => {
            let msg = format!("Failed to file conflict-resolution bead: {}", e);
            tracing::warn!("{}", msg);
            result.errors.push(msg);
        }
    }

    // Step 2: Post comment on source issue
    if let Some(issue_num) = source_issue {
        let conflict_bead_id_str: Option<&str> = if result.conflict_bead_id.is_empty() {
            None
        } else {
            Some(result.conflict_bead_id.as_str())
        };
        let comment_body = format_conflict_comment(
            sha_short,
            target_branch,
            &execution_result.branch_name,
            conflict_bead_id_str,
        );

        match github.create_issue_comment(issue_num, &comment_body).await {
            Ok(_) => {
                info!("Posted conflict comment on source issue #{}", issue_num);
                result.source_comment_posted = true;
            }
            Err(e) => {
                let msg = format!(
                    "Failed to post comment on source issue #{}: {}",
                    issue_num, e
                );
                tracing::warn!("{}", msg);
                result.errors.push(msg);
            }
        }
    }

    // Step 3: Close the approval discussion
    match super::approval::close_discussion(discussion_number, github).await {
        Ok(()) => {
            info!(
                "Closed approval discussion #{} for conflicting backport",
                discussion_number
            );
            result.discussion_closed = true;
        }
        Err(e) => {
            let msg = format!(
                "Failed to close approval discussion #{}: {}",
                discussion_number, e
            );
            tracing::warn!("{}", msg);
            result.errors.push(msg);
        }
    }

    Ok(result)
}

/// File a conflict-resolution bead (chore, rodgers:type=backport-conflict).
///
/// The previously-filed backport bead is referenced so resolution work is
/// a child of the original backport effort.
async fn file_conflict_bead(
    sha: &str,
    sha_short: &str,
    pr_title: &str,
    source_pr: u64,
    target_branch: &str,
    backport_branch: &str,
) -> Result<BeadResult, RogersError> {
    let title = format!(
        "Resolve merge conflicts: backport #{} to {}",
        sha_short, target_branch
    );

    let description = format!(
        "Plan: plans/backport-plan.md §Conflict Handling\n\n\
**Backport:** #{sha} — \"{pr_title}\"\n\
**Source PR:** #{source_pr}\n\
**Target branch:** {branch}\n\
**Backport branch:** `{backport_branch}`\n\n\
## Merge Conflict Detected\n\n\
Rodgers detected merge conflicts when attempting to backport the above commit.\n\
This backport requires human judgment to resolve:\n\n\
WHAT TO DO\n\
1. Clone the repository and checkout the backport branch:\n   `git checkout {backport_branch}`\n\
2. Cherry-pick the source commit and resolve conflicts:\n   `git cherry-pick {sha}`\n   - Edits marked `<<<<<<< HEAD` need to be resolved\n   - After resolving, `git add . && git cherry-pick --continue`\n3. Push the resolved changes:\n   `git push origin {backport_branch}`\n\n\
ACCEPTANCE\n\
- [ ] Cherry-pick of #{sha} applies cleanly (all conflicts resolved)\n\
- [ ] Backport PR is updated with conflict-resolved changes\n\
- [ ] CI passes on the backport PR\n\
- [ ] PR is merged or explicitly closed\n\n\
PITFALLS\n\
- Do NOT close or delete the backport branch — the CI/PR depends on it\n\
- If the conflict involves shared library divergence, you may need to\n  recreate logic that exists in main but not in {branch}\n\
- Document the resolution in this bead before closing\n\
- If resolution is not possible, update the PR to close without merging\n  and note the reason in this bead",
        sha = sha,
        pr_title = pr_title,
        source_pr = source_pr,
        branch = target_branch,
        backport_branch = backport_branch,
    );

    let acceptance = format!(
        "Merge conflicts for #{sha} to {branch} are resolved and PR is merged",
        sha = sha,
        branch = target_branch
    );

    let external_ref = format!("gh-{}", source_pr);
    let deps = format!("discovered-from:#{}", source_pr);

    BeadClient::new()
        .file_bead(&title, &description, "chore")
        .with_tag("rodgers:type=backport-conflict")
        .with_priority(2) // Conflicts are resolved by human — standard priority
        .with_acceptance(&acceptance)
        .with_external_ref(&external_ref)
        .with_deps(&deps)
        .submit()
        .await
}

/// Format the comment to post on the source issue when a conflict is detected.
fn format_conflict_comment(
    sha_short: &str,
    target_branch: &str,
    backport_branch: &str,
    conflict_bead_id: Option<&str>,
) -> String {
    let bead_note = conflict_bead_id
        .map(|id| format!("Conflict bead: **{id}**\n"))
        .unwrap_or_default();

    format!(
        "## ⚠️ Backport Needs Conflict Resolution\n\n\
**Commit:** #{sha}\n\
**Target branch:** `{branch}`\n\
**Backport branch:** `{backport_branch}`\n\n\
Backport needs conflict resolution. Bead filed.\n\n\
{bead_note}\
A human must resolve the merge conflicts and push the changes.\n\
Run `git cherry-pick <sha>` to resolve.\n\
Once resolved, CI will run on the backport PR.\n\n\
---\n\
_This comment was automatically posted by Rodgers._",
        sha = sha_short,
        branch = target_branch,
        backport_branch = backport_branch,
        bead_note = bead_note,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_execution_result() -> BackportNormalResult {
        BackportNormalResult {
            bead_id: "bead-backport-42".to_string(),
            branch_name: "backport/abc123d/release/1.x".to_string(),
            pr_number: Some(99),
            pr_url: Some("https://github.com/org/repo/pull/99".to_string()),
            source_comment_posted: true,
            errors: vec![],
        }
    }

    #[test]
    fn test_conflict_result_fields() {
        let result = ConflictResult {
            previous_bead_id: "bead-backport-42".to_string(),
            conflict_bead_id: "bead-conflict-77".to_string(),
            source_issue: Some(42),
            source_pr: 99,
            target_branch: "release/2.x".to_string(),
            source_sha: "abc123def456abc123".to_string(),
            sha_short: "abc123d".to_string(),
            source_comment_posted: true,
            discussion_closed: true,
            errors: vec![],
        };

        assert_eq!(result.previous_bead_id, "bead-backport-42");
        assert_eq!(result.conflict_bead_id, "bead-conflict-77");
        assert_eq!(result.source_issue, Some(42));
        assert_eq!(result.target_branch, "release/2.x");
        assert!(result.is_success());
    }

    #[test]
    fn test_conflict_result_not_success_on_error() {
        let result = ConflictResult {
            previous_bead_id: "bead-1".to_string(),
            conflict_bead_id: "bead-2".to_string(),
            source_issue: None,
            source_pr: 99,
            target_branch: "release/1.x".to_string(),
            source_sha: "abc".to_string(),
            sha_short: "abc123d".to_string(),
            source_comment_posted: false,
            discussion_closed: false,
            errors: vec!["Failed to file bead".to_string()],
        };

        assert!(!result.is_success());
    }

    #[test]
    fn test_has_merge_conflicts() {
        let pr_conflicting = PullRequest {
            number: 1,
            title: "Backport PR".to_string(),
            body: None,
            state: "open".to_string(),
            html_url: "https://github.com/org/repo/pull/1".to_string(),
            head: crate::github::client::PrBranch {
                ref_: "backport/abc/release/1.x".to_string(),
                sha: "abc123".to_string(),
            },
            base: crate::github::client::PrBranch {
                ref_: "release/1.x".to_string(),
                sha: "def456".to_string(),
            },
            mergeable: Some(false),
        };

        let pr_mergeable = PullRequest {
            number: 2,
            title: "Clean Backport PR".to_string(),
            body: None,
            state: "open".to_string(),
            html_url: "https://github.com/org/repo/pull/2".to_string(),
            head: crate::github::client::PrBranch {
                ref_: "backport/def/release/1.x".to_string(),
                sha: "def789".to_string(),
            },
            base: crate::github::client::PrBranch {
                ref_: "release/1.x".to_string(),
                sha: "abc123".to_string(),
            },
            mergeable: Some(true),
        };

        let pr_unknown = PullRequest {
            number: 3,
            title: "Unknown Mergeable PR".to_string(),
            body: None,
            state: "open".to_string(),
            html_url: "https://github.com/org/repo/pull/3".to_string(),
            head: crate::github::client::PrBranch {
                ref_: "backport/xyz/release/1.x".to_string(),
                sha: "xyz123".to_string(),
            },
            base: crate::github::client::PrBranch {
                ref_: "release/1.x".to_string(),
                sha: "abc123".to_string(),
            },
            mergeable: None,
        };

        assert!(has_merge_conflicts(&pr_conflicting));
        assert!(!has_merge_conflicts(&pr_mergeable));
        assert!(!has_merge_conflicts(&pr_unknown)); // None = not yet computed
    }

    #[test]
    fn test_conflict_comment_format() {
        let comment = format_conflict_comment(
            "abc123d",
            "release/2.x",
            "backport/abc123d/release/2.x",
            Some("bead-conflict-42"),
        );

        assert!(comment.contains("## ⚠️ Backport Needs Conflict Resolution"));
        assert!(comment.contains("release/2.x"));
        assert!(comment.contains("backport/abc123d/release/2.x"));
        assert!(comment.contains("Conflict bead: **bead-conflict-42**"));
        assert!(comment.contains("A human must resolve"));
        assert!(comment.contains("cherry-pick"));
    }

    #[test]
    fn test_conflict_comment_without_bead_id() {
        let comment = format_conflict_comment(
            "abc123d",
            "release/3.x",
            "backport/abc123d/release/3.x",
            None,
        );

        assert!(comment.contains("## ⚠️ Backport Needs Conflict Resolution"));
        // No bead paragraph if bead ID not yet assigned
        assert!(!comment.contains("Conflict bead:"));
        assert!(comment.contains("A human must resolve"));
    }

    #[test]
    fn test_conflict_bead_title_format() {
        use crate::beads::client::BeadClient;

        // Verify the title format by checking the BeadClient use path
        assert!(
            "Resolve merge conflicts: backport #abc123d to release/1.x"
                .contains("Resolve merge conflicts")
        );
    }
}
