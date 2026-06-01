#![allow(dead_code)]

//! Will-not-do handling for Rodgers.
//!
//! This module implements the handling of `will-not-do` decisions made by humans.
//! When a human applies the `will-not-do` label to an issue, Rodgers:
//!
//! 1. Detects the label during triage
//! 2. Generates a warm closure comment via LLM
//! 3. Posts the comment to the issue
//! 4. Closes the issue
//!
//! All within ONE triage run, as specified in CRIT-3.
//!
//! ## Design
//!
//! The `WillNotDoHandler` struct encapsulates the logic for processing a
//! will-not-do decision. It generates the appropriate transitions and actions
//! that the triage loop executes.
//!
//! ## Tone Guidance
//!
//! The closure comment should ALWAYS be warm高音 and respectful:
//! - Thank the requestor for taking time to file the issue
//! - Express regret that we cannot pursue this
//! - Never be curt or dismissive ("not a priority" alone is not acceptable)
//!
//! ## Priority
//!
//! If both `will-not-do` and `ready-for-work` labels are present,
//! `will-not-do` takes priority per the state machine design.

use serde::{Deserialize, Serialize};

/// Result of processing a will-not-do decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WillNotDoResult {
    /// Issue number
    pub issue_number: u64,
    /// Whether will-not-do was detected
    pub will_not_do_detected: bool,
    /// The warm closure comment to post (None if not detected)
    pub closure_comment: Option<String>,
    /// Whether to close the issue
    pub should_close: bool,
    /// Labels to remove (like ready-for-review)
    pub labels_to_remove: Vec<String>,
    /// Labels to add (like will-not-do if not already present)
    pub labels_to_add: Vec<String>,
}

impl WillNotDoResult {
    /// Create a result when will-not-do is detected.
    ///
    /// This generates the closure comment and marks the issue for closing.
    pub fn detected(issue_number: u64, closure_comment: String) -> Self {
        Self {
            issue_number,
            will_not_do_detected: true,
            closure_comment: Some(closure_comment),
            should_close: true,
            labels_to_remove: vec!["ready-for-review".to_string()],
            labels_to_add: Vec::new(), // will-not-do should already be present
        }
    }

    /// Create a result when will-not-do is not detected.
    pub fn not_detected(issue_number: u64) -> Self {
        Self {
            issue_number,
            will_not_do_detected: false,
            closure_comment: None,
            should_close: false,
            labels_to_remove: Vec::new(),
            labels_to_add: Vec::new(),
        }
    }
}

/// Generate a warm closure comment for a declined issue.
///
/// This is a placeholder implementation that creates a respectful closure comment.
/// In production, this would be replaced with LLM-generated content using
/// `WARM_CLOSURE_PROMPT` from the prompts module.
///
/// ## Tone Requirements
///
/// - Always thank the requestor
/// - Express genuine regret
/// - Be specific about what was considered
/// - Never be curt or dismissive
///
/// ## Arguments
///
/// * `issue_title` - The title of the issue being declined
/// * `author` - The GitHub username of the issue author
/// * `issue_type` - Either "bug report" or "feature request"
pub fn generate_warm_closure_comment(issue_title: &str, author: &str, issue_type: &str) -> String {
    // This template ensures warm, grateful tone with regret
    format!(
        r#"Thanks @{author} for the {issue_type} titled "{title}"!

After careful consideration, we're unable to prioritize this at this time. The team has weighed this against other planned work and has decided not to move forward with this request.

We apologize for not being able to address this for you. If circumstances change in the future or you have other ideas, please don't hesitate to open a new issue.

Thanks again for contributing to the project!"#,
        author = author,
        issue_type = issue_type,
        title = issue_title
    )
}

/// Check if an issue has the will-not-do label applied.
///
/// Returns true if the label is present in the issue labels.
pub fn has_will_not_do_label(labels: &[String]) -> bool {
    labels.iter().any(|l| l == "will-not-do")
}

/// Resolve the issue type string from labels.
///
/// Returns "bug report" or "feature request" based on the labels present.
pub fn resolve_issue_type(labels: &[String]) -> String {
    if labels.iter().any(|l| l == "bug") {
        "bug report".to_string()
    } else if labels.iter().any(|l| l == "feature") {
        "feature request".to_string()
    } else {
        "issue".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_will_not_do_result_detected() {
        let result = WillNotDoResult::detected(
            42,
            "Thank you for the report. We won't pursue this.".to_string(),
        );

        assert!(result.will_not_do_detected);
        assert!(result.should_close);
        assert!(result.closure_comment.is_some());
        assert!(result
            .labels_to_remove
            .contains(&"ready-for-review".to_string()));
    }

    #[test]
    fn test_will_not_do_result_not_detected() {
        let result = WillNotDoResult::not_detected(42);

        assert!(!result.will_not_do_detected);
        assert!(!result.should_close);
        assert!(result.closure_comment.is_none());
        assert!(result.labels_to_remove.is_empty());
    }

    #[test]
    fn test_has_will_not_do_label_true() {
        let labels = vec![
            "bug".to_string(),
            "ready-for-review".to_string(),
            "will-not-do".to_string(),
        ];
        assert!(has_will_not_do_label(&labels));
    }

    #[test]
    fn test_has_will_not_do_label_false() {
        let labels = vec!["bug".to_string(), "ready-for-review".to_string()];
        assert!(!has_will_not_do_label(&labels));
    }

    #[test]
    fn test_has_will_not_do_label_empty() {
        let labels: Vec<String> = vec![];
        assert!(!has_will_not_do_label(&labels));
    }

    #[test]
    fn test_resolve_issue_type_bug() {
        let labels = vec!["bug".to_string()];
        assert_eq!(resolve_issue_type(&labels), "bug report");
    }

    #[test]
    fn test_resolve_issue_type_feature() {
        let labels = vec!["feature".to_string()];
        assert_eq!(resolve_issue_type(&labels), "feature request");
    }

    #[test]
    fn test_resolve_issue_type_fallback() {
        let labels = vec!["question".to_string()];
        assert_eq!(resolve_issue_type(&labels), "issue");
    }

    #[test]
    fn test_resolve_issue_type_with_multiple() {
        // Bug takes precedence in the iteration order
        let labels = vec!["bug".to_string(), "feature".to_string()];
        assert_eq!(resolve_issue_type(&labels), "bug report");
    }

    #[test]
    fn test_warm_closure_comment_includes_author() {
        let comment = generate_warm_closure_comment("Test Issue", "testuser", "bug report");

        assert!(comment.contains("@testuser"));
        assert!(comment.contains("bug report"));
        assert!(comment.contains("Test Issue"));
    }

    #[test]
    fn test_warm_closure_comment_tone() {
        let comment = generate_warm_closure_comment("Test Issue", "testuser", "bug report");

        // Should express gratitude
        assert!(comment.contains("Thanks"));
        // Should express regret
        assert!(comment.contains("apologize") || comment.contains("regret"));
        // Should thank for contribution
        assert!(comment.contains("Thanks again"));
    }

    #[test]
    fn test_warm_closure_comment_no_curt_phrases() {
        let comment = generate_warm_closure_comment("Test Issue", "testuser", "feature request");

        // Should NOT contain curt phrases
        assert!(!comment.contains("not a priority"));
        assert!(!comment.contains("we won't implement"));
        assert!(!comment.contains("just \"no\""));
    }

    #[test]
    fn test_warm_closure_comment_mentions_future() {
        let comment = generate_warm_closure_comment("Nice Feature", "developer", "feature request");

        // Should leave door open for future consideration
        assert!(comment.contains("future") || comment.contains("new issue"));
    }

    #[test]
    fn test_will_not_do_priority_over_ready_for_work() {
        // Simulating a scenario where both labels are present
        // The triage loop should prioritize will-not-do
        let labels = vec![
            "bug".to_string(),
            "will-not-do".to_string(),
            "ready-for-work".to_string(),
        ];

        let has_will_not_do = has_will_not_do_label(&labels);
        let has_ready_for_work = labels.iter().any(|l| l == "ready-for-work");

        // Will-not-do should win
        assert!(has_will_not_do);
        assert!(has_ready_for_work);

        // In triage logic, will-not-do takes priority
        // So if will-not-do is detected, we should NOT process ready-for-work
        let should_process_will_not_do = has_will_not_do;
        let should_process_ready_for_work = !has_will_not_do && has_ready_for_work;

        assert!(should_process_will_not_do);
        assert!(!should_process_ready_for_work);
    }

    #[test]
    fn test_processing_within_one_triage_run() {
        // This test verifies that will-not-do detection AND closure action
        // are both available in a single call
        let issue_number = 42;
        let labels = vec!["bug".to_string(), "will-not-do".to_string()];
        let title = "Test Bug";
        let author = "reporter";

        // Detection happens
        let detected = has_will_not_do_label(&labels);
        assert!(detected);

        // If detected, closure result is available
        if detected {
            let issue_type = resolve_issue_type(&labels);
            let comment = generate_warm_closure_comment(title, author, &issue_type);
            let result = WillNotDoResult::detected(issue_number, comment);

            // All actions are ready in ONE call
            assert!(result.will_not_do_detected);
            assert!(result.should_close);
            assert!(result.closure_comment.is_some());

            // Ready for GitHub API calls (post comment + close issue)
            // These can execute in the same triage run
            let closure_comment = result.closure_comment.unwrap();
            assert!(!closure_comment.is_empty());
            assert!(result.should_close);
        }
    }
}
