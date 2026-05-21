//! Triage router - Routes classified issues to appropriate workflow handlers.
//!
//! Implements Top-Level Classification → routes from plans/triage-workflow-plan.md
//! and plans/question-routing-plan.md.

use crate::issue_templates;
use crate::question_router::process_question;
use crate::triage::{IssueState, TriageAction, TriageIssue, TriageResult};

/// Label marking an issue as a question from the community.
pub const LABEL_QUESTION: &str = "question";
/// Label marking an issue as having been routed to the question workflow.
pub const LABEL_RODGERS_QUESTION: &str = "rodgers:question";

/// Label marking an issue as documentation/template work.
pub const LABEL_DOCS: &str = "docs";
/// Label marking an issue as having been routed to the issue-templates workflow.
pub const LABEL_RODGERS_DOCS: &str = "rodgers:docs";

/// Route a single issue to the appropriate workflow handler.
///
/// Returns `Some(TriageResult)` when the router handles the issue:
/// - Docs issues are routed to the issue-templates workflow
/// - Question issues are routed to the question-routing workflow
/// - Already-handled questions and docs return a no-op result
///
/// Returns `None` when the issue should be handled by the default bug/feature
/// workflow in `process_issue`.
///
/// Routing priority:
/// 1. Closed issues → None (skip)
/// 2. Already-routed docs → NoAction
/// 3. Docs label → route to issue-templates workflow
/// 4. Already-routed questions → NoAction
/// 5. Question label → route to question-routing workflow
/// 6. Everything else → None (fall through to bug/feature)
pub fn route_issue(issue: &TriageIssue) -> Option<TriageResult> {
    // Skip closed issues - let normal closed handling deal with them
    if issue.state == IssueState::Closed {
        return None;
    }

    // Already routed to issue-templates workflow: no-op so we don't re-process
    if issue.labels.iter().any(|l| l == LABEL_RODGERS_DOCS) {
        return Some(TriageResult {
            issue_number: issue.number,
            processed: false,
            action: TriageAction::NoAction,
            comment_to_post: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        });
    }

    // Route docs issues to the issue-templates workflow
    // This handles: missing templates, template field updates, documentation corrections
    //
    // Edge case: Issues with the `question` label that happen to be about docs
    // are handled by the question workflow (doc-gap beads route there, not here).
    // Only issues with the `docs` label (explicitly classified as docs work) go here.
    if issue.labels.iter().any(|l| l == LABEL_DOCS) {
        return Some(route_docs_issue(issue));
    }

    // Already routed to question workflow: no-op so we don't re-process
    if issue.labels.iter().any(|l| l == LABEL_RODGERS_QUESTION) {
        return Some(TriageResult {
            issue_number: issue.number,
            processed: false,
            action: TriageAction::NoAction,
            comment_to_post: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        });
    }

    // Route question issues to the question-routing workflow
    if issue.labels.iter().any(|l| l == LABEL_QUESTION) {
        return Some(route_question_issue(issue));
    }

    // Not a question or docs; fall through to bug/feature workflow
    None
}

/// Route a question issue through the question-routing workflow.
///
/// The `rodgers:question` label is applied **before** any routing decision so
/// that subsequent triage runs know this issue has already entered the
/// question workflow.
fn route_question_issue(issue: &TriageIssue) -> TriageResult {
    // CRIT-3 requirement: apply rodgers:question label BEFORE routing
    let mut labels_to_add = vec![LABEL_RODGERS_QUESTION.to_string()];

    // Hand off to the question router for doc search, code search, or doc-gap
    let qr_output = process_question(issue);

    // Merge any additional labels from the question router
    labels_to_add.extend(qr_output.labels_to_add);

    TriageResult {
        issue_number: issue.number,
        processed: qr_output.processed,
        action: qr_output.action,
        comment_to_post: qr_output.comment,
        labels_to_add,
        labels_to_remove: qr_output.labels_to_remove,
    }
}

/// Route a docs issue through the issue-templates workflow.
///
/// The `rodgers:docs` label is applied **before** any routing decision so
/// that subsequent triage runs know this issue has already entered the
/// issue-templates workflow.
///
/// This handles: missing templates, template field updates, documentation corrections.
/// Template changes require human review (governance decision) — Rodgers files a bead
/// but does not auto-commit.
fn route_docs_issue(issue: &TriageIssue) -> TriageResult {
    // Apply rodgers:docs label BEFORE routing
    let mut labels_to_add = vec![LABEL_RODGERS_DOCS.to_string()];

    // Hand off to the issue-templates workflow
    let it_output = issue_templates::process_issue_templates(issue);

    // Merge any additional labels from the issue-templates workflow
    labels_to_add.extend(it_output.labels_to_add);

    TriageResult {
        issue_number: issue.number,
        processed: it_output.processed,
        action: it_output.action,
        comment_to_post: it_output.comment,
        labels_to_add,
        labels_to_remove: it_output.labels_to_remove,
    }
}

/// Batch route multiple issues as part of a single triage run.
pub fn route_issues(issues: &[TriageIssue]) -> Vec<TriageResult> {
    issues
        .iter()
        .map(|issue| {
            route_issue(issue).unwrap_or_else(|| {
                // If the router didn't handle it, this is a non-question issue.
                // Return a placeholder that the caller can replace with bug/feature
                // processing.  We use SkippedNotTriaged as the sentinel so that
                // batch callers can distinguish "not routed" from "routed as noop".
                TriageResult {
                    issue_number: issue.number,
                    processed: false,
                    action: TriageAction::SkippedNotTriaged,
                    comment_to_post: None,
                    labels_to_add: Vec::new(),
                    labels_to_remove: Vec::new(),
                }
            })
        })
        .collect()
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
    fn test_question_issue_gets_rodgers_question_label() {
        let issue = create_test_issue(
            vec!["question"],
            "How do I configure the scheduler interval?",
            IssueState::Open,
        );

        let result = route_issue(&issue).expect("should route question issues");

        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string())
        );
    }

    #[test]
    fn test_question_issue_routed_to_question_workflow() {
        let issue = create_test_issue(
            vec!["question"],
            "How does the triage loop classify issues?",
            IssueState::Open,
        );

        let result = route_issue(&issue).expect("should route question issues");

        assert!(result.processed);
        assert!(
            result.action == TriageAction::RoutedToQuestionWorkflow
                || result.action == TriageAction::AppliedNeedsInformation
                || result.action == TriageAction::QuestionAnsweredDoc
                || result.action == TriageAction::QuestionAnsweredCode
                || result.action == TriageAction::QuestionDocGapFiled
                || result.action == TriageAction::QuestionReclassified,
            "question should be routed to a question workflow action, got {:?}",
            result.action
        );
    }

    #[test]
    fn test_non_question_issue_not_routed() {
        let issue = create_test_issue(vec!["bug"], "The app crashes on startup.", IssueState::Open);

        let result = route_issue(&issue);

        assert!(
            result.is_none(),
            "bug issues should not be routed by question router"
        );
    }

    #[test]
    fn test_feature_issue_not_routed() {
        let issue = create_test_issue(vec!["feature"], "Add dark mode support.", IssueState::Open);

        let result = route_issue(&issue);

        assert!(
            result.is_none(),
            "feature issues should not be routed by question router"
        );
    }

    #[test]
    fn test_question_already_handled_is_noop() {
        let issue = create_test_issue(
            vec!["question", "rodgers:question"],
            "How do I configure the app?",
            IssueState::Open,
        );

        let result = route_issue(&issue).expect("should return noop for already-handled");

        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::NoAction);
        assert!(result.labels_to_add.is_empty());
    }

    #[test]
    fn test_closed_question_skipped_by_router() {
        let issue = create_test_issue(vec!["question"], "How do I install?", IssueState::Closed);

        let result = route_issue(&issue);

        assert!(
            result.is_none(),
            "closed questions should be skipped by router"
        );
    }

    #[test]
    fn test_batch_routing_mixed_issues() {
        let issues = vec![
            create_test_issue(vec!["question"], "How do I install?", IssueState::Open),
            create_test_issue(vec!["bug"], "It crashes.", IssueState::Open),
            create_test_issue(vec!["feature"], "Add X.", IssueState::Open),
        ];

        let results = route_issues(&issues);

        assert_eq!(results.len(), 3);
        // Question routed
        assert!(
            results[0]
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string())
        );
        // Bug not routed by router (SkippedNotTriaged sentinel)
        assert_eq!(results[1].action, TriageAction::SkippedNotTriaged);
        // Feature not routed
        assert_eq!(results[2].action, TriageAction::SkippedNotTriaged);
    }

    #[test]
    fn test_rodgers_question_label_applied_before_routing() {
        let issue = create_test_issue(
            vec!["question"],
            "What is the configuration format?",
            IssueState::Open,
        );

        let result = route_issue(&issue).expect("should route");

        // rodgers:question must be present in labels_to_add
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string()),
            "rodgers:question must be applied before routing"
        );
        // Some question workflow action should also be present
        assert!(
            result.processed || result.action == TriageAction::NoAction,
            "routing should have occurred"
        );
    }

    // =============================================================================
    // Docs routing tests
    // =============================================================================

    #[test]
    fn test_docs_issue_gets_rodgers_docs_label() {
        let issue = create_test_issue(
            vec!["docs"],
            "Missing issue templates for the project.",
            IssueState::Open,
        );

        let result = route_issue(&issue).expect("should route docs issues");

        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_DOCS.to_string()),
            "docs issue should get rodgers:docs label"
        );
    }

    #[test]
    fn test_docs_issue_routed_to_issue_templates_workflow() {
        let issue = create_test_issue(
            vec!["docs"],
            "Missing templates, suggested templates available.",
            IssueState::Open,
        );

        let result = route_issue(&issue).expect("should route docs issues");

        assert!(
            result.processed,
            "docs issue should be processed by issue-templates workflow"
        );
        assert!(
            result.action == TriageAction::RoutedToIssueTemplates,
            "docs issue should be routed to issue-templates workflow, got {:?}",
            result.action
        );
    }

    #[test]
    fn test_docs_already_handled_is_noop() {
        let issue = create_test_issue(
            vec!["docs", "rodgers:docs"],
            "Missing templates",
            IssueState::Open,
        );

        let result = route_issue(&issue).expect("should return noop for already-handled");

        assert!(!result.processed);
        assert_eq!(result.action, TriageAction::NoAction);
        assert!(result.labels_to_add.is_empty());
    }

    #[test]
    fn test_closed_docs_issue_skipped_by_router() {
        let issue = create_test_issue(vec!["docs"], "Missing templates", IssueState::Closed);

        let result = route_issue(&issue);

        assert!(result.is_none(), "closed docs should be skipped by router");
    }

    #[test]
    fn test_rodgers_docs_label_applied_before_routing() {
        let issue = create_test_issue(vec!["docs"], "Missing templates", IssueState::Open);

        let result = route_issue(&issue).expect("should route");

        // rodgers:docs must be present in labels_to_add
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_DOCS.to_string()),
            "rodgers:docs must be applied before routing"
        );
    }

    #[test]
    fn test_docs_issue_with_comment() {
        let issue = create_test_issue(vec!["docs"], "Missing templates", IssueState::Open);

        let result = route_issue(&issue).expect("should route");

        // Should have a comment from the issue-templates workflow
        assert!(result.comment_to_post.is_some());
        assert!(
            result
                .comment_to_post
                .as_ref()
                .unwrap()
                .contains("Hi @testuser"),
            "comment should address the author"
        );
    }

    #[test]
    fn test_bug_issue_not_routed_to_docs() {
        // Bug issues should not be routed even if they mention docs
        let issue = create_test_issue(
            vec!["bug"],
            "The docs on the website are wrong",
            IssueState::Open,
        );

        let result = route_issue(&issue);

        assert!(
            result.is_none(),
            "bug issues should not be routed by router"
        );
    }

    #[test]
    fn test_batch_routing_with_docs_issues() {
        let issues = vec![
            create_test_issue(vec!["docs"], "Missing templates", IssueState::Open),
            create_test_issue(vec!["question"], "How do I install?", IssueState::Open),
            create_test_issue(vec!["bug"], "It crashes.", IssueState::Open),
            create_test_issue(vec!["feature"], "Add X.", IssueState::Open),
        ];

        let results = route_issues(&issues);

        assert_eq!(results.len(), 4);
        // Docs routed
        assert!(
            results[0]
                .labels_to_add
                .contains(&LABEL_RODGERS_DOCS.to_string())
        );
        assert_eq!(results[0].action, TriageAction::RoutedToIssueTemplates);
        // Question routed
        assert!(
            results[1]
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string())
        );
        // Bug not routed
        assert_eq!(results[2].action, TriageAction::SkippedNotTriaged);
        // Feature not routed
        assert_eq!(results[3].action, TriageAction::SkippedNotTriaged);
    }

    #[test]
    fn test_docs_vs_question_distinction() {
        // docs label → issue-templates workflow
        let docs_issue = create_test_issue(vec!["docs"], "Missing templates", IssueState::Open);
        let docs_result = route_issue(&docs_issue).expect("should route docs");
        assert_eq!(docs_result.action, TriageAction::RoutedToIssueTemplates);

        // question label → question workflow (even about docs content)
        let question_issue = create_test_issue(
            vec!["question"],
            "Where is the documentation?",
            IssueState::Open,
        );
        let question_result = route_issue(&question_issue).expect("should route question");
        assert!(
            question_result.action != TriageAction::RoutedToIssueTemplates,
            "question issues should not go to issue-templates workflow"
        );
    }
}
