//! Triage loop for bug report completeness check.
//!
//! This module implements the triage loop step for bug report completeness:
//! - When a bug report has all required fields populated → apply `ready-for-review`
//! - When any field is missing → apply `needs-information` for that specific field
//! - NO additional info requests when complete
//!
//! ## Integration with State Machine
//!
//! Per the triage workflow plan (plans/triage-workflow-plan.md):
//! - BUG_INCOMPLETE → READY_FOR_REVIEW: all required info present
//! - BUG_INCOMPLETE → NEEDS_INFO: post comment, label needs-information
//!
//! This module handles the transition logic.

use serde::{Deserialize, Serialize};

use crate::feature_bug::completeness::{check_bug_completeness, check_bug_completeness_semantic};
use crate::github::IssueUpdate;
use crate::templates::mapping::CanonicalField;

/// Actions to take after triage determines completeness.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TriageAction {
    /// Apply ready-for-review — all fields present and populated.
    /// No needs-information comment. No additional info requests.
    ReadyForReview {
        /// Summary comment to post when transitioning to ready-for-review.
        comment: String,
    },
    /// Apply needs-information — some fields are missing.
    /// Posts a comment requesting only the missing fields.
    NeedsInformation {
        /// The specific missing fields.
        missing_fields: Vec<CanonicalField>,
        /// Comment to post requesting the missing information.
        comment: String,
        /// Labels to add (includes needs-information).
        labels: Vec<String>,
    },
    /// No action needed — issue already has a status label.
    NoAction,
}

impl TriageAction {
    /// Generate an IssueUpdate for this action.
    pub fn to_issue_update(&self) -> Option<IssueUpdate> {
        match self {
            TriageAction::ReadyForReview { .. } => Some(IssueUpdate::with_labels(vec![
                "ready-for-review".to_string(),
            ])),
            TriageAction::NeedsInformation { labels, .. } => {
                Some(IssueUpdate::with_labels(labels.clone()))
            }
            TriageAction::NoAction => None,
        }
    }

    /// Get the comment body for this action, if any.
    pub fn to_comment(&self) -> Option<&str> {
        match self {
            TriageAction::ReadyForReview { comment } => Some(comment),
            TriageAction::NeedsInformation { comment, .. } => Some(comment),
            TriageAction::NoAction => None,
        }
    }
}

/// Process a bug report issue through the completeness triage.
///
/// This is the main entry point for the triage loop. It checks completeness,
/// determines the appropriate action, and returns the action to take.
///
/// # Arguments
///
/// * `body` - The issue body text
/// * `current_labels` - The current labels on the issue
/// * `semantic_mapping` - Whether to use semantic field mapping (for custom templates)
///
/// # Returns
///
/// A `TriageAction` indicating what to do next.
pub fn triage_bug_completeness(
    body: &str,
    current_labels: &[String],
    semantic_mapping: bool,
) -> TriageAction {
    // Check if issue already has a status label (skip re-triage)
    let status_labels = [
        "ready-for-review",
        "will-not-do",
        "ready-for-work",
        "in-progress",
    ];
    if current_labels
        .iter()
        .any(|l| status_labels.contains(&l.as_str()))
    {
        return TriageAction::NoAction;
    }

    // Check if issue already has needs-information
    if current_labels.iter().any(|l| l == "needs-information") {
        return TriageAction::NoAction;
    }

    // Perform completeness check
    let result = if semantic_mapping {
        check_bug_completeness_semantic(body)
    } else {
        check_bug_completeness(body)
    };

    if result.is_complete {
        // All fields present — transition to ready-for-review
        TriageAction::ReadyForReview {
            comment: "This bug report appears to have all the information we need. We'll review it shortly. In the meantime, no further action is needed from you.".to_string(),
        }
    } else {
        // Missing fields — request only what's needed
        let labels = vec!["needs-information".to_string()];
        let comment = result.to_request_comment().unwrap_or_default();

        TriageAction::NeedsInformation {
            missing_fields: result.missing_fields,
            comment,
            labels,
        }
    }
}

/// Check if a bug report is complete and should transition to ready-for-review.
///
/// This is a convenience function for simple completeness checking.
/// Returns `true` if the bug is complete and ready for review.
///
/// # Arguments
///
/// * `body` - The issue body text
///
/// # Returns
///
/// `true` if the bug is complete
pub fn is_bug_ready_for_review(body: &str) -> bool {
    check_bug_completeness(body).is_complete
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Ready-For-Review Tests (CRIT-7 primary acceptance criterion) ===

    #[test]
    fn test_triage_complete_bug_returns_ready_for_review() {
        let body = r#"
## Bug Summary
Network timeout

## Environment
- OS: Ubuntu 22.04
- Version: 1.0.0

## Steps to Reproduce
1. Open the app
2. Wait for network request

## Expected Behavior
Request should succeed

## Actual Behavior
Request times out after 30s
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        match action {
            TriageAction::ReadyForReview { ref comment } => {
                assert!(comment.contains("review"));
            }
            _ => panic!("Expected ReadyForReview action"),
        }
    }

    #[test]
    fn test_ready_for_review_no_needs_information_comment() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        match action {
            TriageAction::ReadyForReview { .. } => {
                // No needs-information comment — verified by action type
            }
            _ => panic!("Expected ReadyForReview, not {:?}", action),
        }
    }

    #[test]
    fn test_ready_for_review_no_additional_info_requests() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        match action {
            TriageAction::ReadyForReview { .. } => {
                // No missing fields → no additional info requests
            }
            TriageAction::NeedsInformation {
                ref missing_fields, ..
            } => {
                panic!(
                    "Should NOT be NeedsInformation, got {:?} missing fields",
                    missing_fields
                );
            }
            _ => panic!("Expected ReadyForReview action"),
        }
    }

    // === Needs-Information Tests ===

    #[test]
    fn test_triage_incomplete_bug_returns_needs_information() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        match action {
            TriageAction::NeedsInformation {
                missing_fields,
                comment,
                labels,
            } => {
                assert_eq!(labels, vec!["needs-information"]);
                assert!(!missing_fields.is_empty());
                assert!(!comment.is_empty());
            }
            _ => panic!("Expected NeedsInformation action"),
        }
    }

    #[test]
    fn test_needs_information_requests_only_missing_field() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        // Should be ReadyForReview because all fields are present
        match action {
            TriageAction::ReadyForReview { .. } => {}
            _ => panic!("Expected ReadyForReview because all fields present"),
        }
    }

    #[test]
    fn test_needs_information_missing_environment() {
        let body = r#"
## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        match action {
            TriageAction::NeedsInformation { missing_fields, .. } => {
                assert!(missing_fields.contains(&CanonicalField::Environment));
            }
            _ => panic!("Expected NeedsInformation action"),
        }
    }

    // === No-Action Tests ===

    #[test]
    fn test_no_action_when_already_ready_for_review() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let action = triage_bug_completeness(
            body,
            &["bug".to_string(), "ready-for-review".to_string()],
            false,
        );

        match action {
            TriageAction::NoAction => {}
            _ => panic!("Expected NoAction when already ready-for-review"),
        }
    }

    #[test]
    fn test_no_action_when_already_needs_information() {
        let body = r#"
## Bug Summary
Something broken
"#;

        let action = triage_bug_completeness(
            body,
            &["bug".to_string(), "needs-information".to_string()],
            false,
        );

        match action {
            TriageAction::NoAction => {}
            _ => panic!("Expected NoAction when already needs-information"),
        }
    }

    #[test]
    fn test_no_action_when_already_ready_for_work() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let action = triage_bug_completeness(body, &["ready-for-work".to_string()], false);

        match action {
            TriageAction::NoAction => {}
            _ => panic!("Expected NoAction when already ready-for-work"),
        }
    }

    #[test]
    fn test_issue_update_ready_for_review() {
        let action = TriageAction::ReadyForReview {
            comment: "test".to_string(),
        };
        let update = action.to_issue_update();
        assert!(update.is_some());
    }

    #[test]
    fn test_issue_update_needs_information() {
        let action = TriageAction::NeedsInformation {
            missing_fields: vec![CanonicalField::Environment],
            comment: "test".to_string(),
            labels: vec!["needs-information".to_string()],
        };
        let update = action.to_issue_update();
        assert!(update.is_some());
    }

    #[test]
    fn test_issue_update_no_action() {
        let action = TriageAction::NoAction;
        assert!(action.to_issue_update().is_none());
    }

    #[test]
    fn test_is_bug_ready_for_review_complete() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        assert!(is_bug_ready_for_review(body));
    }

    #[test]
    fn test_is_bug_ready_for_review_incomplete() {
        let body = r#"
## Bug Summary
Something broken
"#;

        assert!(!is_bug_ready_for_review(body));
    }

    // === Semantic Mapping Tests ===

    #[test]
    fn test_triage_semantic_mapping_custom_fields() {
        let body = r#"
## System
- OS: macOS

## How to Reproduce
1. Open app

## Expected
Should work

## What Happened
Crashes
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], true);

        match action {
            TriageAction::ReadyForReview { .. } => {}
            _ => panic!("Expected ReadyForReview with semantic mapping"),
        }
    }

    #[test]
    fn test_triage_non_semantic_custom_fields_missing() {
        let body = r#"
## System
- OS: macOS

## How to Reproduce
1. Open app

## Expected
Should work

## What Happened
Crashes
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        // Without semantic mapping, these custom field names won't match
        // So all standard fields will be missing
        match action {
            TriageAction::NeedsInformation { .. } => {}
            _ => panic!("Expected NeedsInformation without semantic mapping"),
        }
    }

    // === Comment Generation Tests ===

    #[test]
    fn test_triage_action_comment_for_ready_for_review() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        match action {
            TriageAction::ReadyForReview { ref comment } => {
                assert!(comment.contains("review"));
                assert!(action.to_comment().is_some());
            }
            _ => panic!("Expected ReadyForReview"),
        }
    }

    #[test]
    fn test_triage_action_comment_for_needs_information() {
        let body = r#"
## Bug Summary
Something broken
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        match action {
            TriageAction::NeedsInformation { ref comment, .. } => {
                assert!(!comment.is_empty());
                assert!(action.to_comment().is_some());
            }
            _ => panic!("Expected NeedsInformation"),
        }
    }

    #[test]
    fn test_triage_action_no_comment_for_no_action() {
        let action = TriageAction::NoAction;
        assert!(action.to_comment().is_none());
    }

    // === Integration: Template-filed bug → ready-for-review ===

    #[test]
    fn test_template_filed_bug_transitions_to_ready_for_review() {
        // Simulates a bug filed via bug_report template with all fields
        let body = r#"
## Bug Summary
Dashboard loads slowly

## Environment
- OS: Windows 11
- Version: 3.2.1
- Browser: Chrome 120

## Steps to Reproduce
1. Log in to the dashboard
2. Wait for data to load
3. Observe load time

## Expected Behavior
Dashboard should load in under 2 seconds

## Actual Behavior
Dashboard takes 15+ seconds to load
"#;

        let action = triage_bug_completeness(body, &["bug".to_string()], false);

        // Should transition to ready-for-review
        match action {
            TriageAction::ReadyForReview { .. } => {
                // SUCCESS: transitions to ready-for-review
            }
            TriageAction::NeedsInformation {
                ref missing_fields, ..
            } => {
                panic!(
                    "FAILED: Bug with all 4 fields should be ready-for-review, but got NeedsInformation for: {:?}",
                    missing_fields
                );
            }
            TriageAction::NoAction => {
                panic!("FAILED: Expected action, got NoAction");
            }
        }
    }
}
