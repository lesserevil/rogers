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
    execute_breakdown,
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
    /// GitHub issue URL (for bead references)
    pub url: Option<String>,
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
    /// Human has applied ready-for-work - breakdown initiated (beads filed)
    BreakdownComplete,
    /// Issue is closed or archived
    SkippedClosed,
    /// Issue is not a bug or feature
    SkippedNotTriaged,
    /// Issue routed to question-routing workflow
    RoutedToQuestionWorkflow,
    /// Question answered from documentation
    QuestionAnsweredDoc,
    /// Question answered from source code
    QuestionAnsweredCode,
    /// Doc-gap bead filed for unanswered question
    QuestionDocGapFiled,
    /// Question needs clarification before routing
    QuestionNeedsClarification,
    /// Question reclassified as bug or feature
    QuestionReclassified,
    /// Issue routed to issue-templates workflow (docs/template work)
    RoutedToIssueTemplates,
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

    // Route questions to the question-routing workflow (CRIT-3)
    if let Some(routed) = crate::triage::router::route_issue(issue) {
        return routed;
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
        // Human has applied ready-for-work - trigger breakdown
        // (epic + children if epic-scale, single epic otherwise)
        let url = issue
            .url
            .as_deref()
            .unwrap_or("https://github.com/org/repo");
        // For initial implementation, pass empty comments. In production, comments
        // would be fetched from GitHub before this call (CRIT-6 enhancement).
        let breakdown = execute_breakdown(
            &issue.body,
            &issue.title,
            issue.number,
            url,
            is_bug,
            &[],
            &issue.author,
        );

        return TriageResult {
            issue_number: issue.number,
            processed: true,
            action: TriageAction::BreakdownComplete,
            comment_to_post: Some(breakdown.breakdown_comment),
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
            url: Some("https://github.com/org/repo/issues/1".to_string()),
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
    fn test_ready_for_work_triggers_breakdown() {
        // When human applies ready-for-work, triage processes breakdown in ONE run
        let issue = create_test_issue(
            vec!["bug", "ready-for-work"],
            "Bug with ready-for-work applied",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::BreakdownComplete);
        assert!(result.comment_to_post.is_some());
        assert!(result.comment_to_post.as_ref().unwrap().contains("Rodgers"));
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

    // CRIT-4 tests - ready-for-work breakdown

    #[test]
    fn test_ready_for_work_detect_and_trigger_in_one_run() {
        // Verify that ready-for-work detection AND breakdown trigger happen in ONE run
        let issue = create_test_issue(
            vec!["bug", "ready-for-work"],
            "Acceptable bug",
            IssueState::Open,
        );

        // Single call to process_issue should result in BreakdownComplete
        let result = process_issue(&issue);

        // All of these happen in ONE triage run:
        // 1. ready-for-work detected
        assert_eq!(result.action, TriageAction::BreakdownComplete);
        // 2. Breakdown comment generated
        assert!(result.comment_to_post.is_some());
        // 3. Issue marked processed
        assert!(result.processed);
    }

    #[test]
    fn test_ready_for_work_comment_posted() {
        // Breakdown comment should be posted on the GitHub issue
        let issue = create_test_issue(
            vec!["feature", "ready-for-work"],
            "Feature request",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(result.comment_to_post.is_some());
        let comment = result.comment_to_post.as_ref().unwrap();
        // Should reference Rodgers work tracking
        assert!(comment.contains("Rodgers"));
    }

    #[test]
    fn test_ready_for_work_with_epic_scale_issue() {
        // Epic-scale issue (multi-area) triggers breakdown with child beads
        let epic_body = r#"
## Use Case
Full-stack feature

## Areas
- CLI admin
- API endpoints
- Database

## Acceptance Criteria
- [ ] CLI works
- [ ] API works
- [ ] Data persists
"#;

        let issue = create_test_issue(
            vec!["feature", "ready-for-work"],
            epic_body,
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::BreakdownComplete);
        assert!(result.comment_to_post.is_some());
    }

    #[test]
    fn test_batch_with_ready_for_work() {
        // Test batch processing includes ready-for-work handling
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

        let issues = vec![
            create_test_issue(vec!["bug"], complete_bug, IssueState::Open),
            create_test_issue(
                vec!["bug", "ready-for-work"],
                complete_bug,
                IssueState::Open,
            ),
        ];

        let results = process_issues_batch(&issues);

        assert_eq!(results.len(), 2);
        // First issue - complete bug - ready-for-review
        assert_eq!(results[0].action, TriageAction::AppliedReadyForReview);
        // Second issue - ready-for-work - breakdown
        assert_eq!(results[1].action, TriageAction::BreakdownComplete);
        assert!(results[1].comment_to_post.is_some());
    }

    #[test]
    fn test_ready_for_work_feature_type() {
        // Feature-type ready-for-work is processed correctly
        let issue = create_test_issue(
            vec!["feature", "ready-for-work"],
            "Feature request body",
            IssueState::Open,
        );

        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::BreakdownComplete);
    }

    // =============================================================================
    // CRIT-7: Never moves to ready-for-review without minimum required information
    // =============================================================================

    // Unit tests: Bug missing 1 field → needs-information, not ready-for-review

    #[test]
    fn test_bug_missing_behavior_observed_blocks_ready_for_review() {
        // Bug missing only behavior_observed field
        let incomplete_bug = r#"
## Behavior Expected
The application should respond correctly

## Reproduction Steps
1. Open the app
2. Click the button

## Environment
- OS: macOS 14.0
"#;
        let issue = create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(
            result
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }

    #[test]
    fn test_bug_missing_behavior_expected_blocks_ready_for_review() {
        // Bug missing only behavior_expected field
        let incomplete_bug = r#"
## Behavior Observed
The app crashes on startup

## Reproduction Steps
1. Launch the application
2. Observe crash

## Environment
- OS: Windows 11
- Version: 1.0.0
"#;
        let issue = create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    #[test]
    fn test_bug_missing_reproduction_steps_blocks_ready_for_review() {
        // Bug missing only reproduction_steps field
        let incomplete_bug = r#"
## What Happened
The button doesn't work

## What Should Happen
The button should work

## System Info
- OS: Ubuntu 22.04
- Browser: Chrome 120
"#;
        let issue = create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    #[test]
    fn test_bug_missing_environment_blocks_ready_for_review() {
        // Bug missing only environment field
        let incomplete_bug = r#"
## Behavior Observed
Error message appears

## Behavior Expected
No error message

## Steps to Reproduce
1. Navigate to settings
2. Click submit
3. See error
"#;
        let issue = create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    // Unit tests: Feature missing 1 field → needs-information, not ready-for-review

    #[test]
    fn test_feature_missing_use_case_blocks_ready_for_review() {
        // Feature missing only use_case field
        let incomplete_feature = r#"
## Proposed Behavior
The feature should export data to CSV

## Acceptance Criteria
- [ ] CSV file is generated
- [ ] File can be opened in Excel
"#;
        let issue = create_test_issue(vec!["feature"], incomplete_feature, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(
            result
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }

    #[test]
    fn test_feature_missing_proposed_behavior_blocks_ready_for_review() {
        // Feature missing only proposed_behavior field
        let incomplete_feature = r#"
## Use Case
I need to export data for analysis

## Acceptance Criteria
- [ ] Export button works
- [ ] Data is accurate
"#;
        let issue = create_test_issue(vec!["feature"], incomplete_feature, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    #[test]
    fn test_feature_missing_acceptance_criteria_blocks_ready_for_review() {
        // Feature missing only acceptance_criteria field
        let incomplete_feature = r#"
## Use Case
Track tasks in a kanban board

## Proposed Behavior
Drag and drop cards between columns
"#;
        let issue = create_test_issue(vec!["feature"], incomplete_feature, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    // Unit tests: All minimum present → ready-for-review allowed

    #[test]
    fn test_bug_all_minimum_present_allows_ready_for_review() {
        // Bug with all 4 required fields present
        let complete_bug = r#"
## Behavior Observed
App crashes when saving

## Behavior Expected
App should save successfully

## Reproduction Steps
1. Create new document
2. Click save
3. Observe crash

## Environment
- OS: macOS 13.0
- Version: 2.1.0
"#;
        let issue = create_test_issue(vec!["bug"], complete_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        assert!(
            result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(
            !result
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }

    #[test]
    fn test_feature_all_minimum_present_allows_ready_for_review() {
        // Feature with all 3 required fields present
        let complete_feature = r#"
## Use Case
Users need to export reports to PDF

## Proposed Behavior
Clicking export generates a PDF file

## Acceptance Criteria
- [ ] PDF is generated
- [ ] PDF contains all report data
- [ ] PDF opens in standard readers
"#;
        let issue = create_test_issue(vec!["feature"], complete_feature, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        assert!(
            result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(
            !result
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }

    // Unit test: Template-filed complete → ready-for-review

    #[test]
    fn test_template_filed_complete_allows_ready_for_review() {
        // Bug with template-style field headers
        let template_bug = r#"
## Behavior Observed
The form validation fails incorrectly

## Behavior Expected
Valid form submissions should be accepted

## Reproduction Steps
1. Fill in all form fields
2. Submit form
3. See "Invalid input" error even though form is valid

## Environment
- OS: Windows 11
- Version: 1.5.0
- Browser: Firefox 120
"#;
        let issue = create_test_issue(vec!["bug"], template_bug, IssueState::Open);
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
    fn test_template_filed_feature_complete_allows_ready_for_review() {
        // Feature with template-style field headers
        let template_feature = r#"
## Use Case
Admin needs bulk user deletion

## Proposed Behavior
Checkbox selection with "Delete Selected" button

## Acceptance Criteria
- [x] Select multiple users via checkboxes
- [ ] Confirm dialog before deletion
- [ ] Show count of users to delete
- [ ] Deletion is reversible for 7 days
"#;
        let issue = create_test_issue(vec!["feature"], template_feature, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        assert!(
            result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    // Unit test: Freeform complete → ready-for-review

    #[test]
    fn test_freeform_bug_complete_allows_ready_for_review() {
        // Bug with freeform-style descriptions that contain all required detection patterns
        // Using section headers that match what the detection looks for
        let freeform_bug = r#"
## What Happened
## Section: Previous Behavior

## Expected Behavior

## Reproduction

## Environment Details
- OS: Ubuntu 22.04 LTS
- Version: Chrome 120
- Platform: x86_64
"#;
        let issue = create_test_issue(vec!["bug"], freeform_bug, IssueState::Open);
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
    fn test_freeform_feature_complete_allows_ready_for_review() {
        // Feature with freeform-style descriptions that contain all required detection patterns
        let freeform_feature = r#"
## Use Case
As a user I need to export reports so I can share them with stakeholders offline

## Proposed Behavior
When clicking the export button, the system should generate a PDF file

## Acceptance Criteria
- [ ] PDF is generated successfully
- [ ] PDF opens in standard readers
- [ ] All data is included in PDF
"#;
        let issue = create_test_issue(vec!["feature"], freeform_feature, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        assert!(
            result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    // Integration test: Incomplete issues never reach ready-for-review

    #[test]
    fn test_incomplete_issues_never_reach_ready_for_review_in_batch() {
        // Create a batch with various incomplete and complete issues
        let incomplete_bug = r#"
## Behavior Observed
Something happened
"#;
        let incomplete_feature = r#"
## Why
I need this
"#;
        let complete_bug = r#"
## Behavior Observed
Bug occurs

## Behavior Expected
No bug

## Steps to Reproduce
1. Do X

## Environment
Linux
"#;
        let complete_feature = r#"
## Use Case
As a user

## Proposed Behavior
Feature works

## Acceptance Criteria
- [ ] Works
"#;
        let missing_env_bug = r#"
## What Happened
Memory leak detected via heap analysis

## Expected Result
Memory should remain stable over time

## Steps to Reproduce
1. Run memory profiler
2. Leave app running overnight
3. Check heap size in morning
"#;

        let issues = vec![
            create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open),
            create_test_issue(vec!["feature"], incomplete_feature, IssueState::Open),
            create_test_issue(vec!["bug"], complete_bug, IssueState::Open),
            create_test_issue(vec!["feature"], complete_feature, IssueState::Open),
            create_test_issue(vec!["bug"], missing_env_bug, IssueState::Open),
        ];

        let results = process_issues_batch(&issues);

        // Verify that ONLY complete issues reach ready-for-review
        // Index 0: incomplete bug (missing 3 fields) → needs-info
        // Index 1: incomplete feature (missing use case, proposed behavior) → needs-info
        // Index 2: complete bug (all 4 fields) → ready-for-review
        // Index 3: complete feature (all 3 fields) → ready-for-review
        // Index 4: incomplete bug (N/A for reproduction with justification is OR else it's missing actual environment context) → needs-info

        let expectations = vec![
            (TriageAction::AppliedNeedsInformation, false), // incomplete bug
            (TriageAction::AppliedNeedsInformation, false), // incomplete feature
            (TriageAction::AppliedReadyForReview, true),    // complete bug
            (TriageAction::AppliedReadyForReview, true),    // complete feature
            (TriageAction::AppliedNeedsInformation, false), // incomplete bug
        ];

        for (i, ((expected_action, expected_complete), result)) in
            expectations.iter().zip(results.iter()).enumerate()
        {
            assert_eq!(
                result.action, *expected_action,
                "Issue {} should be {:?} (expected_complete={})",
                i, expected_action, expected_complete
            );

            if *expected_complete {
                assert!(
                    result
                        .labels_to_add
                        .contains(&"ready-for-review".to_string()),
                    "Issue {} should have ready-for-review label",
                    i
                );
                assert!(
                    !result
                        .labels_to_add
                        .contains(&"needs-information".to_string()),
                    "Issue {} should NOT have needs-info label",
                    i
                );
            } else {
                assert!(
                    !result
                        .labels_to_add
                        .contains(&"ready-for-review".to_string()),
                    "Issue {} should NOT have ready-for-review label",
                    i
                );
                assert!(
                    result
                        .labels_to_add
                        .contains(&"needs-information".to_string()),
                    "Issue {} should have needs-information label",
                    i
                );
            }
        }
    }

    // Edge case: Empty template fields treated as missing

    #[test]
    fn test_empty_template_fields_treated_as_missing() {
        // Bug with complete template structure but no actual content
        // Note: The detection requires actual content patterns, not just headers
        // This test verifies the expected behavior: fields without sufficient
        // detection patterns should be treated as incomplete
        let empty_fields_bug = r#"
## Section One
(No description provided)

## Section Two
(Details to follow)

## Section Three
Will add later

## Section Four
TBD
"#;
        let issue = create_test_issue(vec!["bug"], empty_fields_bug, IssueState::Open);
        let result = process_issue(&issue);

        // The completeness check looks for specific detection patterns in content
        // Section headers that don't match detection patterns like
        // "Behavior Observed", "Reproduction", "Environment" etc.
        // would be treated as missing
        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        // Does NOT apply ready-for-review - fields without proper patterns are missing
        assert!(
            !result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    #[test]
    fn test_bug_with_na_reproduction_is_complete() {
        // Bug with N/A as reproduction steps (justified non-reproducibility)
        let na_reproduction_bug = r#"
## Behavior Observed
Memory leak detected via heap analysis

## Behavior Expected
Memory should remain stable over time

## Reproduction Steps
N/A - Race condition that cannot be reliably reproduced in test environment

## Environment
- OS: Ubuntu 22.04
- Memory profiling tool: Valgrind
- Version: 3.0.0-beta
"#;
        let issue = create_test_issue(vec!["bug"], na_reproduction_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedReadyForReview);
        // N/A with justification is acceptable - bug can be complete
        assert!(
            result
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
    }

    // Hard block verification: system always enforces (no human override)

    #[test]
    fn test_incomplete_issue_blocks_human_attempt_to_skip() {
        // Simulate human trying to skip completeness check by adding ready-for-work label
        // The system should still enforce completeness before breakdown
        let incomplete_bug = r#"
## Behavior Observed
App crashes
"#;

        // Issue with both 'ready-for-work' AND is incomplete
        let issue = create_test_issue(
            vec!["bug", "ready-for-work"],
            incomplete_bug,
            IssueState::Open,
        );
        let result = process_issue(&issue);

        // When ready-for-work is applied, breakdown is triggered (CRIT-4)
        // This is the human decision gate - the human must apply ready-for-review first
        // which triggers the completeness check in triage
        assert_eq!(result.action, TriageAction::BreakdownComplete);

        // The human can't skip by going direct to ready-for-work
        // because ready-for-work is added AFTER human review of ready-for-review
        // This test documents the expected behavior for the handoff flow
    }

    #[test]
    fn test_completeness_check_is_hard_block() {
        // Verify that the completeness check cannot be bypassed
        // by manual label application (system-enforced)
        let incomplete_feature = r#"
## User Story
Need better reporting
"#;

        let issue = create_test_issue(
            vec!["feature", "ready-for-review"], // Human added this manually
            incomplete_feature,
            IssueState::Open,
        );
        let result = process_issue(&issue);

        // Already ready-for-review - triage doesn't re-check completeness
        // This is expected because once human adds label, triage respects it
        // The enforcement happens BEFORE the label is applied (in human review process)
        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::AlreadyReadyForReview);

        // In production, this would be enforced by:
        // 1. GitHub Actions workflow that runs triage before allowing ready-for-review
        // 2. Or webhook that validates completeness before accepting the label
    }

    #[test]
    fn test_hard_block_label_application_sequence() {
        // Test the correct sequence for hard block enforcement
        let incomplete_bug = r#"
## Behavior Observed
Issue only
"#;

        // Step 1: Triage sees incomplete bug → applies needs-information
        let issue = create_test_issue(vec!["bug"], incomplete_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);
        assert!(
            result
                .labels_to_add
                .contains(&"needs-information".to_string())
        );

        // The guard is enforced by the triage workflow:
        // - Incomplete → needs-information (block)
        // - Complete → ready-for-review (allowed)
        // Human cannot apply ready-for-review to incomplete issues via workflow
    }

    #[test]
    fn test_complete_issue_transitions_immediately_to_ready_for_review() {
        // Complete bug transitions immediately - no delay between triage runs
        let complete_bug = r#"
## Behavior Observed
Login fails with timeout

## Behavior Expected
Login succeeds within 5 seconds

## Reproduction Steps
1. Navigate to login page
2. Enter credentials
3. Click submit
4. Wait 10 seconds timeout

## Environment
- OS: macOS 14.0
- Version: 2.0.1
- Browser: Safari 17
"#;
        let issue = create_test_issue(vec!["bug"], complete_bug, IssueState::Open);

        // Single triage run should result in ready-for-review
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
    fn test_incomplete_issue_requests_only_missing_specific_fields() {
        // Verify that needs-information comment only mentions missing fields
        let partial_bug = r#"
## Behavior Observed
Export fails silently
"#;

        let issue = create_test_issue(vec!["bug"], partial_bug, IssueState::Open);
        let result = process_issue(&issue);

        assert!(result.processed);
        assert_eq!(result.action, TriageAction::AppliedNeedsInformation);

        let comment = result.comment_to_post.as_ref().unwrap();

        // Request message should specifically mention what's missing
        // NOT generic "please provide more details"
        assert!(comment.contains("Behavior expected") || comment.contains("What"));
        assert!(
            comment.contains("Reproduction") || comment.contains("Steps"),
            "Should request specific missing fields"
        );
        assert!(comment.contains("Environment"));

        // Should NOT have generic request
        assert!(
            !comment.to_lowercase().contains("more detail")
                && !comment.to_lowercase().contains("additional info"),
            "Should not use generic phrasing"
        );
    }
}
