//! Backport manager — processes detected backport candidates.
//!
//! Entry point: `process_candidates`. For each candidate, this module files
//! one backport bead per active release branch, plus creates a GitHub Discussion
//! for each bead to gate human approval.
//!
//! The approval flow is handled via `check_pending_discussions`, which:
//! - Monitors voting on discussions (via GraphQL)
//! - Posts reminders when voting window expires
//! - Closes discussions that exceed stale threshold
//! - Notifies when a backport is approved or rejected
//!
//! Bead shape follows plans/backport-plan.md:
//!   title: "Backport #{sha_short} to {branch_name}"
//!   type:  chore
//!   tag:   rodgers:type=backport
//!   priority: 1 for security, 2 for bug/backport-me
//!   --deps discovered-from:{source_issue}
//!
//! The bead is linked back to the source GitHub issue via `discovered-from`
//! deps argument.

use tracing::info;

use super::approval::{
    ApprovalState, DiscussionVoteResult, check_approval_status, close_discussion,
    post_reminder_comment,
};
use super::bead::BackportBead;
use super::conflicts::{handle_conflict as handle_backport_conflict, has_merge_conflicts, wait_for_mergeable};
use super::detector::{BackportCandidate, BackportReason};
use super::execution::{BackportExecutionResult, execute_backport};
use crate::RogersError;
use crate::beads::BeadClient;
use crate::beads::client::BeadResult;
use crate::config::schema::ReleaseConfig;
use crate::github::client::GithubClient;

/// Result of processing one backport candidate across all active branches.
#[derive(Debug, Clone)]
pub struct BackportResult {
    /// The PR number processed.
    pub pr_number: u64,
    /// PR title at time of detection.
    pub pr_title: String,
    /// Backport reason classification.
    pub reason: BackportReason,
    /// Number of branches a bead was filed for.
    pub branches_filed: usize,
    /// Number of branches where a discussion was created.
    pub discussions_created: usize,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

/// Tracks a pending backport discussion and its approval status.
#[derive(Debug, Clone)]
pub struct PendingBackportDiscussion {
    /// The backport bead ID (for traceability).
    pub bead_id: String,
    /// Source PR number.
    pub pr_number: u64,
    /// Source PR title.
    pub pr_title: String,
    /// Merge commit SHA.
    pub commit_sha: String,
    /// Target release branch.
    pub target_branch: String,
    /// GitHub Discussion number.
    pub discussion_number: u64,
    /// When the discussion was created (ISO 8601).
    pub discussion_created_at: String,
    /// Current approval status.
    pub approval_result: Option<DiscussionVoteResult>,
    /// Execution result (if approved and executed).
    pub execution_result: Option<BackportExecutionResult>,
}

impl PendingBackportDiscussion {
    /// Extract source issue number from PR body.
    pub fn source_issue_from_body(&self, body: &str) -> Option<u64> {
        super::execution::extract_source_issue(body)
    }
}

/// Result of filing a backport and creating its discussion.
#[derive(Debug, Clone)]
pub struct FiledBackport {
    /// The backport bead.
    pub bead_id: String,
    /// Source PR number.
    pub pr_number: u64,
    /// Target release branch.
    pub target_branch: String,
    /// GitHub Discussion number.
    pub discussion_number: u64,
    /// When the discussion was created.
    pub discussion_created_at: String,
}

impl FiledBackport {
    /// Convert to a PendingBackportDiscussion for tracking.
    pub fn to_pending(self, pr_title: &str, commit_sha: &str) -> PendingBackportDiscussion {
        PendingBackportDiscussion {
            bead_id: self.bead_id,
            pr_number: self.pr_number,
            pr_title: pr_title.to_string(),
            commit_sha: commit_sha.to_string(),
            target_branch: self.target_branch,
            discussion_number: self.discussion_number,
            discussion_created_at: self.discussion_created_at,
            approval_result: None,
            execution_result: None,
        }
    }
}

/// Process all backport candidates, creating one backport bead per active branch
/// and one GitHub Discussion per bead for approval gating.
///
/// Implemented as a simple flat map — no internal state machine, no partial progress.
/// If any branch fails, the error is recorded in the result but processing continues
/// for remaining branches.
pub async fn process_candidates(
    candidates: &[BackportCandidate],
    active_branches: &[String],
    github: &GithubClient,
    discussion_category_id: &str,
) -> Result<Vec<BackportResult>, RogersError> {
    let mut results = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        let result =
            process_candidate(candidate, active_branches, github, discussion_category_id).await;
        results.push(result);
    }

    Ok(results)
}

/// Process a single backport candidate, filing beads and discussions for all active branches.
async fn process_candidate(
    candidate: &BackportCandidate,
    active_branches: &[String],
    github: &GithubClient,
    discussion_category_id: &str,
) -> BackportResult {
    let pr = &candidate.pr;
    let mut filed = 0;
    let mut discussions = 0;
    let mut errors = Vec::new();

    for branch in active_branches {
        match file_backport_and_discussion(candidate, branch, github, discussion_category_id).await
        {
            Ok(fb) => {
                info!(
                    "Backport bead filed and discussion created: PR #{} → branch '{}' (priority={}, bead={}, discussion=#{})",
                    pr.number, branch, candidate.priority, fb.bead_id, fb.discussion_number
                );
                filed += 1;
                discussions += 1;
            }
            Err(e) => {
                let msg = format!(
                    "Failed to file backport bead/discussion for branch '{}': {}",
                    branch, e
                );
                tracing::warn!("{}", msg);
                errors.push(msg);
            }
        }
    }

    BackportResult {
        pr_number: pr.number,
        pr_title: pr.title.clone(),
        reason: candidate.reason.clone(),
        branches_filed: filed,
        discussions_created: discussions,
        errors,
    }
}

/// File a single backport bead and create an approval discussion for one target branch.
/// Returns the filed backport details for tracking.
pub async fn file_backport_and_discussion(
    candidate: &BackportCandidate,
    target_branch: &str,
    github: &GithubClient,
    discussion_category_id: &str,
) -> Result<FiledBackport, RogersError> {
    // Build the bead struct
    let bead = BackportBead::build(candidate, target_branch);

    // Submit to bd
    let bead_result = submit_backport_bead(&bead).await?;

    info!(
        "Backport bead created: id={} title=\"{}\"",
        bead_result.id, bead.title
    );

    // Create GitHub Discussion for human approval
    let discussion_body = format_discussion_body(candidate, &bead);

    // Use the bead ID (or PR number as fallback) in discussion title for traceability
    let discussion_title = format!("Backport Approval: {} → {}", bead.title, bead_result.id);

    let discussion = github
        .create_discussion(discussion_category_id, &discussion_title, &discussion_body)
        .await?;

    info!(
        "Approval discussion created: #{} \"{}\" at {}",
        discussion.number, discussion.title, discussion.html_url
    );

    Ok(FiledBackport {
        bead_id: bead_result.id,
        pr_number: candidate.pr.number,
        target_branch: target_branch.to_string(),
        discussion_number: discussion.number,
        discussion_created_at: discussion.created_at.clone(),
    })
}

/// Submit a backport bead via `bd create`.
async fn submit_backport_bead(bead: &BackportBead) -> Result<BeadResult, RogersError> {
    let mut client = BeadClient::new()
        .file_bead(&bead.title, &bead.description, bead.bead_type)
        .with_tag(&bead.tag)
        .with_priority(bead.priority)
        .with_acceptance(&bead.acceptance)
        .with_external_ref(bead.external_ref.as_deref().unwrap_or(""));

    // Add discovered-from dependency if present
    if let Some(ref deps) = bead.deps_arg() {
        client = client.with_deps(deps);
    }

    client.submit().await
}

/// Build the GitHub Discussion body for backport approval.
///
/// Per plan/backport-plan.md:
///
/// ## Backport Proposal
///
/// **Commit:** {sha} — "{message}"
/// **Source issue:** #{number}
/// **Target branch:** release/{X.Y}
///
/// This fix meets backport criteria. Approve by reacting 👍.
/// Backport will be filed as a PR targeting release/{X.Y}.
fn format_discussion_body(candidate: &BackportCandidate, bead: &BackportBead) -> String {
    let pr = &candidate.pr;
    let sha = pr.merge_commit_sha.as_deref().unwrap_or("unknown");

    // Extract issue number for source issue display
    let source_text = extract_issue_display(pr.body.as_deref().unwrap_or(""), pr.number);

    format!(
        "## Backport Proposal\n\n**Commit:** {sha} — \"{title}\"\n\
**Source issue:** {source}\n\
**Target branch:** {branch}\n\n\
This fix meets backport criteria. Approve by reacting 👍.\n\
Backport will be filed as a PR targeting {branch}.\n\n\
---\n\n\
_Backport bead: {bead_id} — filed by Rodgers_",
        sha = sha,
        title = pr.title,
        source = source_text,
        branch = target_branch_from_bead(&bead.title),
        bead_id = bead.title,
    )
}

/// Extract a display string for the source issue from PR body text.
///
/// Tries to find "Closes #N", "Fixes #N", etc. Falls back to "PR #{n}".
fn extract_issue_display(body: &str, pr_number: u64) -> String {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"(?:(?:closes?|fixes?|resolves?)\s+)?#(\d+)")
            .expect("hardcoded regex is valid")
    });
    re.captures(body)
        .and_then(|c| c.get(1))
        .map(|m| format!("#{}", m.as_str()))
        .unwrap_or_else(|| format!("PR #{}", pr_number))
}

/// Extract branch name from bead title like "Backport #abc123d to release/1.x".
fn target_branch_from_bead(title: &str) -> &str {
    title
        .strip_prefix("Backport #")
        .and_then(|s| s.split_whitespace().last())
        .unwrap_or(title)
}

/// Check approval status for all pending backport discussions and execute approved backports.
///
/// This function is called during each triage run to:
/// 1. Monitor voting on discussions via GraphQL
/// 2. Execute backports for approved discussions (within one triage run)
/// 3. Post reminders when voting window expires
/// 4. Close discussions that exceed stale threshold
///
/// Returns discussions with their execution results populated.
pub async fn check_pending_discussions(
    discussions: &[PendingBackportDiscussion],
    github: &GithubClient,
    release_config: &ReleaseConfig,
) -> Result<Vec<PendingBackportDiscussion>, RogersError> {
    let mut results: Vec<PendingBackportDiscussion> = Vec::new();
    let mut needs_reminder: Vec<u64> = Vec::new();
    let mut needs_close: Vec<u64> = Vec::new();

    for discussion in discussions {
        let result = check_approval_status(
            discussion.discussion_number,
            &discussion.discussion_created_at,
            release_config,
            github,
        )
        .await?;

        let mut updated_discussion = discussion.clone();
        updated_discussion.approval_result = Some(result.clone());

        match &result.state {
            ApprovalState::Approved => {
                info!(
                    "Backport approved: discussion #{} for PR #{} → {}",
                    discussion.discussion_number, discussion.pr_number, discussion.target_branch
                );

                // Execute the backport within the SAME triage run
                let sha_short = &discussion.commit_sha[..discussion.commit_sha.len().min(7)];
                let source_issue = extract_source_issue(&discussion.pr_title, discussion.pr_number);

                let execution = execute_backport(
                    &discussion.commit_sha,
                    sha_short,
                    discussion.pr_number,
                    &discussion.pr_title,
                    source_issue,
                    &discussion.target_branch,
                    github,
                    &discussion.bead_id,
                )
                .await;

                match execution {
                    Ok(exec_result) => {
                        if exec_result.is_success() {
                            info!(
                                "Backport execution successful: branch={}, PR=#{}",
                                exec_result.branch_name,
                                exec_result.pr_number.unwrap_or(0)
                            );

                            // CRIT-5: Detect merge conflicts and file conflict-resolution bead.
                            // Per plan/backport-plan.md §Conflict Handling, we detect conflicts
                            // after PR creation and file a bead without autonomous resolution.
                            if let Some(pr_num) = exec_result.pr_number {
                                match wait_for_mergeable(pr_num, github, 10, 2000).await {
                                    Ok(pr) => {
                                        if has_merge_conflicts(&pr) {
                                            info!(
                                                "Merge conflicts detected on backport PR #{} to {}",
                                                pr_num, discussion.target_branch
                                            );
                                            // Handle conflict: file bead, post comment, close discussion.
                                            // No autonomous resolution attempted.
                                            let conflict_result = handle_backport_conflict(
                                                &exec_result,
                                                source_issue,
                                                discussion.pr_number,
                                                &discussion.commit_sha,
                                                sha_short,
                                                &discussion.pr_title,
                                                &discussion.target_branch,
                                                discussion.discussion_number,
                                                github,
                                                release_config,
                                            )
                                            .await;
                                            match conflict_result {
                                                Ok(cr) => {
                                                    if cr.is_success() {
                                                        info!(
                                                            "Conflict handling complete: bead={}, comment_posted={}, discussion_closed={}",
                                                            cr.conflict_bead_id,
                                                            cr.source_comment_posted,
                                                            cr.discussion_closed
                                                        );
                                                    } else {
                                                        tracing::warn!(
                                                            "Conflict handling completed with errors: {:?}",
                                                            cr.errors
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to handle conflict on PR #{}: {}",
                                                        pr_num,
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Could not determine merge status for PR #{}: {}",
                                            pr_num,
                                            e
                                        );
                                    }
                                }
                            }
                        } else {
                            tracing::warn!(
                                "Backport execution had errors: {:?}",
                                exec_result.errors
                            );
                        }
                        updated_discussion.execution_result = Some(exec_result);
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to execute backport for discussion #{}: {}",
                            discussion.discussion_number,
                            e
                        );
                        updated_discussion.execution_result = Some(BackportExecutionResult {
                            bead_id: discussion.bead_id.clone(),
                            branch_name: format!(
                                "backport/{}/{}",
                                sha_short, discussion.target_branch
                            ),
                            pr_number: None,
                            pr_url: None,
                            source_comment_posted: false,
                            errors: vec![e.to_string()],
                        });
                    }
                }
            }
            ApprovalState::Rejected { reason } => {
                tracing::warn!(
                    "Backport rejected for discussion #{} ({}): {}",
                    discussion.discussion_number,
                    discussion.bead_id,
                    reason
                );
                // Post acknowledgment comment
                let author = result
                    .most_recent
                    .as_ref()
                    .map(|v| v.voter.as_str())
                    .unwrap_or("maintainer");
                let comment = format!(
                    "## ❌ Backport Rejected\n\n\
                    @{author} has rejected this backport. Backport will not be filed.\n\n\
                    {reason}\n\n\
                    Please contact a maintainer for guidance."
                );
                let _ = github
                    .create_discussion_comment(discussion.discussion_number, &comment)
                    .await;
            }
            ApprovalState::Stale { reminder_sent: _ } => {
                // Track but don't double-remind in same run
                needs_reminder.push(discussion.discussion_number);
            }
            ApprovalState::Expired => {
                info!(
                    "Backport discussion #{} expired (no response within {} days)",
                    discussion.discussion_number, release_config.stale_threshold_days
                );
                needs_close.push(discussion.discussion_number);
            }
            ApprovalState::Pending => {
                // Still waiting - no action needed
            }
        }

        results.push(updated_discussion);
    }

    // Post reminders (avoid duplicates by collecting unique numbers first)
    let mut seen = std::collections::HashSet::new();
    let unique_reminders: Vec<u64> = needs_reminder
        .into_iter()
        .filter(|n| seen.insert(*n))
        .collect();
    for discussion_number in unique_reminders {
        if let Err(e) = post_reminder_comment(discussion_number, github).await {
            tracing::warn!(
                "Failed to post reminder for discussion #{}: {}",
                discussion_number,
                e
            );
        } else {
            info!("Posted reminder for discussion #{}", discussion_number);
        }
    }

    // Close expired discussions
    for discussion_number in needs_close {
        if let Err(e) = close_discussion(discussion_number, github).await {
            tracing::warn!("Failed to close discussion #{}: {}", discussion_number, e);
        } else {
            info!("Closed expired discussion #{}", discussion_number);
        }
    }

    Ok(results)
}

/// Extract source issue number from PR title/body or fall back to PR number.
fn extract_source_issue(pr_title: &str, pr_number: u64) -> Option<u64> {
    // Try to extract from title (assuming it might contain issue reference)
    // For now, fall back to PR number if no explicit issue reference found
    super::execution::extract_source_issue(pr_title).or(Some(pr_number))
}

impl GithubClient {
    /// Create a reply comment on an existing discussion.
    pub async fn create_discussion_comment(
        &self,
        discussion_number: u64,
        body: &str,
    ) -> Result<(), RogersError> {
        let query = r#"
            mutation($owner: String!, $repo: String!, $discussionNumber: Int!, $body: String!) {
              addDiscussionComment(
                input: {
                  discussionId: $discussionNumber
                  body: $body
                }
              ) {
                comment { id }
              }
            }
        "#;

        #[derive(serde::Serialize)]
        struct GraphQLRequest<'a> {
            query: &'a str,
            variables: serde_json::Value,
        }

        #[derive(serde::Deserialize)]
        struct GraphQLResponse {
            data: Option<serde_json::Value>,
            errors: Option<Vec<serde_json::Value>>,
        }

        let variables = serde_json::json!({
            "owner": self.config().owner,
            "repo": self.config().repo,
            "discussionNumber": discussion_number,
            "body": body
        });

        let request = GraphQLRequest { query, variables };

        let url = format!("{}/graphql", self.config().api_url);
        let resp = self
            .client()
            .post(&url)
            .header("Authorization", &self.auth_header())
            .header("Accept", "application/vnd.github.zzz机构的-preview+json")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let code = resp.status().as_u16();
            let message = resp.text().await.unwrap_or_default();
            return Err(RogersError::GitHubStatus { code, message });
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backport::detector::BackportReason;
    use crate::github::client::{GithubUser, MergedPr};

    fn fake_pr(number: u64, title: &str, sha: Option<&str>, body: Option<&str>) -> MergedPr {
        MergedPr {
            number,
            title: title.to_string(),
            body: body.map(String::from),
            merged_at: Some("2024-01-01T00:00:00Z".to_string()),
            merge_commit_sha: sha.map(String::from),
            user: GithubUser {
                login: "test".to_string(),
                user_type: "User".to_string(),
            },
            labels: vec![],
            state: "closed".to_string(),
        }
    }

    fn fake_candidate(pr: MergedPr, reason: BackportReason, priority: u8) -> BackportCandidate {
        BackportCandidate {
            pr,
            reason,
            priority,
        }
    }

    #[test]
    fn test_backport_result_structure() {
        let pr = fake_pr(
            42,
            "Fix login crash",
            Some("abc123def456abc123"),
            Some("Closes #77"),
        );
        let _c = fake_candidate(pr, BackportReason::BugFix, 2);

        let result = BackportResult {
            pr_number: 42,
            pr_title: "Fix login crash".to_string(),
            reason: BackportReason::BugFix,
            branches_filed: 2,
            discussions_created: 2,
            errors: vec![],
        };

        assert_eq!(result.pr_number, 42);
        assert_eq!(result.branches_filed, 2);
        assert_eq!(result.discussions_created, 2);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_target_branch_from_bead() {
        assert_eq!(
            target_branch_from_bead("Backport #abc123d to release/1.x"),
            "release/1.x"
        );
        assert_eq!(
            target_branch_from_bead("Backport #def456a to release/2.x"),
            "release/2.x"
        );
    }

    #[test]
    fn test_extract_issue_display_with_closes() {
        assert_eq!(
            extract_issue_display("Closes #12345 some text", 99),
            "#12345"
        );
        assert_eq!(extract_issue_display("Fixes #99.", 77), "#99");
    }

    #[test]
    fn test_extract_issue_display_fallback() {
        assert_eq!(
            extract_issue_display("No issue reference here", 42),
            "PR #42"
        );
    }

    #[test]
    fn test_discussion_body_has_all_required_sections() {
        let pr = fake_pr(
            42,
            "Fix critical security vuln",
            Some("abc123def456abc123"),
            Some("Closes #99"),
        );
        let c = fake_candidate(pr, BackportReason::SecurityPatch, 1);
        let bead = BackportBead::build(&c, "release/1.x");
        let body = format_discussion_body(&c, &bead);

        // Required fields per plan specification:
        assert!(body.contains("## Backport Proposal"));
        assert!(body.contains("**Commit:** abc123def456abc123"));
        assert!(body.contains("abc123def456abc123"));
        assert!(body.contains("**Source issue:** #99"));
        assert!(body.contains("#99"));
        assert!(body.contains("**Target branch:** release/1.x"));
        assert!(body.contains("release/1.x"));
        // Approval instruction
        assert!(body.contains("Approve by reacting 👍"));
        assert!(body.to_lowercase().contains("approve"));
    }

    #[test]
    fn test_filed_backport_to_pending() {
        let filed = FiledBackport {
            bead_id: "test-beado-123".to_string(),
            pr_number: 42,
            target_branch: "release/1.x".to_string(),
            discussion_number: 100,
            discussion_created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let pending = filed.to_pending("Fix login bug", "abc123def456abc123");

        assert_eq!(pending.bead_id, "test-beado-123");
        assert_eq!(pending.pr_number, 42);
        assert_eq!(pending.pr_title, "Fix login bug");
        assert_eq!(pending.commit_sha, "abc123def456abc123");
        assert_eq!(pending.target_branch, "release/1.x");
        assert_eq!(pending.discussion_number, 100);
        assert_eq!(pending.discussion_created_at, "2024-01-01T00:00:00Z");
        assert!(pending.approval_result.is_none());
        assert!(pending.execution_result.is_none());
    }
}
