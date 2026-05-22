//! Feature bug issue and transition types.
//!
//! Defines types used by the triage loop to coordinate between
//! completeness checking and label transitions.

use serde::{Deserialize, Serialize};

/// Represents a bug or feature issue for transition processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureBugIssue {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author: String,
    pub is_bug: bool,
    pub is_feature: bool,
}

/// Summary of a triage transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSummary {
    pub comment: String,
    pub labels_to_add: Vec<String>,
    pub labels_to_remove: Vec<String>,
    pub applied_needs_information: bool,
}

impl TransitionSummary {
    /// Generate a transition summary for a bug ready for review.
    pub fn bug_ready_for_review(issue: &FeatureBugIssue) -> Self {
        Self {
            comment: format!(
                "Thank you @{} for the detailed bug report \"{}\"! This has been marked ready for review by the team.",
                issue.author, issue.title
            ),
            labels_to_add: vec!["ready-for-review".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
            applied_needs_information: false,
        }
    }

    /// Generate a transition summary for a feature ready for review.
    pub fn feature_ready_for_review(issue: &FeatureBugIssue) -> Self {
        Self {
            comment: format!(
                "Thank you @{} for the detailed feature request \"{}\"! This has been marked ready for review by the team.",
                issue.author, issue.title
            ),
            labels_to_add: vec!["ready-for-review".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
            applied_needs_information: false,
        }
    }

    /// Generate a transition summary requesting information for a bug.
    pub fn bug_needs_information(issue: &FeatureBugIssue, request_message: &str) -> Self {
        Self {
            comment: format!(
                "Hi @{}, thanks for reporting this bug! {}\n\nIssue #{}: {}",
                issue.author, request_message, issue.number, issue.title
            ),
            labels_to_add: vec!["needs-information".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
            applied_needs_information: true,
        }
    }

    /// Generate a transition summary requesting information for a feature.
    pub fn feature_needs_information(issue: &FeatureBugIssue, request_message: &str) -> Self {
        Self {
            comment: format!(
                "Hi @{}, thanks for the feature request! {}\n\nIssue #{}: {}",
                issue.author, request_message, issue.number, issue.title
            ),
            labels_to_add: vec!["needs-information".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
            applied_needs_information: true,
        }
    }
}

/// Execute a breakdown for ready-for-work issues.
/// Returns a simplified breakdown result.
pub struct BreakdownResult {
    pub breakdown_comment: String,
}

/// Execute a simplified breakdown (non-LLM version).
///
/// For the initial implementation, this generates a deterministic
/// breakdown comment without LLM analysis.
pub fn execute_breakdown(
    _body: &str,
    title: &str,
    _number: u64,
    _url: &str,
    _is_bug: bool,
    _comments: &[&str],
    author: &str,
) -> BreakdownResult {
    let breakdown_comment = format!(
        "## Rodgers Breakdown\n\nHi @{}, the issue \"{}\" has been marked ready for work. Rodgers will track the implementation work.\n\nThank you!",
        author, title
    );

    BreakdownResult {
        breakdown_comment,
    }
}
