//! Backport manager — processes detected backport candidates.
//!
//! Entry point: `process_candidates`. For each candidate, this module files
//! one backport bead per active release branch.
//!
//! Bead shape follows plans/backport-plan.md:
//!   title: "Backport #{sha_short} to {branch_name}"
//!   type:  chore
//!   tag:   rodgers:type=backport
//!   priority: 1 for security, 2 for bug/backport-me
//!
//! The bead is linked back to the source GitHub issue via `discovered-from`
//! if the beads tracking system supports it.

use tracing::info;

use super::detector::{BackportCandidate, BackportReason};
use crate::RogersError;
use crate::beads::BeadClient;

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
    /// Any errors encountered.
    pub errors: Vec<String>,
}

/// Process all backport candidates, creating one backport bead per active branch.
///
/// Implemented as a simple flat map — no internal state machine, no partial progress.
/// If any branch fails, the error is recorded in the result but processing continues
/// for remaining branches.
pub async fn process_candidates(
    candidates: &[BackportCandidate],
    active_branches: &[String],
) -> Result<Vec<BackportResult>, RogersError> {
    let mut results = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        let result = process_candidate(candidate, active_branches).await;
        results.push(result);
    }

    Ok(results)
}

/// Process a single backport candidate, filing beads for all active branches.
async fn process_candidate(
    candidate: &BackportCandidate,
    active_branches: &[String],
) -> BackportResult {
    let pr = &candidate.pr;
    let mut filed = 0;
    let mut errors = Vec::new();

    for branch in active_branches {
        match file_backport_bead(candidate, branch).await {
            Ok(_) => {
                info!(
                    "Backport bead filed: PR #{} → branch '{}' (priority={})",
                    pr.number, branch, candidate.priority
                );
                filed += 1;
            }
            Err(e) => {
                let msg = format!(
                    "Failed to file backport bead for branch '{}': {}",
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
        errors,
    }
}

/// File a single backport bead for one target release branch.
async fn file_backport_bead(
    candidate: &BackportCandidate,
    target_branch: &str,
) -> Result<(), RogersError> {
    let pr = &candidate.pr;
    let sha = pr.merge_commit_sha.as_deref().unwrap_or("unknown");
    let sha_short = &sha[..sha.len().min(7)];

    let priority = candidate.priority;

    let title = format!("Backport #{sha_short} to {target_branch}");

    let description = format!(
        "Plan: plans/backport-plan.md\n\n\
Backport for: #{sha} — \"{title_text}\"\n\
Source PR: #{pr_number}\n\
Target branch: {target_branch}\n\
Priority: {priority}\n\n\
WHAT TO DO\n\
Cherry-pick commit #{sha} to {target_branch}. Create a PR targeting\n\
{target_branch} with the cherry-pick. Resolve any merge conflicts.\n\n\
ACCEPTANCE\n\
- [ ] Cherry-pick of #{sha} applies cleanly to {target_branch} (or conflicts resolved)\n\
- [ ] PR is open targeting {target_branch}\n\
- [ ] CI passes on the backport PR\n\
- [ ] PR is merged or given explicit approval to close without merging\n\n\
PITFALLS\n\
- If the fix requires changes to shared library code that has diverged\n\
  between main and {target_branch}, the cherry-pick may require\n\
  manual conflict resolution. Document any non-trivial conflicts\n\
  in this bead before closing.",
        sha = sha,
        title_text = pr.title,
        pr_number = pr.number,
        target_branch = target_branch,
        priority = priority,
    );

    let tag = "rodgers:type=backport";
    let acceptance = format!(
        "Backport #{sha} to {branch} is merged or explicitly closed without merging",
        sha = sha,
        branch = target_branch
    );

    // Build and submit the bead
    BeadClient::new()
        .file_bead(&title, &description, "chore")
        .with_tag(tag)
        .with_priority(priority)
        .with_acceptance(&acceptance)
        .submit()
        .await
        .map_err(|e| RogersError::Beads(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backport::detector::{BackportCandidate, BackportReason};
    use crate::github::client::{GithubLabel, GithubUser, MergedPr};

    fn fake_pr(number: u64, title: &str, sha: Option<&str>) -> MergedPr {
        MergedPr {
            number,
            title: title.to_string(),
            body: Some(format!("Closes #{}", number)),
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

    fn fake_candidate(pr: MergedPr, reason: BackportReason) -> BackportCandidate {
        BackportCandidate::new(pr, reason)
    }

    #[test]
    fn test_backport_result_structure() {
        let pr = fake_pr(42, "Fix login crash", Some("abc123def456abc123"));
        let c = fake_candidate(pr, BackportReason::BugFix);

        let result = BackportResult {
            pr_number: 42,
            pr_title: "Fix login crash".to_string(),
            reason: BackportReason::BugFix,
            branches_filed: 2,
            errors: vec![],
        };

        assert_eq!(result.pr_number, 42);
        assert_eq!(result.branches_filed, 2);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_priority_in_result() {
        let pr = fake_pr(1, "Fix CVE-2024-99999", Some("abc123def456abc123"));
        let c = fake_candidate(pr, BackportReason::SecurityPatch);

        assert_eq!(c.priority, 1);

        let pr2 = fake_pr(2, "Fix bug", None);
        let c2 = fake_candidate(pr2, BackportReason::BugFix);
        assert_eq!(c2.priority, 2);
    }
}
