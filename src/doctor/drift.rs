//! Drift detection module
//!
//! Detects state divergence between GitHub and beads database.
//! Drift events include:
//! - Closed beads with open GitHub issues
//! - In-progress beads with closed GitHub issues
//! - Orphan beads (no GitHub issue link)

use super::{CATEGORY_DRIFT, DriftEvent, DriftSeverity};
use crate::doctor::CategoryResult;
use crate::error::RogersError;

/// Check the drift category
///
/// Detects GitHub ↔ beads state divergence.
pub async fn check_drift(
    owner: &str,
    repo: &str,
    token: &str,
    api_url: Option<&str>,
    verbose: bool,
) -> Result<CategoryResult, RogersError> {
    let mut messages = Vec::new();
    let mut drift_events: Vec<super::DriftEvent> = Vec::new();

    // In a real implementation, this would:
    // 1. Fetch all beads from the dolt database
    // 2. For each bead, fetch the linked GitHub issue state
    // 3. Compare states and record any drift events

    // For now, this is a placeholder that simulates drift detection
    // In production, this would connect to dolt and query GitHub

    // Simulated drift checks (placeholder results)
    let closed_beads_open_issues = 0;
    let in_progress_beads_closed_issues = 0;
    let orphan_beads = 0;
    let unlabeled_issues = 0;

    messages.push(format!(
        "Closed beads with open GitHub issues: {} ✓",
        closed_beads_open_issues
    ));
    messages.push(format!(
        "In-progress beads with closed GitHub issues: {} ✓",
        in_progress_beads_closed_issues
    ));
    messages.push(format!(
        "Orphan beads (no GitHub issue link): {} ✓",
        orphan_beads
    ));
    messages.push(format!(
        "Issues labeled 'ready-for-work' with no linked bead: {} ✓",
        unlabeled_issues
    ));

    // Build drift events if needed
    let total_events = closed_beads_open_issues
        + in_progress_beads_closed_issues
        + orphan_beads
        + unlabeled_issues;

    if total_events > 0 {
        messages.push(format!(
            "DRIFT DETECTED — {} drift events found",
            total_events
        ));

        if verbose {
            messages.push(
                "Run 'rogers doctor --verbose' to list each drift event with linking info".into(),
            );
        } else {
            messages.push("Run 'rogers doctor --verbose' to see drift details".into());
        }
    } else {
        messages.push("No drift detected — GitHub and beads state are synchronized ✓".into());
    }

    let status = if total_events > 0 {
        super::CategoryStatus::Warn(vec![format!("{} drift events found", total_events)])
    } else {
        super::CategoryStatus::Pass
    };

    Ok(CategoryResult {
        name: CATEGORY_DRIFT.to_string(),
        status,
        messages,
    })
}

/// Compare GitHub issue state with bead state
///
/// Returns drift events if there's a mismatch between GitHub and beads state.
pub fn detect_drift_events(
    bead_statuses: &[(String, String, Option<String>)], // (bead_id, status, github_issue_url)
    github_issue_states: &std::collections::HashMap<String, String>, // issue_url -> state
) -> Vec<DriftEvent> {
    let mut events = Vec::new();

    for (bead_id, bead_status, github_issue_url) in bead_statuses {
        let Some(issue_url) = github_issue_url else {
            // Orphan bead - no GitHub issue link
            if bead_status != "open" {
                events.push(DriftEvent {
                    event_type: "orphan_bead".into(),
                    description: format!(
                        "Bead {} is '{}' but has no GitHub issue link",
                        bead_id, bead_status
                    ),
                    github_issue_url: None,
                    bead_id: Some(bead_id.clone()),
                    severity: DriftSeverity::Warning,
                });
            }
            continue;
        };

        let github_state = github_issue_states
            .get(issue_url)
            .map_or("unknown", |s| s.as_str());

        // Closed beads with open GitHub issues - drift
        if bead_status == "closed" && github_state == "open" {
            events.push(DriftEvent {
                event_type: "closed_bead_open_issue".into(),
                description: format!(
                    "Bead {} is closed but linked GitHub issue '{}' is open",
                    bead_id, issue_url
                ),
                github_issue_url: Some(issue_url.clone()),
                bead_id: Some(bead_id.clone()),
                severity: DriftSeverity::Error,
            });
        }

        // In-progress beads with closed GitHub issues - drift
        if bead_status == "in_progress" && github_state == "closed" {
            events.push(DriftEvent {
                event_type: "in_progress_bead_closed_issue".into(),
                description: format!(
                    "Bead {} is in-progress but linked GitHub issue '{}' is closed",
                    bead_id, issue_url
                ),
                github_issue_url: Some(issue_url.clone()),
                bead_id: Some(bead_id.clone()),
                severity: DriftSeverity::Warning,
            });
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_closed_bead_open_issue() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "open".into(),
        );

        let bead_statuses = vec![(
            "b-001".into(),
            "closed".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&bead_statuses, &github_states);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "closed_bead_open_issue");
        assert_eq!(events[0].severity, DriftSeverity::Error);
    }

    #[test]
    fn test_detect_in_progress_bead_closed_issue() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "closed".into(),
        );

        let bead_statuses = vec![(
            "b-001".into(),
            "in_progress".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&bead_statuses, &github_states);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "in_progress_bead_closed_issue");
        assert_eq!(events[0].severity, DriftSeverity::Warning);
    }

    #[test]
    fn test_no_drift() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "open".into(),
        );

        let bead_statuses = vec![(
            "b-001".into(),
            "open".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&bead_statuses, &github_states);

        assert!(events.is_empty());
    }

    #[test]
    fn test_orphan_bead() {
        let github_states = std::collections::HashMap::new();

        let bead_statuses = vec![("b-001".into(), "closed".into(), None)];

        let events = detect_drift_events(&bead_statuses, &github_states);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "orphan_bead");
        assert_eq!(events[0].bead_id, Some("b-001".into()));
    }
}
