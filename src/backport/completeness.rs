//! Release completeness checking for backports.
//!
//! When a backport PR is merged, Rodgers checks whether all critical backports
//! for a given release version have been merged. If complete, a release
//! suggestion bead is filed.
//!
//! Per plan/backport-plan.md §Integration with Release Management:
//! - Critical backports: Security + high priority bug fixes (priority=1)
//! - Non-critical: Normal bug fixes and backport-me requests (priority=2)
//! - Release completeness: All critical backports for a version are merged
//!
//! ## Tracking model
//!
//! Beads tagged `rodgers:type=backport` track backport state per version.
//! A backport is "complete" when its bead is closed (merged or explicitly
//! cancelled). Completeness is evaluated per target branch (version).
//!
//! ## Release suggestion bead
//!
//! When all critical backports for a release branch are complete, Rodgers
//! files a release suggestion bead (type=chore, tag=rodgers:type=release)
//! that triggers the release manager workflow.

use tracing::info;

/// Represents a backport bead and its current state.
#[derive(Debug, Clone)]
pub struct BackportBeadInfo {
    /// The bead ID.
    pub id: String,
    /// The target release branch (e.g., "release/1.x").
    pub target_branch: String,
    /// Whether this is a critical backport (priority=1).
    pub is_critical: bool,
    /// Whether the backport bead is closed.
    pub is_closed: bool,
    /// The source commit SHA.
    pub source_sha: Option<String>,
    /// The source PR number.
    pub source_pr: Option<u64>,
}

/// Result of a completeness check for one release branch.
#[derive(Debug, Clone)]
pub struct CompletenessResult {
    /// The release branch that was evaluated.
    pub release_branch: String,
    /// Total number of backport beads for this branch.
    pub total_beads: usize,
    /// Number of critical (priority=1) backport beads.
    pub critical_beads: usize,
    /// Number of critical beads that are closed.
    pub critical_closed: usize,
    /// Number of non-critical beads that are closed.
    pub non_critical_closed: usize,
    /// Whether all critical backports are complete.
    pub is_complete: bool,
    /// Beads that remain open (both critical and non-critical).
    pub remaining_beads: Vec<BackportBeadInfo>,
}

impl CompletenessResult {
    /// Returns true if all critical backports are merged (bead closed).
    ///
    /// A release suggestion is warranted only when critical=closed AND
    /// there are critical beads tracking. Empty critical set is not
    /// considered "complete" (no reason to release).
    pub fn should_suggest_release(&self) -> bool {
        self.critical_beads > 0 && self.critical_beads == self.critical_closed
    }
}

/// Check if all critical backports for a release branch are complete.
///
/// A "complete" set means every backport bead with priority=1 (security/high
/// priority) is in a closed state. Non-critical (priority=2) beads do not
/// gate the release suggestion.
///
/// # Arguments
/// - `branch`: The target release branch to check (e.g., "release/1.x")
/// - `beads`: List of backport bead infos for this branch
///
/// # Returns
/// A `CompletenessResult` with details about the check.
pub fn check_branch_completeness(branch: &str, beads: &[BackportBeadInfo]) -> CompletenessResult {
    let critical_beads: Vec<_> = beads.iter().filter(|b| b.is_critical).collect();

    let critical_open: Vec<_> = critical_beads.iter().filter(|b| !b.is_closed).collect();

    let critical_closed = critical_beads.iter().filter(|b| b.is_closed).count();

    let non_critical_closed = beads
        .iter()
        .filter(|b| !b.is_critical && b.is_closed)
        .count();

    // "Complete" means all critical beads are closed.
    // With no critical beads: is_complete = false (nothing to declare "complete")
    // so semantics are clear: "complete" implies work was tracked AND finished.
    let is_complete = if critical_beads.is_empty() {
        false
    } else {
        critical_open.is_empty()
    };

    let remaining_beads: Vec<_> = beads.iter().filter(|b| !b.is_closed).cloned().collect();

    if is_complete && !critical_beads.is_empty() {
        info!(
            "Release completeness achieved for {}: all {} critical backports closed",
            branch,
            critical_beads.len()
        );
    }

    CompletenessResult {
        release_branch: branch.to_string(),
        total_beads: beads.len(),
        critical_beads: critical_beads.len(),
        critical_closed,
        non_critical_closed,
        is_complete,
        remaining_beads,
    }
}

/// Aggregate completeness results across all active branches.
///
/// Returns branches where release suggestion is warranted.
pub fn aggregate_results(results: &[CompletenessResult]) -> Vec<&CompletenessResult> {
    results
        .iter()
        .filter(|r| r.should_suggest_release())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bead(id: &str, branch: &str, priority: u8, closed: bool) -> BackportBeadInfo {
        BackportBeadInfo {
            id: id.to_string(),
            target_branch: branch.to_string(),
            is_critical: priority == 1,
            is_closed: closed,
            source_sha: Some("abc123".to_string()),
            source_pr: Some(42),
        }
    }

    #[test]
    fn test_all_critical_closed_is_complete() {
        let beads = vec![
            make_bead("b1", "release/1.x", 1, true),
            make_bead("b2", "release/1.x", 1, true),
        ];

        let result = check_branch_completeness("release/1.x", &beads);

        assert!(result.is_complete);
        assert!(result.should_suggest_release());
        assert_eq!(result.critical_beads, 2);
        assert_eq!(result.critical_closed, 2);
        assert!(result.remaining_beads.is_empty());
    }

    #[test]
    fn test_one_critical_open_is_not_complete() {
        let beads = vec![
            make_bead("b1", "release/1.x", 1, true),
            make_bead("b2", "release/1.x", 1, false), // still open
        ];

        let result = check_branch_completeness("release/1.x", &beads);

        assert!(!result.is_complete);
        assert!(!result.should_suggest_release());
        assert_eq!(result.critical_beads, 2);
        assert_eq!(result.critical_closed, 1);
        assert_eq!(result.remaining_beads.len(), 1);
    }

    #[test]
    fn test_empty_critical_set_is_not_complete() {
        let beads = vec![
            make_bead("b1", "release/1.x", 2, true), // non-critical only
            make_bead("b2", "release/1.x", 2, true),
        ];

        let result = check_branch_completeness("release/1.x", &beads);

        assert!(!result.is_complete);
        assert!(!result.should_suggest_release()); // no critical beads
        assert_eq!(result.critical_beads, 0);
        assert!(result.remaining_beads.is_empty());
    }

    #[test]
    fn test_non_critical_does_not_gate_release() {
        let beads = vec![
            make_bead("b1", "release/1.x", 2, false), // non-critical, open
        ];

        let result = check_branch_completeness("release/1.x", &beads);

        // is_complete is false when there are no critical beads at all
        // (nothing tracked as "complete" - semantics: no critical work exists)
        assert!(!result.is_complete);
        // Non-critical beads being open does NOT gate the release
        assert!(!result.should_suggest_release()); // no critical beads to begin with
        assert_eq!(result.critical_beads, 0);
        assert_eq!(result.remaining_beads.len(), 1);
    }

    #[test]
    fn test_mixed_critical_and_non_critical() {
        let beads = vec![
            make_bead("b1", "release/1.x", 1, true),
            make_bead("b2", "release/1.x", 1, true),
            make_bead("b3", "release/1.x", 2, false), // non-critical open
            make_bead("b4", "release/1.x", 2, true),
        ];

        let result = check_branch_completeness("release/1.x", &beads);

        assert!(result.is_complete);
        assert!(result.should_suggest_release());
        assert_eq!(result.critical_beads, 2);
        assert_eq!(result.critical_closed, 2);
        assert_eq!(result.non_critical_closed, 1);
        assert_eq!(result.remaining_beads.len(), 1); // only non-critical remains
        assert_eq!(result.remaining_beads[0].id, "b3");
    }

    #[test]
    fn test_aggregate_results() {
        let results = vec![
            CompletenessResult {
                release_branch: "release/1.x".to_string(),
                total_beads: 3,
                critical_beads: 2,
                critical_closed: 2,
                non_critical_closed: 1,
                is_complete: true,
                remaining_beads: vec![],
            },
            CompletenessResult {
                release_branch: "release/2.x".to_string(),
                total_beads: 3,
                critical_beads: 2,
                critical_closed: 1,
                non_critical_closed: 1,
                is_complete: false,
                remaining_beads: vec![],
            },
            CompletenessResult {
                release_branch: "release/3.x".to_string(),
                total_beads: 1,
                critical_beads: 1,
                critical_closed: 1,
                non_critical_closed: 0,
                is_complete: true,
                remaining_beads: vec![],
            },
        ];

        let ready = aggregate_results(&results);

        assert_eq!(ready.len(), 2);
        assert_eq!(ready[0].release_branch, "release/1.x");
        assert_eq!(ready[1].release_branch, "release/3.x");
    }

    #[test]
    fn test_completeness_result_structure() {
        let result = CompletenessResult {
            release_branch: "release/1.x".to_string(),
            total_beads: 5,
            critical_beads: 2,
            critical_closed: 2,
            non_critical_closed: 3,
            is_complete: true,
            remaining_beads: vec![],
        };

        assert_eq!(result.release_branch, "release/1.x");
        assert_eq!(result.total_beads, 5);
        assert!(result.is_complete);
        assert!(result.should_suggest_release());
    }
}
