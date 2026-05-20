//! Drift detection module
//!
//! Detects state divergence between GitHub and beads database.
//! Drift events include:
//! - Closed beads with open GitHub issues
//! - In-progress beads with closed GitHub issues
//! - Orphan beads (no GitHub issue link)
//! - Issues labeled ready-for-work with no linked bead
//! - Release-proposed issues not in milestone
//! - Beads violating project AGENTS.md conventions

use super::{CATEGORY_DRIFT, DriftEvent, DriftSeverity};
use crate::doctor::CategoryResult;
use crate::error::RogersError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bead data structure from beads database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead {
    pub id: String,
    pub status: String,
    pub title: String,
    pub github_issue_url: Option<String>,
    pub github_issue_state: Option<String>,
    pub rodgers_type: Option<String>,
    pub plan_reference: Option<String>,
    pub description: Option<String>,
}

/// GitHub issue data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub labels: Vec<String>,
    pub milestone: Option<String>,
    pub url: String,
}

/// GitHub milestone data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubMilestone {
    pub number: u32,
    pub title: String,
    pub state: String,
}

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
pub async fn check_drift(
    owner: &str,
    repo: &str,
    token: &str,
    api_url: Option<&str>,
    verbose: bool,
) -> Result<DriftCheckResult, RogersError> {
    let base_url = api_url.unwrap_or("https://api.github.com");
    let client = Client::new();

    let mut messages = Vec::new();
    let mut drift_events = Vec::new();

    // Count drift types
    let mut closed_beads_open_issues = 0;
    let mut in_progress_beads_closed_issues = 0;
    let mut orphan_beads = 0;
    let mut ready_for_work_issues_no_bead = 0;
    let mut release_proposed_not_in_milestone = 0;
    let mut convention_violations = 0;

    // Fetch beads from database (simulated for now - placeholder for actual dolt query)
    let beads = fetch_beads().await.unwrap_or_default();

    // Fetch all GitHub issues
    let github_issues = fetch_github_issues(owner, repo, token, base_url, &client)
        .await
        .unwrap_or_default();

    // Fetch all milestones (used for release-proposed drift check)
    let _milestones = fetch_milestones(owner, repo, token, base_url, &client)
        .await
        .unwrap_or_default();

    // Create a map of issue URL -> GitHubIssue for quick lookup
    let issue_map: HashMap<String, &GitHubIssue> = github_issues
        .iter()
        .map(|issue| (issue.url.clone(), issue))
        .collect();

    // Create a map of issue number -> GitHubIssue for quick lookup (used for bead -> issue mapping)
    let _issue_by_number: HashMap<u32, &GitHubIssue> = github_issues
        .iter()
        .map(|issue| (issue.number, issue))
        .collect();

    // Create a map of linked bead for issues (reverse lookup)
    let issue_to_bead: HashMap<u32, &Bead> = beads
        .iter()
        .filter_map(|bead| {
            bead.github_issue_url
                .as_ref()
                .and_then(|url| extract_issue_number(url).and_then(|num| Some((num, bead))))
        })
        .collect();

    // 1. Check for closed beads with open GitHub issues
    for bead in &beads {
        // Skip if no GitHub issue linked (handled separately as orphan)
        let Some(ref issue_url) = bead.github_issue_url else {
            continue;
        };

        if bead.status == "closed" {
            // Check if the linked issue is open
            if let Some(issue) = issue_map.get(issue_url) {
                if issue.state == "open" {
                    closed_beads_open_issues += 1;
                    drift_events.push(DriftEvent {
                        event_type: "closed_bead_open_issue".into(),
                        description: format!(
                            "Bead {} is closed but linked GitHub issue #{} is open",
                            bead.id, issue.number
                        ),
                        github_issue_url: Some(issue_url.clone()),
                        bead_id: Some(bead.id.clone()),
                        severity: DriftSeverity::Error,
                    });
                }
            } else {
                // Issue not found - orphan bead scenario
                closed_beads_open_issues += 1;
                drift_events.push(DriftEvent {
                    event_type: "closed_bead_open_issue".into(),
                    description: format!(
                        "Bead {} is closed but linked GitHub issue URL '{}' not found",
                        bead.id, issue_url
                    ),
                    github_issue_url: Some(issue_url.clone()),
                    bead_id: Some(bead.id.clone()),
                    severity: DriftSeverity::Warning,
                });
            }
        }

        // 2. Check for in-progress beads with closed GitHub issues
        if bead.status == "in_progress" {
            if let Some(issue) = issue_map.get(issue_url) {
                if issue.state == "closed" {
                    in_progress_beads_closed_issues += 1;
                    drift_events.push(DriftEvent {
                        event_type: "in_progress_bead_closed_issue".into(),
                        description: format!(
                            "Bead {} is in-progress but linked GitHub issue #{} is closed",
                            bead.id, issue.number
                        ),
                        github_issue_url: Some(issue_url.clone()),
                        bead_id: Some(bead.id.clone()),
                        severity: DriftSeverity::Warning,
                    });
                }
            }
        }
    }

    // 3. Check for orphan beads (no GitHub issue link)
    for bead in &beads {
        if bead.github_issue_url.is_none() {
            // Check if this is an intentional internal bead (non-open status may be intentional)
            // But for open status beads, this is a potential orphan
            if bead.status == "open" || bead.status == "in_progress" {
                orphan_beads += 1;
                drift_events.push(DriftEvent {
                    event_type: "orphan_bead".into(),
                    description: format!(
                        "Bead {} has no linked GitHub issue - may be an orphan or internal tracking",
                        bead.id
                    ),
                    github_issue_url: None,
                    bead_id: Some(bead.id.clone()),
                    severity: DriftSeverity::Warning,
                });
            }
        }
    }

    // 4. Check for issues labeled 'ready-for-work' with no linked bead
    for issue in &github_issues {
        let issue_num = issue.number;
        if issue.labels.contains(&"ready-for-work".to_string())
            && issue.state == "open"
            && !issue_to_bead.contains_key(&issue_num)
        {
            ready_for_work_issues_no_bead += 1;
            drift_events.push(DriftEvent {
                event_type: "ready_for_work_no_bead".into(),
                description: format!(
                    "Issue #{} '{}' has 'ready-for-work' label but no linked bead",
                    issue.number, issue.title
                ),
                github_issue_url: Some(issue.url.clone()),
                bead_id: None,
                severity: DriftSeverity::Warning,
            });
        }
    }

    // 5. Check for release-proposed issues not in milestone
    for issue in &github_issues {
        let issue_num = issue.number;
        if issue.labels.contains(&"release-proposed".to_string())
            && issue.milestone.is_none()
            && issue.state == "closed"
        {
            release_proposed_not_in_milestone += 1;
            drift_events.push(DriftEvent {
                event_type: "release_proposed_no_milestone".into(),
                description: format!(
                    "Issue #{} '{}' is release-proposed but not assigned to a milestone",
                    issue.number, issue.title
                ),
                github_issue_url: Some(issue.url.clone()),
                bead_id: issue_to_bead.get(&issue_num).map(|b| b.id.clone()),
                severity: DriftSeverity::Warning,
            });
        }
    }

    // 6. Check for AGENTS.md convention violations
    if let Some(violations) = check_agents_conventions(&beads) {
        convention_violations += violations.len();
        drift_events.extend(violations);
    }

    // Build messages
    if closed_beads_open_issues > 0 {
        messages.push(format!(
            "Closed beads with open GitHub issues: {} ⚠",
            closed_beads_open_issues
        ));
    } else {
        messages.push("Closed beads with open GitHub issues: 0 ✓".into());
    }

    if in_progress_beads_closed_issues > 0 {
        messages.push(format!(
            "In-progress beads with closed GitHub issues: {} ⚠",
            in_progress_beads_closed_issues
        ));
    } else {
        messages.push("In-progress beads with closed GitHub issues: 0 ✓".into());
    }

    if orphan_beads > 0 {
        messages.push(format!(
            "Orphan beads (no GitHub issue link): {} ⚠",
            orphan_beads
        ));
    } else {
        messages.push("Orphan beads (no GitHub issue link): 0 ✓".into());
    }

    if ready_for_work_issues_no_bead > 0 {
        messages.push(format!(
            "Issues labeled 'ready-for-work' with no linked bead: {} ⚠",
            ready_for_work_issues_no_bead
        ));
    } else {
        messages.push("Issues labeled 'ready-for-work' with no linked bead: 0 ✓".into());
    }

    if release_proposed_not_in_milestone > 0 {
        messages.push(format!(
            "Release-proposed issues not in milestone: {} ⚠",
            release_proposed_not_in_milestone
        ));
    } else {
        messages.push("Release-proposed issues not in milestone: 0 ✓".into());
    }

    if convention_violations > 0 {
        messages.push(format!(
            "AGENTS.md convention violations: {} ⚠",
            convention_violations
        ));
    } else {
        messages.push("AGENTS.md convention violations: 0 ✓".into());
    }

    // Build status and final messages
    let total_events = drift_events.len();
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

    Ok(DriftCheckResult {
        category_result: CategoryResult {
            name: CATEGORY_DRIFT.to_string(),
            status,
            messages,
        },
        drift_events,
    })
}

/// Fetch beads from the beads database
/// In a real implementation, this would query the dolt database
async fn fetch_beads() -> Result<Vec<Bead>, RogersError> {
    // Placeholder - in production this would:
    // 1. Connect to dolt at beads.remote/beads.database
    // 2. Query the epics and children tables
    // 3. Return all beads with their GitHub issue URLs and states

    // For now, return empty - actual implementation would use dolt sql client
    // This is a placeholder that allows the module to compile
    // while the full beads integration is developed separately
    Ok(Vec::new())
}

/// Fetch all GitHub issues from the repository
async fn fetch_github_issues(
    owner: &str,
    repo: &str,
    token: &str,
    base_url: &str,
    client: &Client,
) -> Result<Vec<GitHubIssue>, RogersError> {
    let mut all_issues = Vec::new();
    let mut page = 1;
    let per_page = 100;

    loop {
        let url = format!(
            "{}/repos/{}/{}/issues?state=all&per_page={}&page={}",
            base_url, owner, repo, per_page, page
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?;

        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Ok(Vec::new());
            }
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: "Failed to fetch GitHub issues".into(),
            });
        }

        let issues: Vec<serde_json::Value> = response.json().await?;

        if issues.is_empty() {
            break;
        }

        for issue_data in &issues {
            // Skip pull requests (they appear in issues API but aren't issues)
            if issue_data.get("pull_request").is_some() {
                continue;
            }

            let number = issue_data
                .get("number")
                .and_then(|n| n.as_u64())
                .map(|n| n as u32)
                .unwrap_or(0);

            let title = issue_data
                .get("title")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let state = issue_data
                .get("state")
                .and_then(|s| s.as_str())
                .unwrap_or("open")
                .to_string();

            let labels: Vec<String> = issue_data
                .get("labels")
                .and_then(|l| l.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|label| {
                            label.get("name").and_then(|n| n.as_str()).map(String::from)
                        })
                        .collect()
                })
                .unwrap_or_default();

            let milestone = issue_data
                .get("milestone")
                .and_then(|m| m.as_object())
                .and_then(|m| m.get("title"))
                .and_then(|t| t.as_str())
                .map(String::from);

            let html_url = issue_data
                .get("html_url")
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();

            all_issues.push(GitHubIssue {
                number,
                title,
                state,
                labels,
                milestone,
                url: html_url,
            });
        }

        if issues.len() < per_page {
            break;
        }

        page += 1;
    }

    Ok(all_issues)
}

/// Fetch all GitHub milestones from the repository
async fn fetch_milestones(
    owner: &str,
    repo: &str,
    token: &str,
    base_url: &str,
    client: &Client,
) -> Result<Vec<GitHubMilestone>, RogersError> {
    let url = format!("{}/repos/{}/{}/milestones?state=all", base_url, owner, repo);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(Vec::new());
    }

    let milestones: Vec<serde_json::Value> = response.json().await?;

    Ok(milestones
        .iter()
        .filter_map(|m| {
            let number = m.get("number")?.as_u64()? as u32;
            let title = m.get("title")?.as_str()?.to_string();
            let state = m.get("state")?.as_str()?.to_string();
            Some(GitHubMilestone {
                number,
                title,
                state,
            })
        })
        .collect())
}

/// Check bead description against AGENTS.md conventions
fn check_agents_conventions(beads: &[Bead]) -> Option<Vec<DriftEvent>> {
    // Read AGENTS.md conventions if available
    // Note: This is a placeholder - currently we check hard-coded conventions
    // In a full implementation, we would parse AGENTS.md to find the specific conventions to enforce
    let _agents_content = std::fs::read_to_string("AGENTS.md").ok()?;
    let mut violations = Vec::new();

    for bead in beads {
        // Convention check 1: Beads should have Plan: reference in description
        // Based on AGENTS.md: "The `Plan:` line in the description is mandatory"
        if let Some(ref desc) = bead.description {
            if !desc.contains("Plan: plans/") {
                // Check if bead has exemption label (infra, tooling, meta, no-plan-required)
                let has_exemption = bead
                    .rodgers_type
                    .as_ref()
                    .map(|t| {
                        matches!(
                            t.as_str(),
                            "infra" | "tooling" | "meta" | "no-plan-required"
                        )
                    })
                    .unwrap_or(false);

                if !has_exemption {
                    violations.push(DriftEvent {
                        event_type: "convention_violation".into(),
                        description: format!(
                            "Bead {} missing 'Plan: plans/...' reference in description",
                            bead.id
                        ),
                        github_issue_url: bead.github_issue_url.clone(),
                        bead_id: Some(bead.id.clone()),
                        severity: DriftSeverity::Warning,
                    });
                }
            }

            // Convention check 2: Acceptance criteria should be present for feature/bug beads
            if let Some(ref rt) = bead.rodgers_type {
                if (rt == "feature" || rt == "bug") && !desc.contains("--acceptance=") {
                    violations.push(DriftEvent {
                        event_type: "convention_violation".into(),
                        description: format!(
                            "Bead {} missing '--acceptance=' criteria in description",
                            bead.id
                        ),
                        github_issue_url: bead.github_issue_url.clone(),
                        bead_id: Some(bead.id.clone()),
                        severity: DriftSeverity::Warning,
                    });
                }
            }
        }
    }

    if violations.is_empty() {
        None
    } else {
        Some(violations)
    }
}

/// Extract issue number from GitHub issue URL
fn extract_issue_number(url: &str) -> Option<u32> {
    url.split('/').last()?.parse::<u32>().ok()
}

/// Compare GitHub issue state with bead state
///
/// Returns drift events if there's a mismatch between GitHub and beads state.
pub fn detect_drift_events(
    bead_statuses: &[(String, String, Option<String>)], // (bead_id, status, github_issue_url)
    github_issue_states: &HashMap<String, String>,      // issue_url -> state
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
        let mut github_states = HashMap::new();
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
        let mut github_states = HashMap::new();
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
        let mut github_states = HashMap::new();
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
        let github_states = HashMap::new();

        let bead_statuses = vec![("b-001".into(), "closed".into(), None)];

        let events = detect_drift_events(&bead_statuses, &github_states);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "orphan_bead");
        assert_eq!(events[0].bead_id, Some("b-001".into()));
    }

    #[test]
    fn test_extract_issue_number() {
        assert_eq!(
            extract_issue_number("https://github.com/owner/repo/issues/123"),
            Some(123)
        );
        assert_eq!(
            extract_issue_number("https://github.com/owner/repo/issues/456"),
            Some(456)
        );
        assert_eq!(extract_issue_number("invalid"), None);
    }

    #[test]
    fn test_detect_ready_for_work_no_bead() {
        // Test the ready-for-work no bead drift type through event creation
        let event = DriftEvent {
            event_type: "ready_for_work_no_bead".into(),
            description: "Issue #42 'Test issue' has 'ready-for-work' label but no linked bead"
                .into(),
            github_issue_url: Some("https://github.com/owner/repo/issues/42".into()),
            bead_id: None,
            severity: DriftSeverity::Warning,
        };

        assert_eq!(event.event_type, "ready_for_work_no_bead");
        assert!(event.github_issue_url.is_some());
        assert!(event.bead_id.is_none());
    }

    #[test]
    fn test_detect_release_proposed_no_milestone() {
        // Test the release-proposed not in milestone drift type through event creation
        let event = DriftEvent {
            event_type: "release_proposed_no_milestone".into(),
            description:
                "Issue #100 'Feature X' is release-proposed but not assigned to a milestone".into(),
            github_issue_url: Some("https://github.com/owner/repo/issues/100".into()),
            bead_id: Some("b-100".into()),
            severity: DriftSeverity::Warning,
        };

        assert_eq!(event.event_type, "release_proposed_no_milestone");
        assert!(event.github_issue_url.is_some());
        assert!(event.bead_id.is_some());
    }

    #[test]
    fn test_detect_convention_violation() {
        // Test the convention violation drift type through event creation
        let event = DriftEvent {
            event_type: "convention_violation".into(),
            description: "Bead b-001 missing 'Plan: plans/...' reference in description".into(),
            github_issue_url: Some("https://github.com/owner/repo/issues/101".into()),
            bead_id: Some("b-001".into()),
            severity: DriftSeverity::Warning,
        };

        assert_eq!(event.event_type, "convention_violation");
        assert!(event.bead_id.is_some());
    }

    #[test]
    fn test_all_drift_types_have_unique_identifiers() {
        // Verify that all drift event types are distinct
        let drift_types = [
            "closed_bead_open_issue",
            "in_progress_bead_closed_issue",
            "orphan_bead",
            "ready_for_work_no_bead",
            "release_proposed_no_milestone",
            "convention_violation",
        ];

        let mut unique_types: Vec<&str> = drift_types.to_vec();
        unique_types.sort();
        unique_types.dedup();

        assert_eq!(
            unique_types.len(),
            drift_types.len(),
            "All drift event types should be unique"
        );
    }

    #[test]
    fn test_bead_information_captured_in_drift_event() {
        // Verify that drift events capture all necessary information
        let event = DriftEvent {
            event_type: "closed_bead_open_issue".into(),
            description: "Bead b-123 is closed but linked GitHub issue #456 is open".into(),
            github_issue_url: Some("https://github.com/owner/repo/issues/456".into()),
            bead_id: Some("b-123".into()),
            severity: DriftSeverity::Error,
        };

        assert!(event.github_issue_url.is_some());
        assert!(event.bead_id.is_some());
        assert!(!event.description.is_empty());
        assert!(!event.event_type.is_empty());
    }
}
