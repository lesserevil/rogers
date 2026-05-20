//! Feature and bug issue handling for Rodgers.
//!
//! This module implements the transition logic for bug and feature issues
//! as defined in plans/feature-bug-plan.md. It handles:
//!
//! - Completeness verification (see `completeness` module)
//! - Will-not-do handling (see `will_not_do` module)
//! - Transition to ready-for-review when complete
//! - Application of needs-information when incomplete
//! - Generating summary comments and acceptance criteria

mod completeness;
pub mod will_not_do;

pub use completeness::{
    CompletenessCheckResult, check_bug_completeness, check_feature_completeness,
};

use serde::{Deserialize, Serialize};

/// Represents a bug or feature issue that needs triaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureBugIssue {
    /// GitHub issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body (description)
    pub body: String,
    /// Author username
    pub author: String,
    /// Whether this is a bug report
    pub is_bug: bool,
    /// Whether this is a feature request
    pub is_feature: bool,
}

/// Represents the summary comment to be posted on the issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSummary {
    /// The comment body to post
    pub comment: String,
    /// Whether ready-for-review was applied
    pub applied_ready_for_review: bool,
    /// Whether needs-information was applied
    pub applied_needs_information: bool,
    /// Labels to add
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
}

impl TransitionSummary {
    /// Create a summary for a bug that is now ready for review.
    pub fn bug_ready_for_review(issue: &FeatureBugIssue) -> Self {
        let comment = generate_bug_summary(issue);
        Self {
            comment,
            applied_ready_for_review: true,
            applied_needs_information: false,
            labels_to_add: vec!["ready-for-review".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
        }
    }

    /// Create a summary for a feature that is now ready for review.
    pub fn feature_ready_for_review(issue: &FeatureBugIssue) -> Self {
        let comment = generate_feature_summary(issue);
        Self {
            comment,
            applied_ready_for_review: true,
            applied_needs_information: false,
            labels_to_add: vec!["ready-for-review".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
        }
    }

    /// Create a summary for an incomplete bug that needs more information.
    pub fn bug_needs_information(issue: &FeatureBugIssue, request: &str) -> Self {
        let comment = generate_needs_information_comment(issue, request);
        Self {
            comment,
            applied_ready_for_review: false,
            applied_needs_information: true,
            labels_to_add: vec!["needs-information".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
        }
    }

    /// Create a summary for an incomplete feature that needs more information.
    pub fn feature_needs_information(issue: &FeatureBugIssue, request: &str) -> Self {
        let comment = generate_needs_information_comment(issue, request);
        Self {
            comment,
            applied_ready_for_review: false,
            applied_needs_information: true,
            labels_to_add: vec!["needs-information".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
        }
    }
}

/// Generate the summary comment for a complete bug report.
fn generate_bug_summary(issue: &FeatureBugIssue) -> String {
    format!(
        r#"## Rodgers Triage Summary

Thank you for the detailed bug report, @{author}! I've reviewed the information provided and everything looks complete.

### Summary
- **Reported issue**: {title}
- **Status**: Ready for human review

I'll now mark this as ready for review. A human maintainer will evaluate this and either:
- Work on a fix if it fits the project priorities
- Or close it with an explanation if it's not something we can address

Thanks again for taking the time to report this! "#,
        author = issue.author,
        title = issue.title
    )
}

/// Generate the summary comment for a complete feature request.
fn generate_feature_summary(issue: &FeatureBugIssue) -> String {
    format!(
        r#"## Rodgers Triage Summary

Thanks for the feature request, @{author}! I've reviewed the information provided and everything looks complete.

### Summary
- **Requested feature**: {title}
- **Status**: Ready for human review

### Rodgers Generated Acceptance Criteria

{criteria}

I'll mark this as ready for review. A human maintainer will evaluate this request and either:
- Accept it for implementation if it aligns with project goals
- Or close it with an explanation if it's not something we can prioritize right now

Thanks for taking the time to share your ideas! "#,
        author = issue.author,
        title = issue.title,
        criteria = generate_acceptance_criteria(issue)
    )
}

/// Generate a preliminary acceptance criteria section.
///
/// This generates draft acceptance criteria from the issue content.
/// A human reviewer may accept, reject, or modify these before marking ready-for-work.
fn generate_acceptance_criteria(issue: &FeatureBugIssue) -> String {
    // This is a placeholder that would be enhanced with LLM-based extraction
    // For now, generate basic criteria based on whether it's a bug or feature
    if issue.is_bug {
        String::from(
            r#"- [ ] AC-1: The reported behavior is verified and understood
- [ ] AC-2: A fix is implemented that resolves the issue
- [ ] AC-3: Existing functionality is not broken by the fix"#,
        )
    } else {
        String::from(
            r#"- [ ] AC-1: The feature is implemented with the proposed behavior
- [ ] AC-2: Existing functionality is not broken
- [ ] AC-3: The feature meets the stated use case"#,
        )
    }
}

/// Generate the needs-information comment for incomplete issues.
fn generate_needs_information_comment(issue: &FeatureBugIssue, request: &str) -> String {
    let issue_type = if issue.is_bug {
        "bug report"
    } else {
        "feature request"
    };

    format!(
        r#"Hi @{author}, thanks for this {issue_type}!

To help us understand and work on this, could you provide a bit more information?

{request}

Thanks for taking the time to fill this out — the more context you provide, the better we can evaluate and address this!"#,
        author = issue.author,
        issue_type = issue_type,
        request = request
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue(is_bug: bool, is_feature: bool) -> FeatureBugIssue {
        FeatureBugIssue {
            number: 42,
            title: "Test Issue".to_string(),
            body: "Test body content".to_string(),
            author: "testuser".to_string(),
            is_bug,
            is_feature,
        }
    }

    #[test]
    fn test_bug_ready_for_review_transition() {
        let issue = create_test_issue(true, false);
        let summary = TransitionSummary::bug_ready_for_review(&issue);

        assert!(summary.applied_ready_for_review);
        assert!(!summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(
            summary
                .labels_to_remove
                .contains(&"needs-information".to_string())
        );
        assert!(summary.comment.contains("@testuser"));
    }

    #[test]
    fn test_feature_ready_for_review_transition() {
        let issue = create_test_issue(false, true);
        let summary = TransitionSummary::feature_ready_for_review(&issue);

        assert!(summary.applied_ready_for_review);
        assert!(!summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(summary.comment.contains("Acceptance Criteria"));
    }

    #[test]
    fn test_bug_needs_information_transition() {
        let issue = create_test_issue(true, false);
        let request = "Please add reproduction steps";
        let summary = TransitionSummary::bug_needs_information(&issue, request);

        assert!(!summary.applied_ready_for_review);
        assert!(summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
        assert!(
            summary
                .labels_to_remove
                .contains(&"ready-for-review".to_string())
        );
        assert!(summary.comment.contains("reproduction steps"));
    }

    #[test]
    fn test_feature_needs_information_transition() {
        let issue = create_test_issue(false, true);
        let request = "Please describe the use case";
        let summary = TransitionSummary::feature_needs_information(&issue, request);

        assert!(!summary.applied_ready_for_review);
        assert!(summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }

    #[test]
    fn test_generated_acceptance_criteria_includes_bug_criteria() {
        let bug_issue = create_test_issue(true, false);
        let criteria = generate_acceptance_criteria(&bug_issue);

        assert!(criteria.contains("AC-1"));
        assert!(criteria.contains("fix"));
    }

    #[test]
    fn test_generated_acceptance_criteria_includes_feature_criteria() {
        let feature_issue = create_test_issue(false, true);
        let criteria = generate_acceptance_criteria(&feature_issue);

        assert!(criteria.contains("AC-1"));
        assert!(criteria.contains("feature"));
    }

    #[test]
    fn test_complete_bug_workflow() {
        use completeness::check_bug_completeness;

        let complete_body = r#"
## Behavior Observed
It crashes when X happens.

## Behavior Expected
It should not crash.

## Reproduction Steps
1. Do X
2. Observe crash

## Environment
macOS 13.0
"#;

        let issue = FeatureBugIssue {
            number: 1,
            title: "Test bug".to_string(),
            body: complete_body.to_string(),
            author: "reporter".to_string(),
            is_bug: true,
            is_feature: false,
        };

        let result = check_bug_completeness(&issue.body);
        assert!(result.is_complete);

        let transition = TransitionSummary::bug_ready_for_review(&issue);
        assert!(transition.applied_ready_for_review);
    }

    #[test]
    fn test_complete_feature_workflow() {
        use completeness::check_feature_completeness;

        let complete_body = r#"
## Use Case
I need this to solve problem X.

## Proposed Behavior
It should do Y.

## Acceptance Criteria
- [ ] It does Y
- [ ] It works well
"#;

        let issue = FeatureBugIssue {
            number: 2,
            title: "Test feature".to_string(),
            body: complete_body.to_string(),
            author: "requester".to_string(),
            is_bug: false,
            is_feature: true,
        };

        let result = check_feature_completeness(&issue.body);
        assert!(result.is_complete);

        let transition = TransitionSummary::feature_ready_for_review(&issue);
        assert!(transition.applied_ready_for_review);
    }

    #[test]
    fn test_incomplete_bug_workflow_in_one_run() {
        use completeness::check_bug_completeness;

        let incomplete_body = r#"
## Behavior Observed
It crashes sometimes.
"#;

        let issue = FeatureBugIssue {
            number: 3,
            title: "Incomplete bug".to_string(),
            body: incomplete_body.to_string(),
            author: "reporter".to_string(),
            is_bug: true,
            is_feature: false,
        };

        let result = check_bug_completeness(&issue.body);
        assert!(!result.is_complete);
        assert!(!result.missing_bug_fields.is_empty());

        let transition = TransitionSummary::bug_needs_information(&issue, &result.request_message);
        assert!(transition.applied_needs_information);
    }

    #[test]
    fn test_no_delay_in_transition() {
        // This test verifies that the transition logic completes within a single run
        // by ensuring all operations are synchronous and deterministic

        let issue = create_test_issue(true, false);
        let complete_body = r#"
## What Happened
Something wrong.

## What Should Happen
Something right.

## Steps
1. Step 1

## Environment
- OS: Linux
"#;

        let issue = FeatureBugIssue {
            number: 1,
            title: issue.title,
            body: complete_body.to_string(),
            author: issue.author,
            is_bug: true,
            is_feature: false,
        };

        // Completeness check
        let result = check_bug_completeness(&issue.body);

        // Transition decision - should be immediate
        let transition = if result.is_complete {
            TransitionSummary::bug_ready_for_review(&issue)
        } else {
            TransitionSummary::bug_needs_information(&issue, &result.request_message)
        };

        // Both operations should complete in this single run
        assert!(result.is_complete);
        assert!(transition.applied_ready_for_review);
    }
}
