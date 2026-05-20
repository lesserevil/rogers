//! Triage loop for Rodgers.
//!
//! This module implements the main triage logic as defined in
//! plans/triage-workflow-plan.md and plans/feature-bug-plan.md.
//!
//! On each run, Rodgers processes all issues that have changed since
//! the last run. For each issue:
//! 1. Read full issue state (labels, comments, body, assignee)
//! 2. Classify as bug, feature, or other
//! 3. Run completeness check for bug/feature issues
//! 4. Apply appropriate transition (ready-for-review or needs-information)
//! 5. Process will-not-do decisions (post closure comment, close issue)
//!
//! All processing happens within ONE triage run - no delays.

use crate::feature_bug::will_not_do::{
    generate_warm_closure_comment, has_will_not_do_label, resolve_issue_type,
};
use crate::feature_bug::{
    FeatureBugIssue, TransitionSummary, check_bug_completeness, check_feature_completeness,
};
use serde::{Deserialize, Serialize};

/// Label constants for triage operations.
pub const LABEL_BUG: &str = "bug";
pub const LABEL_FEATURE: &str = "feature";
pub const LABEL_READY_FOR_REVIEW: &str = "ready-for-review";
pub const LABEL_NEEDS_INFORMATION: &str = "needs-information";
pub const LABEL_WILL_NOT_DO: &str = "will-not-do";
pub const LABEL_READY_FOR_WORK: &str = "ready-for-work";

/// Represents a GitHub issue with all relevant metadata for triage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageIssue {
    /// GitHub issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body
    pub body: String,
    /// Author username
    pub author: String,
    /// Current labels on the issue
    pub labels: Vec<String>,
    /// Issue state (open/closed)
    pub state: IssueState,
}

/// GitHub issue state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueState {
    Open,
    Closed,
}

/// Result of a triage operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageResult {
    /// Issue number processed
    pub issue_number: u64,
    /// Whether the issue was processed
    pub processed: bool,
    /// Action taken (if any)
    pub action: TriageAction,
    /// Comment to post (if any)
    pub comment_to_post: Option<String>,
    /// Labels to apply
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
}

/// Actions that can be taken during triage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TriageAction {
    /// No action needed
    NoAction,
    /// Applied ready-for-review (complete issue)
    AppliedReadyForReview,
    /// Applied needs-information (incomplete issue)
    AppliedNeedsInformation,
    /// Issue is already ready for review
    AlreadyReadyForReview,
    /// Human has applied will-not-do - closure comment needed
    WillNotDo,
    /// Issue has a human gate label (ready-for-work)
    HumanGateDetected,
    /// Issue is closed or archived
    SkippedClosed,
    /// Issue is not a bug or feature
    SkippedNotTriaged,
}

/// The main triage loop processor.
///
/// This processes a single issue and determines what action to take.
pub fn process_issue(issue: &TriageIssue) -> TriageResult {
    // Skip closed issues
    if issue.state == IssueState::Closed {
        return TriageResult {
            issue_number: issue.number,
            processed: false,
            action: TriageAction::SkippedClosed,
            comment_to_post: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Check if this is a bug or feature issue
    let is_bug = issue.labels.iter().any(|l| l == LABEL_BUG);
    let is_feature = issue.labels.iter().any(|l| l == LABEL_FEATURE);

    if !is_bug && !is_feature {
        return TriageResult {
            issue_number: issue.number,
            processed: false,
            action: TriageAction::SkippedNotTriaged,
            comment_to_post: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Check for human gate labels
    // Priority: will-not-do > ready-for-work (per state machine design)
    let has_will_not_do = has_will_not_do_label(&issue.labels);
    let has_ready_for_work = issue.labels.iter().any(|l| l == LABEL_READY_FOR_WORK);

    if has_will_not_do {
        // Will-not-do takes priority - generate warm closure and mark for close
        let issue_type = resolve_issue_type(&issue.labels);
        let closure_comment =
            generate_warm_closure_comment(&issue.title, &issue.author, &issue_type);

        return TriageResult {
            issue_number: issue.number,
            processed: true,
            action: TriageAction::WillNotDo,
            comment_to_post: Some(closure_comment),
            labels_to_add: Vec::new(),
            labels_to_remove: vec![LABEL_READY_FOR_REVIEW.to_string()],
        };
    }

    if has_ready_for_work {
        return TriageResult {
            issue_number: issue.number,
            processed: false,
            action: TriageAction::HumanGateDetected,
            comment_to_post: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Check if already ready for review
    let already_ready = issue.labels.iter().any(|l| l == LABEL_READY_FOR_REVIEW);
    if already_ready {
        return TriageResult {
            issue_number: issue.number,
            processed: false,
            action: TriageAction::AlreadyReadyForReview,
            comment_to_post: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Run completeness check
    run_completeness_check(issue, is_bug, is_feature)
}

/// Run the completeness check and return the appropriate transition.
fn run_completeness_check(issue: &TriageIssue, is_bug: bool, is_feature: bool) -> TriageResult {
    let fb_issue = FeatureBugIssue {
        number: issue.number,
        title: issue.title.clone(),
        body: issue.body.clone(),
        author: issue.author.clone(),
        is_bug,
        is_feature,
    };

    let (is_complete, transition) = if is_bug {
        let result = check_bug_completeness(&issue.body);
        if result.is_complete {
            (true, TransitionSummary::bug_ready_for_review(&fb_issue))
        } else {
            (
                false,
                TransitionSummary::bug_needs_information(&fb_issue, &result.request_message),
            )
        }
    } else {
        let result = check_feature_completeness(&issue.body);
        if result.is_complete {
            (true, TransitionSummary::feature_ready_for_review(&fb_issue))
        } else {
            (
                false,
                TransitionSummary::feature_needs_information(&fb_issue, &result.request_message),
            )
        }
    };

    TriageResult {
        issue_number: issue.number,
        processed: true,
        action: if is_complete {
            TriageAction::AppliedReadyForReview
        } else {
            TriageAction::AppliedNeedsInformation
        },
        comment_to_post: Some(transition.comment),
        labels_to_add: transition.labels_to_add,
        labels_to_remove: transition.labels_to_remove,
    }
}

/// Batch process multiple issues as part of a single triage run.
pub fn process_issues_batch(issues: &[TriageIssue]) -> Vec<TriageResult> {
    issues.iter().map(process_issue).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue(labels: Vec<&str>, body: &str, state: IssueState) -> TriageIssue {
        TriageIssue {
            number: 1,
            title: "Test Issue".to_string(),
            body: body.to_string(),
            author: "testuser".to_string(),
            labels: labels.into_iter().map(String::from).collect(),
            state,
        }
    }

    #[test]
    fn test_complete_bug_transitions_in_one_run() {
        let complete_bug = r#"
## Behavior Observed
It crashes.

## Behavior Expected
It should not crash.

## Reproduction Steps
1. Click the button

## Environment
OS: Linux
"#;

        let issue = create_test_issue(vec!["bug"], complete_bug, IssueState::Open);

        // Process the issue - should complete in ONE run
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        assert!(
            result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(result.comment_to_post.is_some());
    }

    #[test]
    fn test_complete_feature_transitions_in_one_run() {
        let complete_feature = r#"
## Use Case
I need this feature.

## Proposed Behavior
It should do X.

## Acceptance Criteria
- [ ] It does X
"#;

        let issue = create_test_issue(vec!["feature"], complete_feature, IssueState::Open);

        // Process the issue - should complete in ONE run
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        assert!(
            result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    #[test]
    fn test_incomplete_bug_applies_needs_information_in_one_run() {
        let incomplete_bug = r#"
## Behavior Observed
It does not work.
"#;

        let issue = create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open);

        // Process the issue - should request info in ONE run
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        assert!(
            result
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
        assert!(result.comment_to_post.is_some());
        assert!(
            result
                .comment_to_post
                .as_ref()
                .unwrap()
                .contains("Reproduction steps")
        );
    }

    #[test]
    fn test_summary_comment_posted() {
        let complete_bug = r#"
## What Happened
Something wrong

## What Should Happen
Something right

## Steps
1. Step one

## Environment
Linux
"#;

        let issue = create_test_issue(vec!["bug"], complete_bug, IssueState::Open);

        let result = process_issue(&issue);

        assert!(result.comment_to_post.is_some());
        let comment = result.comment_to_post.as_ref().unwrap();
        assert!(comment.contains("summary") || comment.contains("Thank you"));
    }

    #[test]
    fn test_no_delay_same_run() {
        // Verify that complete issues transition immediately without delay
        let complete_feature = r#"
## Use Case
I want feature X

## Proposed Behavior
It does X

## Verification
- [ ] Works correctly
"#;

        let issue = create_test_issue(vec!["feature"], complete_feature, IssueState::Open);

        // Single call to process_issue should result in ready-for-review
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        // No additional conditions - immediate transition
    }

    #[test]
    fn test_already_ready_for_review_skipped() {
        let complete_bug = r#"
## Behavior Observed
It crashes

## Behavior Expected
No crash

## Reproduction Steps
1. Click

## Environment
Linux
"#;

        let issue = create_test_issue(
            vec!["bug", "ready-for-review"],
            complete_bug,
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::AlreadyReadyForReview);
    }

    #[test]
    fn test_closed_issues_skipped() {
        let issue = create_test_issue(vec!["bug"], "Body", IssueState::Closed);

        let result = process_issue(&issue);

        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::SkippedClosed);
    }

    #[test]
    fn test_non_triaged_issues_skipped() {
        let issue = create_test_issue(vec![], "Just a question?", IssueState::Open);

        let result = process_issue(&issue);

        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::SkippedNotTriaged);
    }

    #[test]
    fn test_human_gate_approved_for_work() {
        // When human applies ready-for-work (approved), triage detects but waits
        let issue = create_test_issue(vec!["bug", "ready-for-work"], "Body", IssueState::Open);

        let result = process_issue(&issue);

        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::HumanGateDetected);
    }

    #[test]
    fn test_batch_processing() {
        let complete_bug = r#"
## Behavior Observed
Bug 1

## Behavior Expected
No bug

## Reproduction Steps
1. Step

## Environment
Linux
"#;

        let incomplete_bug = r#"
## What
Just a title
"#;

        let complete_feature = r#"
## Use Case
Use

## Proposed Behavior
Work

## Acceptance
- [ ] Works
"#;

        let issues = vec![
            create_test_issue(vec!["bug"], complete_bug, IssueState::Open),
            create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open),
            create_test_issue(vec!["feature"], complete_feature, IssueState::Open),
        ];

        let results = process_issues_batch(&issues);

        // All three should be processed
        assert_eq!(results.len(), 3);

        // First issue - complete bug - ready-for-review
        assert_eq!(results[0].action, TriageAction::AppliedReadyForReview);

        // Second issue - incomplete bug - needs-information
        assert_eq!(results[1].action, TriageAction::AppliedNeedsInformation);

        // Third issue - complete feature - ready-for-review
        assert_eq!(results[2].action, TriageAction::AppliedReadyForReview);
    }

    #[test]
    fn test_removes_needs_information_on_complete() {
        // Issue that previously had needs-information but is now complete
        let complete_bug = r#"
## Behavior Observed
Fixed!

## Behavior Expected
Works!

## Reproduction Steps
N/A - timing issue

## Environment
macOS
"#;

        let issue = create_test_issue(
            vec!["bug", "needs-information"],
            complete_bug,
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        assert!(
            result
                .labels_to_remove
                .contains(&"needs-information".to_string())
        );
    }

    #[test]
    fn test_incomplete_feature_requests_specific_fields() {
        let incomplete_feature = r#"
## Why
I want something...
"#;

        let issue = create_test_issue(vec!["feature"], incomplete_feature, IssueState::Open);

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);

        let comment = result.comment_to_post.as_ref().unwrap();
        // Check for specific field requests - use the actual formatted output
        assert!(comment.contains("proposed behavior") || comment.contains("Acceptance criteria"));
    }

    #[test]
    fn test_complete_bug_all_fields() {
        // Verify that a bug with ALL four required fields transitions
        let body = r#"
## What Happened
The application gives an error

## Expected Result
The application should succeed

## How to Reproduce
1. Open the app
2. Login
3. Submit form

## System Details
- OS: Windows 11
- Version: 2.0.0
- Browser: Chrome
"#;

        let issue = create_test_issue(vec!["bug"], body, IssueState::Open);

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
    }

    #[test]
    fn test_complete_feature_all_fields() {
        // Verify that a feature with all three required fields transitions
        let body = r#"
## User Story
As a developer, I want API keys to be rotated automatically so I don't have to do it manually.

## What Should Happen
The system should generate new API keys monthly and notify users.

## How to Test
- [ ] New keys are generated each month
- [ ] Users receive notification
- [ ] Old keys are invalidated after grace period
- [ ] Existing integrations continue to work during transition
"#;

        let issue = create_test_issue(vec!["feature"], body, IssueState::Open);

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
    }

    // Will-not-do tests - CRIT-3

    #[test]
    fn test_will_not_do_detected_generates_closure_comment() {
        // When human applies will-not-do, triage should generate closure comment
        let issue = create_test_issue(
            vec!["bug", "will-not-do"],
            "Bug report body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::WillNotDo);
        assert!(result.comment_to_post.is_some());
    }

    #[test]
    fn test_will_not_do_closure_comment_is_warm() {
        // The closure comment should be warm, not curt
        let issue = create_test_issue(
            vec!["feature", "will-not-do"],
            "Feature request body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        let comment = result.comment_to_post.unwrap();
        // Should express gratitude
        assert!(comment.contains("Thanks"));
        // Should not be curt
        assert!(!comment.contains("not a priority"));
    }

    #[test]
    fn test_will_not_do_removes_ready_for_review() {
        // Should remove ready-for-review label
        let issue = create_test_issue(
            vec!["bug", "will-not-do", "ready-for-review"],
            "Bug report body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(
            result
                .labels_to_remove
                .contains(&"ready-for-review".to_string())
        );
    }

    #[test]
    fn test_will_not_do_in_one_triage_run() {
        // Verify will-not-do detection AND closure action happen in ONE run
        let issue = create_test_issue(
            vec!["bug", "will-not-do", "ready-for-review"],
            "Bug report body",
            IssueState::Open,
        );

        // Single call to process_issue should result in WillNotDo action
        let result = process_issue(&issue);

        // All of these are ready in ONE triage run:
        // 1. WillNotDo action detected
        assert_eq!(result.action, TriageAction::WillNotDo);
        // 2. Closure comment generated
        assert!(result.comment_to_post.is_some());
        // 3. Labels to remove identified
        assert!(!result.labels_to_remove.is_empty());

        // These API calls can execute in the same run:
        // - POST comment with closure_comment
        // - Remove ready-for-review label
        // - Close issue
    }

    #[test]
    fn test_will_not_do_priority_over_ready_for_work() {
        // When both labels are present, will-not-do takes priority
        let issue = create_test_issue(
            vec!["bug", "will-not-do", "ready-for-work"],
            "Bug report body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        // Will-not-do should be processed, not ready-for-work
        assert_eq!(result.action, TriageAction::WillNotDo);
        assert!(result.comment_to_post.is_some());
    }

    #[test]
    fn test_will_not_do_feature_type() {
        // Verify issue type is correctly identified as feature request
        let issue = create_test_issue(
            vec!["feature", "will-not-do"],
            "Feature request body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        let comment = result.comment_to_post.unwrap();
        assert!(comment.contains("feature request"));
    }

    #[test]
    fn test_will_not_do_bug_type() {
        // Verify issue type is correctly identified as bug report
        let issue = create_test_issue(
            vec!["bug", "will-not-do"],
            "Bug report body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        let comment = result.comment_to_post.unwrap();
        assert!(comment.contains("bug report"));
    }

    #[test]
    fn test_will_not_do_includes_author() {
        // The closure comment should address the author
        let issue = create_test_issue(
            vec!["bug", "will-not-do"],
            "Bug report body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        let comment = result.comment_to_post.unwrap();
        // Should mention the author
        assert!(comment.contains("@testuser"));
    }

    #[test]
    fn test_will_not_do_closed_issue_skipped() {
        // Closed issues with will-not-do should be skipped
        let issue = create_test_issue(
            vec!["bug", "will-not-do"],
            "Bug report body",
            IssueState::Closed,
        );

        let result = process_issue(&issue);

        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::SkippedClosed);
    }

    #[test]
    fn test_will_not_do_no_labels_to_add() {
        // will-not-do label should already be present, no change needed
        let issue = create_test_issue(
            vec!["bug", "will-not-do"],
            "Bug report body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        // Will-not-do should already be on the issue - no need to add more
        assert!(result.labels_to_add.is_empty());
        assert!(
            result
                .labels_to_remove
                .contains(&"ready-for-review".to_string())
        );
    }

    #[test]
    fn test_batch_with_will_not_do() {
        // Test batch processing includes will-not-do handling
        let complete_bug = r#"
## What Happened
Bug fixed

## What Should Happen
Works

## Steps
1. Step

## Environment
Linux
"#;

        let issues = vec![
            create_test_issue(vec!["bug"], complete_bug, IssueState::Open),
            create_test_issue(vec!["bug", "will-not-do"], complete_bug, IssueState::Open),
        ];

        let results = process_issues_batch(&issues);

        assert_eq!(results.len(), 2);
        // First issue - complete bug - ready-for-review
        assert_eq!(results[0].action, TriageAction::AppliedReadyForReview);
        // Second issue - will-not-do - closure
        assert_eq!(results[1].action, TriageAction::WillNotDo);
        assert!(results[1].comment_to_post.is_some());
    }
}
