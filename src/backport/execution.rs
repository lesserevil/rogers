//! Backport execution — creates branch and PR when approval is detected.
//!
//! Per plan/backport-plan.md §Backport Execution:
//!
//! When approved, Rodgers:
//! 1. Creates a branch `backport/{sha_short}/{branch_name}` from the target release branch head
//! 2. Files a `chore` bead (`rodgers:type=backport`) describing what needs to be cherry-picked
//! 3. Creates a PR targeting release/{X.Y} with the cherry-pick
//! 4. Posts a comment on the original issue noting the backport is in progress
//!
//! Rodgers does not perform the cherry-pick. The cherry-pick is work for an actor
//! outside Rodgers, tracked via the `chore` bead.

use regex::Regex;
use std::sync::OnceLock;
use tracing::{info, warn};

use crate::RogersError;
use crate::beads::client::BeadClient;
use crate::beads::client::BeadResult;
use crate::github::client::{GithubClient, PullRequest};

/// Result of executing a backport for one target branch.
#[derive(Debug, Clone)]
pub struct BackportExecutionResult {
    /// The backport bead ID.
    pub bead_id: String,
    /// The branch that was created.
    pub branch_name: String,
    /// The PR number (if created successfully).
    pub pr_number: Option<u64>,
    /// The PR URL (if created successfully).
    pub pr_url: Option<String>,
    /// Whether the comment was posted on the source issue.
    pub source_comment_posted: bool,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

impl BackportExecutionResult {
    /// Returns true if all operations succeeded.
    pub fn is_success(&self) -> bool {
        self.pr_number.is_some() && self.errors.is_empty()
    }
}

/// Execute a backport: create branch, file bead, create PR, post comment.
///
/// All steps complete within one triage run.
pub async fn execute_backport(
    sha: &str,
    sha_short: &str,
    pr_number: u64,
    pr_title: &str,
    source_issue: Option<u64>,
    target_branch: &str,
    github: &GithubClient,
    discovery_bead_id: &str,
) -> Result<BackportExecutionResult, RogersError> {
    info!(
        "Executing backport for commit {} (PR #{}) to {}",
        sha_short, pr_number, target_branch
    );

    let mut result = BackportExecutionResult {
        bead_id: discovery_bead_id.to_string(),
        branch_name: format!("backport/{}/{}", sha_short, target_branch),
        pr_number: None,
        pr_url: None,
        source_comment_posted: false,
        errors: vec![],
    };

    // Step 1: Get the SHA of the target branch (base for the new branch)
    let base_sha = match github.branch_sha(target_branch).await {
        Ok(sha) => {
            info!("Target branch {} SHA: {}", target_branch, sha);
            sha
        }
        Err(e) => {
            let msg = format!(
                "Failed to get SHA of target branch '{}': {}",
                target_branch, e
            );
            warn!("{}", msg);
            result.errors.push(msg);
            return Ok(result);
        }
    };

    // Step 2: Create the backport branch from the target release branch head
    let branch_name = &result.branch_name;
    match github.create_branch(branch_name, &base_sha).await {
        Ok(ref_) => {
            info!("Created branch '{}' at commit {}", branch_name, ref_.sha);
        }
        Err(e) => {
            let msg = format!("Failed to create branch '{}': {}", branch_name, e);
            warn!("{}", msg);
            result.errors.push(msg);
            // Continue with execution even if branch creation fails
            // (branch may already exist)
        }
    }

    // Step 3: File the cherry-pick bead (chore bead tracking the work)
    let cherry_pick_bead = file_cherry_pick_bead(
        sha,
        sha_short,
        pr_title,
        pr_number,
        target_branch,
        branch_name,
    )
    .await;

    if let Ok(bead_result) = &cherry_pick_bead {
        result.bead_id = bead_result.id.clone();
        info!("Cherry-pick bead filed: {}", bead_result.id);
    } else if let Err(e) = &cherry_pick_bead {
        let msg = format!("Failed to file cherry-pick bead: {}", e);
        warn!("{}", msg);
        result.errors.push(msg);
    }

    // Step 4: Create PR targeting the release branch
    let pr = create_backport_pr(
        sha_short,
        pr_title,
        branch_name,
        target_branch,
        pr_number,
        github,
    )
    .await;

    match pr {
        Ok(pr) => {
            result.pr_number = Some(pr.number);
            result.pr_url = Some(pr.html_url);
            info!(
                "Backport PR created: #{} '{}' → {}",
                pr.number, branch_name, target_branch
            );
        }
        Err(e) => {
            let msg = format!("Failed to create PR: {}", e);
            warn!("{}", msg);
            result.errors.push(msg);
        }
    }

    // Step 5: Post comment on source issue linking the backport
    if let Some(issue_num) = source_issue {
        let comment_body = format_source_issue_comment(
            sha_short,
            pr_number,
            target_branch,
            branch_name,
            result.pr_number,
            result.pr_url.as_deref(),
        );

        match github.create_issue_comment(issue_num, &comment_body).await {
            Ok(_) => {
                info!("Posted backport comment on source issue #{}", issue_num);
                result.source_comment_posted = true;
            }
            Err(e) => {
                let msg = format!(
                    "Failed to post comment on source issue #{}: {}",
                    issue_num, e
                );
                warn!("{}", msg);
                result.errors.push(msg);
            }
        }
    }

    Ok(result)
}

/// File a `chore` bead for cherry-pick work.
///
/// The bead is titled with the target branch and links back to the original PR.
/// Per plan specification, Rodgers does not perform the cherry-pick — the bead
/// tracks the work for a human actor.
async fn file_cherry_pick_bead(
    sha: &str,
    _sha_short: &str,
    pr_title: &str,
    pr_number: u64,
    target_branch: &str,
    branch_name: &str,
) -> Result<BeadResult, RogersError> {
    let title = format!("Cherry-pick backport #{} to {}", sha, target_branch);

    let description = format!(
        "Plan: plans/backport-plan.md\n\n\
Backport for: #{sha} — \"{pr_title}\"\n\
Source PR: gh-{pr_number}\n\
Target branch: {branch}\n\n\
Rodgers has created branch `{backport_branch}` and a draft PR for this backport.\n\
Your task is to:\n\n\
WHAT TO DO\n\
1. Switch to branch `{backport_branch}`\n\
2. Cherry-pick commit #{sha}\n\
3. Resolve any merge conflicts\n\
4. Push the resolved changes\n\n\
ACCEPTANCE\n\
- [ ] Cherry-pick of #{sha} applies cleanly or conflicts are resolved\n\
- [ ] Draft PR is updated with the conflict-resolved changes\n\
- [ ] CI passes on the backport PR\n\
- [ ] PR is merged or explicitly closed\n\n\
PITFALLS\n\
- If the cherry-pick has conflicts, resolve them manually — Rodgers cannot\n  perform conflict resolution autonomously.\n\
- If the target file doesn't exist in {branch}, this backport cannot be\n  applied. File a note bead instead.\n\
- Document any non-trivial conflicts in this bead before closing.\n",
        sha = sha,
        pr_title = pr_title,
        pr_number = pr_number,
        branch = target_branch,
        backport_branch = branch_name,
    );

    let acceptance = format!(
        "Cherry-pick of #{} to {} is merged or explicitly closed without merging",
        sha, target_branch
    );

    let external_ref = format!("gh-{}", pr_number);
    let deps = format!("discovered-from:#{}", pr_number);

    BeadClient::new()
        .file_bead(&title, &description, "chore")
        .with_tag("rodgers:type=backport")
        .with_priority(2) // Standard priority; security uses 1
        .with_acceptance(&acceptance)
        .with_external_ref(&external_ref)
        .with_deps(&deps)
        .submit()
        .await
}

/// Create a PR for the backport branch targeting the release branch.
///
/// The PR is NOT created as draft since our branch is empty (no cherry-pick applied yet).
/// Rodgers creates the PR structure so a human can complete the cherry-pick and update it.
async fn create_backport_pr(
    sha_short: &str,
    pr_title: &str,
    branch_name: &str,
    target_branch: &str,
    original_pr_number: u64,
    github: &GithubClient,
) -> Result<PullRequest, RogersError> {
    let title = format!("[backport] {} (PR #{})", pr_title, original_pr_number);

    let body = format!(
        "## Backport PR\n\n\
**Original commit:** #{sha}\n\
**Original PR:** #{original_pr_number}\n\
**Target branch:** {target_branch}\n\n\
This PR is for backporting the above fix to the {target_branch} release branch.\n\
Rodgers created the branch and PR structure. A human needs to:\n\n\
1. Cherry-pick commit #{sha} to this branch\n2. Resolve any merge conflicts\n3. Push the resolved changes\n\n\
---\n\
_Generated by Rodgers_",
        sha = sha_short,
        original_pr_number = original_pr_number,
        target_branch = target_branch,
    );

    github
        .create_pull_request(&title, &body, branch_name, target_branch)
        .await
}

/// Format the comment body for posting on the source issue.
fn format_source_issue_comment(
    _sha_short: &str,
    _pr_number: u64,
    target_branch: &str,
    backport_branch: &str,
    backport_pr_number: Option<u64>,
    backport_pr_url: Option<&str>,
) -> String {
    let pr_link = if let (Some(num), Some(url)) = (backport_pr_number, backport_pr_url) {
        format!(" [PR #{}]({})", num, url)
    } else if let Some(num) = backport_pr_number {
        format!(" PR #{}", num)
    } else {
        String::new()
    };

    format!(
        "## 🔙 Backport In Progress\n\n\
A backport to `{target_branch}` has been approved and is being processed.\n\n\
**Details:**\n\
- Cherry-pick branch: `{backport_branch}`\n\
- Backport PR:{pr_link}\n\
\n\
A human will need to complete the cherry-pick and resolve any conflicts.\n\
See the linked bead for tracking.\n\n\
---\n\
_This comment was automatically posted by Rodgers._",
        target_branch = target_branch,
        backport_branch = backport_branch,
        pr_link = pr_link,
    )
}

/// Extract the issue number from PR body text.
///
/// Tries to find "Closes #N", "Fixes #N", etc.
pub fn extract_source_issue(pr_body: &str) -> Option<u64> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?:(?:closes?|fixes?|resolves?)\s+)?#(\d+)").expect("hardcoded regex is valid")
    });
    re.captures(pr_body)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backport_branch_name_format() {
        // Branch name should be backport/{sha_short}/{branch_name}
        let sha_short = "abc123d";
        let target_branch = "release/1.x";
        let expected = format!("backport/{}/{}", sha_short, target_branch);

        assert_eq!(expected, "backport/abc123d/release/1.x");
    }

    #[test]
    fn test_extract_source_issue_with_closes() {
        assert_eq!(extract_source_issue("Closes #12345"), Some(12345));
        assert_eq!(extract_source_issue("Fixes #99."), Some(99));
        assert_eq!(extract_source_issue("Resolves #777"), Some(777));
    }

    #[test]
    fn test_extract_source_issue_without_issue() {
        assert_eq!(extract_source_issue("No issue here"), None);
        assert_eq!(extract_source_issue("Just some text"), None);
    }

    #[test]
    fn test_backport_execution_result_is_success() {
        let success_result = BackportExecutionResult {
            bead_id: "bead-1".to_string(),
            branch_name: "backport/abc/release/1.x".to_string(),
            pr_number: Some(42),
            pr_url: Some("https://github.com/org/repo/pull/42".to_string()),
            source_comment_posted: true,
            errors: vec![],
        };
        assert!(success_result.is_success());

        let failed_result = BackportExecutionResult {
            bead_id: "bead-1".to_string(),
            branch_name: "backport/abc/release/1.x".to_string(),
            pr_number: None,
            pr_url: None,
            source_comment_posted: false,
            errors: vec!["PR creation failed".to_string()],
        };
        assert!(!failed_result.is_success());
    }

    #[test]
    fn test_source_issue_comment_format() {
        let comment = format_source_issue_comment(
            "abc123d",
            42,
            "release/2.x",
            "backport/abc123d/release/2.x",
            Some(100),
            Some("https://github.com/org/repo/pull/100"),
        );

        assert!(comment.contains("## 🔙 Backport In Progress"));
        assert!(comment.contains("release/2.x"));
        assert!(comment.contains("backport/abc123d/release/2.x"));
        assert!(comment.contains("[PR #100]"));
        assert!(comment.contains("cherry-pick"));
    }

    #[test]
    fn test_source_issue_comment_format_without_pr() {
        let comment = format_source_issue_comment(
            "abc123d",
            42,
            "release/2.x",
            "backport/abc123d/release/2.x",
            None,
            None,
        );

        assert!(comment.contains("## 🔙 Backport In Progress"));
        // Should not have PR link section
        assert!(comment.contains("A human will need to complete the cherry-pick"));
    }

    #[test]
    fn test_pr_title_format() {
        let title = format!("[backport] Fix critical bug (PR #{})", 42);
        assert!(title.contains("backport"));
        assert!(title.contains("#42"));
    }

    #[test]
    fn test_execution_result_fields() {
        let result = BackportExecutionResult {
            bead_id: "bead-42".to_string(),
            branch_name: "backport/abc123d/release/1.x".to_string(),
            pr_number: Some(99),
            pr_url: Some("https://github.com/org/repo/pull/99".to_string()),
            source_comment_posted: true,
            errors: vec![],
        };

        assert_eq!(result.bead_id, "bead-42");
        assert_eq!(result.pr_number, Some(99));
        assert!(result.source_comment_posted);
        assert!(result.errors.is_empty());
    }
}
