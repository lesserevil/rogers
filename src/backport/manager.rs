//! Backport manager — processes detected backport candidates.
//!
//! Entry point: `process_candidates`. For each candidate, this module files
//! one backport bead per active release branch, plus creates a GitHub Discussion
//! for each bead to gate human approval.
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

use tracing::{info, warn};

use super::bead::BackportBead;
use super::detector::{BackportCandidate, BackportReason};
use crate::RogersError;
use crate::beads::BeadClient;
use crate::beads::client::BeadResult;
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
            Ok(()) => {
                info!(
                    "Backport bead filed and discussion created: PR #{} → branch '{}' (priority={})",
                    pr.number, branch, candidate.priority
                );
                filed += 1;
                discussions += 1;
            }
            Err(e) => {
                let msg = format!(
                    "Failed to file backport bead/discussion for branch '{}': {}",
                    branch, e
                );
                warn!("{}", msg);
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
async fn file_backport_and_discussion(
    candidate: &BackportCandidate,
    target_branch: &str,
    github: &GithubClient,
    discussion_category_id: &str,
) -> Result<(), RogersError> {
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

    Ok(())
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
        // Approval instruction — match the actual capital-A text in the body
        assert!(body.contains("Approve by reacting 👍"));
        assert!(body.to_lowercase().contains("approve"));
    }
}
