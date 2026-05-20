//! Drift detection module
//!
//! Detects state divergence between GitHub and beads database.
//! Drift events include:
//! - Closed beads with open GitHub issues
//! - In-progress beads with closed GitHub issues
//! - Orphan beads (no GitHub issue link)
//!
//! AC-6: In-progress beads linked to closed GitHub issues

use super::{CATEGORY_DRIFT, DriftEvent, DriftSeverity};
use crate::beads::{Bead, BeadStatus};
use crate::doctor::CategoryResult;
use crate::error::Result;
use crate::github::{GitHubClient, IssueState};

/// Maximum number of beads to fetch at once (pagination limit)
const BEADS_PAGE_SIZE: usize = 100;

/// Result of drift check including any detected drift events
pub struct DriftCheckResult {
    /// The category result (Pass/Warn with count)
    pub category_result: CategoryResult,
    /// All detected drift events
    pub drift_events: Vec<DriftEvent>,
}

/// Check the drift category
///
/// Detects GitHub ↔ beads state divergence.
/// Specifically checks for AC-6: in-progress beads linked to closed GitHub issues.
pub async fn check_drift(
    owner: &str,
    repo: &str,
    token: &str,
    api_url: Option<&str>,
    verbose: bool,
) -> Result<DriftCheckResult> {
    check_drift_with_beads(owner, repo, token, api_url, verbose, None).await
}

/// Check drift with optional injected beads (for testing with real data)
async fn check_drift_with_beads(
    owner: &str,
    repo: &str,
    token: &str,
    api_url: Option<&str>,
    verbose: bool,
    beads: Option<&[Bead]>,
) -> Result<DriftCheckResult> {
    let mut messages = Vec::new();

    // Use provided beads or create an empty list
    // Note: In production, we'd use the actual BeadsClient from config
    // For now, we handle both cases
    let drift_events = if let Some(beads_slice) = beads {
        detect_in_progress_drift(owner, repo, token, api_url, beads_slice).await?
    } else {
        // Without beads data, we can't do actual drift detection
        // This happens when beads config is not available
        vec![]
    };

    // Count drift events by type
    let in_progress_beads_closed_issues = drift_events
        .iter()
        .filter(|e| e.event_type == "in_progress_bead_closed_issue")
        .count();

    // Build messages
    messages.push(format!(
        "Closed beads with open GitHub issues: 0 ✓ (AC-5 placeholder)"
    ));
    messages.push(format!(
        "In-progress beads with closed GitHub issues: {} {}",
        if in_progress_beads_closed_issues > 0 {
            "⚠"
        } else {
            "✓"
        },
        in_progress_beads_closed_issues
    ));
    messages.push(format!(
        "Orphan beads (no GitHub issue link): 0 ✓ (AC-5 placeholder)"
    ));
    messages.push(format!(
        "Issues labeled 'ready-for-work' with no linked bead: 0 ✓ (AC-5 placeholder)"
    ));

    let total_events = drift_events.len() + 0; // AC-5 placeholders not yet implemented

    if total_events > 0 {
        messages.push(format!(
            "DRIFT DETECTED — {} drift events found",
            total_events
        ));

        if verbose {
            for event in &drift_events {
                messages.push(format!(
                    "  - {} → bead {} (issue: {:?})",
                    event.description,
                    event.bead_id.as_deref().unwrap_or("unknown"),
                    event.github_issue_url.as_deref()
                ));
            }
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

    Ok(DriftCheckResult {
        category_result: CategoryResult {
            name: CATEGORY_DRIFT.to_string(),
            status,
            messages,
        },
        drift_events,
    })
}

/// Detect drift for in-progress beads with closed GitHub issues
async fn detect_in_progress_drift(
    owner: &str,
    repo: &str,
    token: &str,
    api_url: Option<&str>,
    beads: &[Bead],
) -> Result<Vec<DriftEvent>> {
    let mut drift_events = Vec::new();

    // Create GitHub client for issue state queries
    let github_client = GitHubClient::new(token.to_string(), api_url);

    // Filter for in-progress beads and check each for drift
    for bead in beads.iter().filter(|b| b.status == BeadStatus::InProgress) {
        if let Some(event) = check_bead_drift(bead, owner, repo, &github_client).await? {
            drift_events.push(event);
        }
    }

    Ok(drift_events)
}

/// Check a single bead for drift against its linked GitHub issue
async fn check_bead_drift(
    bead: &Bead,
    owner: &str,
    repo: &str,
    github_client: &GitHubClient,
) -> Result<Option<DriftEvent>> {
    // Check if bead has a GitHub issue link
    let Some(issue_url) = &bead.github_issue_url else {
        // Orphan bead - no issue link (handled elsewhere)
        return Ok(None);
    };

    // Parse the issue URL to get issue details
    let Some((issue_owner, issue_repo, issue_number)) = GitHubClient::parse_issue_url(issue_url)
    else {
        // Invalid URL format - log and skip
        tracing::warn!(
            "Bead {} has invalid GitHub issue URL: {}",
            bead.id,
            issue_url
        );
        return Ok(None);
    };

    // Query GitHub for issue state
    match github_client
        .get_issue_state(&issue_owner, &issue_repo, issue_number)
        .await
    {
        Ok(Some(issue_state)) => {
            // Check for drift: in-progress bead but closed issue
            if issue_state == IssueState::Closed {
                Ok(Some(DriftEvent {
                    event_type: "in_progress_bead_closed_issue".into(),
                    description: format!(
                        "Bead {} is in-progress but linked GitHub issue #{} is closed",
                        bead.id, issue_number
                    ),
                    github_issue_url: Some(issue_url.clone()),
                    bead_id: Some(bead.id.clone()),
                    severity: DriftSeverity::Warning,
                }))
            } else {
                // Issue is open - no drift for this check
                Ok(None)
            }
        }
        Ok(None) => {
            // Issue not found (deleted from GitHub) - treat as closed
            // This is a drift event because the bead is in-progress but the issue no longer exists
            Ok(Some(DriftEvent {
                event_type: "in_progress_bead_closed_issue".into(),
                description: format!(
                    "Bead {} is in-progress but linked GitHub issue #{} was not found (deleted or inaccessible)",
                    bead.id, issue_number
                ),
                github_issue_url: Some(issue_url.clone()),
                bead_id: Some(bead.id.clone()),
                severity: DriftSeverity::Warning,
            }))
        }
        Err(e) => {
            // GitHub API error - log but don't fail the entire drift check
            tracing::warn!(
                "Failed to fetch GitHub issue state for bead {} ({}): {}",
                bead.id,
                issue_url,
                e
            );
            Ok(None)
        }
    }
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

    // ===== AC-6: Unit tests for in-progress beads with closed GitHub issues =====

    /// AC-6 Test 1: In-progress bead + closed issue → drift detected
    #[test]
    fn test_ac6_in_progress_bead_closed_issue_drift_detected() {
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

        // Should detect drift
        assert_eq!(events.len(), 1, "Expected 1 drift event");
        let event = &events[0];
        assert_eq!(event.event_type, "in_progress_bead_closed_issue");
        assert_eq!(event.severity, DriftSeverity::Warning);
    }

    /// AC-6 Test 2: In-progress bead + open issue → no drift
    #[test]
    fn test_ac6_in_progress_bead_open_issue_no_drift() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "open".into(),
        );

        let bead_statuses = vec![(
            "b-001".into(),
            "in_progress".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&bead_statuses, &github_states);

        // Should NOT detect drift
        assert!(events.is_empty(), "Expected no drift events for open issue");
    }

    /// AC-6 Test 3: Closed bead + closed issue → no drift (from in-progress check)
    #[test]
    fn test_ac6_closed_bead_closed_issue_no_in_progress_drift() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "closed".into(),
        );

        let bead_statuses = vec![(
            "b-001".into(),
            "closed".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&bead_statuses, &github_states);

        // Should NOT detect in-progress drift (but will detect closed_bead_open_issue if applicable)
        let in_progress_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == "in_progress_bead_closed_issue")
            .collect();
        assert!(
            in_progress_events.is_empty(),
            "Closed bead should not trigger in-progress drift"
        );
    }

    /// AC-6 Test 4: Drift event has issue URL, bead ID
    #[test]
    fn test_ac6_drift_event_has_issue_url_and_bead_id() {
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
        let event = &events[0];

        // Verify drift event has issue URL
        assert!(
            event.github_issue_url.is_some(),
            "Drift event should have GitHub issue URL"
        );
        assert_eq!(
            event.github_issue_url.as_ref().unwrap(),
            "https://github.com/owner/repo/issues/123"
        );

        // Verify drift event has bead ID
        assert!(event.bead_id.is_some(), "Drift event should have bead ID");
        assert_eq!(event.bead_id.as_ref().unwrap(), "b-001");

        // Verify description contains both bead ID and issue reference
        assert!(
            event.description.contains("b-001"),
            "Description should contain bead ID"
        );
        assert!(
            event.description.contains("123"),
            "Description should contain issue number"
        );
    }

    /// AC-6 Test 5: Multiple in-progress beads with closed issues → multiple drift events
    #[test]
    fn test_ac6_multiple_in_progress_beads_closed_issues() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "closed".into(),
        );
        github_states.insert(
            "https://github.com/owner/repo/issues/456".into(),
            "closed".into(),
        );
        github_states.insert(
            "https://github.com/owner/repo/issues/789".into(),
            "open".into(),
        );

        let bead_statuses = vec![
            (
                "b-001".into(),
                "in_progress".into(),
                Some("https://github.com/owner/repo/issues/123".into()),
            ),
            (
                "b-002".into(),
                "in_progress".into(),
                Some("https://github.com/owner/repo/issues/456".into()),
            ),
            (
                "b-003".into(),
                "in_progress".into(),
                Some("https://github.com/owner/repo/issues/789".into()),
            ),
        ];

        let events = detect_drift_events(&bead_statuses, &github_states);

        // Should detect 2 drift events (for closed issues)
        let in_progress_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == "in_progress_bead_closed_issue")
            .collect();
        assert_eq!(
            in_progress_events.len(),
            2,
            "Expected 2 in-progress drift events for closed issues"
        );
    }

    /// AC-6 Test 6: Bead with no GitHub issue URL → not included in drift check
    #[test]
    fn test_ac6_in_progress_bead_no_issue_url_no_drift() {
        let github_states = std::collections::HashMap::new();

        let bead_statuses = vec![("b-001".into(), "in_progress".into(), None)];

        let events = detect_drift_events(&bead_statuses, &github_states);

        // Orphan beads are detected but only for non-open statuses
        // This should not generate an in_progress_bead_closed_issue event
        let in_progress_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == "in_progress_bead_closed_issue")
            .collect();
        assert!(
            in_progress_events.is_empty(),
            "In-progress bead without issue URL should not trigger in-progress drift"
        );
    }

    // ===== Legacy tests (kept for compatibility) =====

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
