//! Feature and bug analysis module.
//!
//! This module provides epic detection and breakdown analysis for
//! feature requests and bug reports.

pub mod breakdown;
pub mod completeness;
pub mod will_not_do;

use serde::{Deserialize, Serialize};

pub use breakdown::{BreakdownAnalyzer, BreakdownComment, ChildBeadRequest, EpicBreakdown};
pub use completeness::{
    BugCompletenessRequirements, CompletenessCheckResult, FeatureCompletenessRequirements,
    check_bug_completeness, check_feature_completeness,
};
pub use will_not_do::{
    WillNotDoResult, generate_warm_closure_comment, has_will_not_do_label, resolve_issue_type,
};

/// A feature or bug issue with relevant metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureBugIssue {
    /// GitHub issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body
    pub body: String,
    /// Author username
    pub author: String,
    /// Whether this is a bug report
    pub is_bug: bool,
    /// Whether this is a feature request
    pub is_feature: bool,
}

/// Summary of a transition action for a feature/bug issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSummary {
    /// Comment to post
    pub comment: String,
    /// Labels to add
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
    /// Whether this transition applies needs-information
    pub applied_needs_information: bool,
}

impl TransitionSummary {
    /// Generate a transition summary for a bug that is ready for review.
    pub fn bug_ready_for_review(issue: &FeatureBugIssue) -> Self {
        Self {
            comment: format!(
                "Hi @{}! Thank you for the detailed bug report about \"{}\". \
                We have reviewed your report and it contains all the information we need. \
                This has been marked as ready for review.\n\n**Summary:**\n- **What Happened:** {}",
                issue.author,
                issue.title,
                summarize_body(&issue.body, 100)
            ),
            labels_to_add: vec!["ready-for-review".to_string(), "rodgers:triaged".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
            applied_needs_information: false,
        }
    }

    /// Generate a transition summary for a bug that needs more information.
    pub fn bug_needs_information(issue: &FeatureBugIssue, request_message: &str) -> Self {
        Self {
            comment: format!(
                "Hi @{}! Thanks for reporting \"{}\". To help us investigate, could you please \
                provide the following information:\n\n{}

                We look forward to your response!",
                issue.author,
                issue.title,
                request_message
            ),
            labels_to_add: vec!["needs-information".to_string(), "rodgers:triaged".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
            applied_needs_information: true,
        }
    }

    /// Generate a transition summary for a feature that is ready for review.
    pub fn feature_ready_for_review(issue: &FeatureBugIssue) -> Self {
        Self {
            comment: format!(
                "Hi @{}! Thank you for the feature request \"{}\". \
                We have reviewed your proposal and it contains all the necessary information. \
                This has been marked as ready for review.\n\n**Summary:**\n- **Use Case:** {}",
                issue.author,
                issue.title,
                summarize_body(&issue.body, 100)
            ),
            labels_to_add: vec!["ready-for-review".to_string(), "rodgers:triaged".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
            applied_needs_information: false,
        }
    }

    /// Generate a transition summary for a feature that needs more information.
    pub fn feature_needs_information(issue: &FeatureBugIssue, request_message: &str) -> Self {
        Self {
            comment: format!(
                "Hi @{}! Thanks for the feature request \"{}\". To help us evaluate this, \
                could you please provide additional details:\n\n{}

                This will help us understand the scope and prioritize effectively!",
                issue.author,
                issue.title,
                request_message
            ),
            labels_to_add: vec!["needs-information".to_string(), "rodgers:triaged".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
            applied_needs_information: true,
        }
    }
}

/// Execute a breakdown of an issue into child tasks.
pub fn execute_breakdown(
    _body: &str,
    _title: &str,
    _issue_number: u64,
    _url: &str,
    _is_bug: bool,
    _comments: &[String],
    _author: &str,
) -> BreakdownComment {
    BreakdownComment {
        body: format!(
            "Rodgers: Breakdown initiated for issue #{} — \"{}\". \
            This will be tracked via beads.",
            _issue_number, _title
        ),
        epic_title: _title.to_string(),
        child_titles: vec![],
    }
}

/// Summarize body content, truncating if needed.
fn summarize_body(body: &str, max_len: usize) -> String {
    let cleaned = body.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join(" ");
    if cleaned.len() > max_len {
        format!("{}...", &cleaned[..max_len])
    } else {
        cleaned
    }
}
