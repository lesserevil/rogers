//! Drift detection module
//!
//! Detects state divergence between GitHub and Backlog.md tasks.
//! Drift events include:
//! - Closed tasks with open GitHub issues
//! - In-progress tasks with closed GitHub issues
//! - Orphan tasks (no GitHub issue link)

use super::{CATEGORY_DRIFT, CategoryResult, CategoryStatus, DriftEvent, DriftSeverity};
use crate::backlog::{BacklogClient, Task};
use crate::error::Result;
use crate::github::auth::GitHubAuth;
use crate::github::GitHubClient;
use crate::github::client::{issue_state, parse_issue_url};
use std::path::Path;

/// Result of drift check including any detected drift events
pub struct DriftCheckResult {
    /// The category result (Pass/Warn with count)
    pub category_result: CategoryResult,
    /// All detected drift events
    pub drift_events: Vec<DriftEvent>,
}

/// Check the drift category
///
/// Detects GitHub ↔ tasks state divergence.
pub async fn check_drift(
    _owner: &str,
    _repo: &str,
    token: &str,
    api_url: Option<&str>,
    verbose: bool,
    backlog_path: &Path,
) -> Result<DriftCheckResult> {
    let mut messages = Vec::new();

    // Create the tasks client (per-issue GitHub clients are built inside the
    // loop below, since each task's issue may live in a different repo).
    let backlog_client = BacklogClient::new(backlog_path);

    // Fetch closed tasks from Backlog.md.
    let closed_tasks: Vec<Task> = match backlog_client.get_closed_tasks().await {
        Ok(tasks) => tasks,
        Err(e) => {
            messages.push(format!("Failed to fetch closed tasks: {}", e));
            return Ok(DriftCheckResult {
                category_result: CategoryResult::fail(
                    CATEGORY_DRIFT,
                    format!("Failed to fetch tasks: {}", e),
                ),
                drift_events: Vec::new(),
            });
        }
    };

    if verbose {
        messages.push(format!(
            "Fetching GitHub issue states for {} closed tasks...",
            closed_tasks.len()
        ));
    }

    // For each closed task with a GitHub issue URL, fetch the issue state
    let mut github_issue_states: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut uncertain_count = 0;

    for task in &closed_tasks {
        let Some(issue_url) = &task.github_issue_url else {
            // No GitHub issue URL - will be caught by detect_drift_events as orphan
            continue;
        };

        // Parse the issue URL to get owner, repo, and issue number
        let Some((issue_owner, issue_repo, issue_number)) = parse_issue_url(issue_url) else {
            tracing::debug!("Could not parse issue URL: {}", issue_url);
            continue;
        };

        // Build a per-repo client for the target issue (may differ from owner/repo)
        let mut issue_client = GitHubClient::new(
            &issue_owner,
            &issue_repo,
            GitHubAuth::new_with_default_api(token),
        );
        if let Some(url) = api_url {
            issue_client =
                GitHubClient::new(&issue_owner, &issue_repo, GitHubAuth::new(token, url));
        }

        // Fetch the issue state from GitHub
        match issue_state(&issue_client, issue_number).await {
            Ok(Some(state)) => {
                github_issue_states.insert(issue_url.clone(), state.to_string());
            }
            Ok(None) => {
                // Issue not found (404) - treat as closed (no drift for this task)
                // We still record it as "closed" to avoid false drift
                github_issue_states.insert(issue_url.clone(), "closed".to_string());
            }
            Err(e) => {
                // GitHub API failure - mark as uncertain
                tracing::warn!("Failed to fetch issue {}: {}", issue_url, e);
                uncertain_count += 1;
            }
        }
    }

    if uncertain_count > 0 {
        messages.push(format!(
            "WARNING: {} GitHub API calls failed (marked as uncertain)",
            uncertain_count
        ));
    }

    // Build task statuses for drift detection
    let task_statuses: Vec<(String, String, Option<String>)> = closed_tasks
        .iter()
        .map(|b| {
            (
                b.id.clone(),
                b.status.to_string(),
                b.github_issue_url.clone(),
            )
        })
        .collect();

    // Detect drift events
    let drift_events = detect_drift_events(&task_statuses, &github_issue_states);

    // Count by type for summary messages
    let closed_tasks_open_issues = drift_events
        .iter()
        .filter(|e| e.event_type == "closed_task_open_issue")
        .count();
    let in_progress_tasks_closed_issues = drift_events
        .iter()
        .filter(|e| e.event_type == "in_progress_task_closed_issue")
        .count();
    let orphan_tasks = drift_events
        .iter()
        .filter(|e| e.event_type == "orphan_task")
        .count();
    let unlabeled_issues = 0; // Not implemented in this version

    messages.push(format!(
        "Closed tasks with open GitHub issues: {} ✓",
        closed_tasks_open_issues
    ));
    messages.push(format!(
        "In-progress tasks with closed GitHub issues: {} ✓",
        in_progress_tasks_closed_issues
    ));
    messages.push(format!(
        "Orphan tasks (no GitHub issue link): {} ✓",
        orphan_tasks
    ));
    messages.push(format!(
        "Issues labeled 'ready-for-work' with no linked task: {} ✓",
        unlabeled_issues
    ));

    let total_events = closed_tasks_open_issues
        + in_progress_tasks_closed_issues
        + orphan_tasks
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
        messages.push("No drift detected — GitHub and tasks state are synchronized ✓".into());
    }

    let status = if total_events > 0 {
        if uncertain_count > 0 {
            // Warnings present due to uncertain GitHub API calls
            CategoryStatus::Warn(vec![format!(
                "{} drift events found ({} uncertain)",
                total_events, uncertain_count
            )])
        } else {
            CategoryStatus::Warn(vec![format!("{} drift events found", total_events)])
        }
    } else if uncertain_count > 0 {
        CategoryStatus::Warn(vec![format!(
            "No drift, but {} GitHub API calls were uncertain",
            uncertain_count
        )])
    } else {
        CategoryStatus::Pass
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

/// Compare GitHub issue state with task state
///
/// Returns drift events if there's a mismatch between GitHub and tasks state.
pub fn detect_drift_events(
    task_statuses: &[(String, String, Option<String>)], // (task_id, status, github_issue_url)
    github_issue_states: &std::collections::HashMap<String, String>, // issue_url -> state
) -> Vec<DriftEvent> {
    let mut events = Vec::new();

    for (task_id, task_status, github_issue_url) in task_statuses {
        let Some(issue_url) = github_issue_url else {
            // Orphan task - no GitHub issue link
            if task_status != "open" {
                events.push(DriftEvent {
                    event_type: "orphan_task".into(),
                    description: format!(
                        "Task {} is '{}' but has no GitHub issue link",
                        task_id, task_status
                    ),
                    github_issue_url: None,
                    task_id: Some(task_id.clone()),
                    severity: DriftSeverity::Warning,
                });
            }
            continue;
        };

        let github_state = github_issue_states
            .get(issue_url)
            .map_or("unknown", |s| s.as_str());

        // Closed tasks with open GitHub issues - drift
        if task_status == "closed" && github_state == "open" {
            events.push(DriftEvent {
                event_type: "closed_task_open_issue".into(),
                description: format!(
                    "Task {} is closed but linked GitHub issue '{}' is open",
                    task_id, issue_url
                ),
                github_issue_url: Some(issue_url.clone()),
                task_id: Some(task_id.clone()),
                severity: DriftSeverity::Error,
            });
        }

        // In-progress tasks with closed GitHub issues - drift
        if task_status == "in_progress" && github_state == "closed" {
            events.push(DriftEvent {
                event_type: "in_progress_task_closed_issue".into(),
                description: format!(
                    "Task {} is in-progress but linked GitHub issue '{}' is closed",
                    task_id, issue_url
                ),
                github_issue_url: Some(issue_url.clone()),
                task_id: Some(task_id.clone()),
                severity: DriftSeverity::Warning,
            });
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== AC-5: Unit tests for drift detection =====

    /// AC-5 Unit test: Closed task + open GitHub issue → drift detected
    #[test]
    fn test_detect_closed_task_open_issue() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "open".into(),
        );

        let task_statuses = vec![(
            "b-001".into(),
            "closed".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&task_statuses, &github_states);

        assert_eq!(events.len(), 1, "Should detect 1 drift event");
        assert_eq!(events[0].event_type, "closed_task_open_issue");
        assert_eq!(events[0].severity, DriftSeverity::Error);
    }

    /// AC-5 Unit test: Closed task + closed GitHub issue → no drift
    #[test]
    fn test_detect_closed_task_closed_issue() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "closed".into(),
        );

        let task_statuses = vec![(
            "b-001".into(),
            "closed".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&task_statuses, &github_states);

        assert!(
            events.is_empty(),
            "No drift when task and issue are both closed"
        );
    }

    /// AC-5 Unit test: Open task + open GitHub issue → no drift
    #[test]
    fn test_detect_open_task_open_issue() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "open".into(),
        );

        let task_statuses = vec![(
            "b-001".into(),
            "open".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&task_statuses, &github_states);

        assert!(
            events.is_empty(),
            "No drift when task and issue are both open"
        );
    }

    /// AC-5 Unit test: Missing GitHub issue (404) → treat as closed, no drift
    ///
    /// When a GitHub issue is deleted (returns 404), we treat it as "closed"
    /// to avoid false drift events. The task was closed, and the issue being
    /// deleted means there's no longer a mismatch to fix.
    #[test]
    fn test_detect_missing_issue_treated_as_closed() {
        let mut github_states = std::collections::HashMap::new();
        // Missing issue is recorded as "closed" in the states map
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "closed".into(),
        );

        let task_statuses = vec![(
            "b-001".into(),
            "closed".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&task_statuses, &github_states);

        // No drift event because we treat missing issue as closed
        assert!(
            events.is_empty(),
            "No drift when missing issue is treated as closed"
        );
    }

    /// AC-5 Unit test: Drift event has issue URL, task ID
    ///
    /// Verifies that when drift is detected, the event contains the
    /// GitHub issue URL and task ID for remediation.
    #[test]
    fn test_drift_event_has_issue_url_and_task_id() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/456".into(),
            "open".into(),
        );

        let task_statuses = vec![(
            "b-789".into(),
            "closed".into(),
            Some("https://github.com/owner/repo/issues/456".into()),
        )];

        let events = detect_drift_events(&task_statuses, &github_states);

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].github_issue_url.as_deref(),
            Some("https://github.com/owner/repo/issues/456"),
            "Event should contain GitHub issue URL"
        );
        assert_eq!(
            events[0].task_id.as_deref(),
            Some("b-789"),
            "Event should contain task ID"
        );
        assert_eq!(events[0].severity, DriftSeverity::Error);
    }

    // ===== Additional drift detection tests =====

    /// Test: In-progress task with closed GitHub issue → drift detected
    #[test]
    fn test_detect_in_progress_task_closed_issue() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "closed".into(),
        );

        let task_statuses = vec![(
            "b-001".into(),
            "in_progress".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&task_statuses, &github_states);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "in_progress_task_closed_issue");
        assert_eq!(events[0].severity, DriftSeverity::Warning);
    }

    /// Test: No drift with matching states (closed task + closed issue, in_progress + in-progress, etc.)
    #[test]
    fn test_no_drift() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "open".into(),
        );

        let task_statuses = vec![(
            "b-001".into(),
            "open".into(),
            Some("https://github.com/owner/repo/issues/123".into()),
        )];

        let events = detect_drift_events(&task_statuses, &github_states);

        assert!(events.is_empty());
    }

    /// Test: Orphan task (no GitHub issue link) → drift event
    #[test]
    fn test_orphan_task() {
        let github_states = std::collections::HashMap::new();

        let task_statuses = vec![("b-001".into(), "closed".into(), None)];

        let events = detect_drift_events(&task_statuses, &github_states);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "orphan_task");
        assert_eq!(events[0].task_id, Some("b-001".into()));
    }

    /// Test: Multiple tasks with mixed drift scenarios
    #[test]
    fn test_multiple_tasks_mixed_drift() {
        let mut github_states = std::collections::HashMap::new();
        github_states.insert(
            "https://github.com/owner/repo/issues/123".into(),
            "open".into(), // Drift: closed task, open issue
        );
        github_states.insert(
            "https://github.com/owner/repo/issues/456".into(),
            "open".into(), // No drift: in_progress matches open
        );
        github_states.insert(
            "https://github.com/owner/repo/issues/789".into(),
            "closed".into(), // Drift: closed task, closed issue - no wait, states match so no drift
        );
        github_states.insert(
            "https://github.com/owner/repo/issues/012".into(),
            "closed".into(), // Drift: in_progress task, closed issue
        );
        github_states.insert(
            "https://github.com/owner/repo/issues/345".into(),
            "open".into(), // No drift: in_progress task, open issue
        );

        let task_statuses = vec![
            // b-001: closed task, issue 123 is open → DRIFT
            (
                "b-001".into(),
                "closed".into(),
                Some("https://github.com/owner/repo/issues/123".into()),
            ),
            // b-002: in_progress task, issue 456 is open → OK, no drift
            (
                "b-002".into(),
                "in_progress".into(),
                Some("https://github.com/owner/repo/issues/456".into()),
            ),
            // b-003: closed task, issue 789 is closed → OK, no drift (states match)
            (
                "b-003".into(),
                "closed".into(),
                Some("https://github.com/owner/repo/issues/789".into()),
            ),
            // b-004: closed task, no issue → orphan drift (non-open status with no link)
            ("b-004".into(), "closed".into(), None),
            // b-005: in_progress task, issue 012 is closed → DRIFT
            (
                "b-005".into(),
                "in_progress".into(),
                Some("https://github.com/owner/repo/issues/012".into()),
            ),
            // b-006: open task, no issue → OK, no orphan warning for open tasks
            ("b-006".into(), "open".into(), None),
            // b-007: in_progress task, issue 345 is open → OK, no drift
            (
                "b-007".into(),
                "in_progress".into(),
                Some("https://github.com/owner/repo/issues/345".into()),
            ),
        ];

        let events = detect_drift_events(&task_statuses, &github_states);

        // Expected drift events:
        // - b-001: closed task with open issue (123)
        // - b-004: closed orphan task (no issue URL)
        // - b-005: in_progress task with closed issue (012)
        assert_eq!(events.len(), 3, "Should detect 3 drift events");
        assert!(
            events.iter().any(|e| e.task_id.as_deref() == Some("b-001")),
            "b-001 should be drifted"
        );
        assert!(
            events.iter().any(|e| e.task_id.as_deref() == Some("b-004")),
            "b-004 orphan should be drifted"
        );
        assert!(
            events.iter().any(|e| e.task_id.as_deref() == Some("b-005")),
            "b-005 should be drifted"
        );
    }
}
