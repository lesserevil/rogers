//! Issue router - dispatches classified issues to the appropriate workflow handler.
//!
//! After the LLM classifies an issue (bug, feature, or question), the router
//! sends it to the corresponding workflow. This module handles:
//!
//! - **Bug issues**: routed to the feature/bug workflow with `rodgers:bug` label
//!   and severity assessment (critical/high/medium/low)
//! - **Feature issues**: routed to the feature/bug workflow
//! - **Question issues**: routed to the question-routing workflow
//!   - Immediately applies `rodgers:question` label
//!   - Calls the question router to handle within the same triage run
//!
//! Non-question issues (anything without `bug`, `feature`, or `question` labels)
//! are not routed here and are skipped.
//!
//! Plan: plans/triage-workflow-plan.md §Top-Level Classification

use crate::error::Result;
use crate::triage::severity::{Severity, assess_severity, severity_to_label};
use serde::{Deserialize, Serialize};

/// The TriageIssue type from triage_loop.
use super::TriageIssue;

/// Routing outcome after a router pass.
///
/// Used by the triage loop to apply label changes and dispatch to
/// the appropriate workflow handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    /// Issue number processed
    pub issue_number: u64,
    /// Whether the issue was routed (not a non-question issue)
    pub routed: bool,
    /// Workflow category the issue was routed to
    pub workflow: Option<Workflow>,
    /// Labels to add (includes rodgers:question for question issues,
    /// rodgers:bug for bug issues, and severity label)
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
    /// Comment to post (if any, e.g. question router doc-link or gap filing)
    pub comment_to_post: Option<String>,
    /// Whether the issue needs to go through question routing
    pub needs_question_routing: bool,
    /// Severity assessment for bug issues
    pub severity: Option<Severity>,
    /// Priority mapped from severity (P1-P4)
    pub priority: Option<String>,
    /// Whether this issue is a bug that was routed
    pub is_bug_routed: bool,
}

/// The workflow category for a routed issue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Workflow {
    /// Question-routing workflow (plans/question-routing-plan.md)
    QuestionRouting,
    /// Bug/Feature workflow (plans/feature-bug-plan.md)
    FeatureBug,
}

/// Label constant for marking question issues that have been routed.
pub const LABEL_RODGERS_QUESTION: &str = "rodgers:question";

/// Label constant for marking bug issues that have been routed to feature-bug workflow.
pub const LABEL_RODGERS_BUG: &str = "rodgers:bug";

/// Label constant for question classification.
pub const LABEL_QUESTION: &str = "question";

/// Check if an issue is a question (has the question label).
pub fn is_question_issue(issue: &TriageIssue) -> bool {
    issue.labels.iter().any(|l| l == LABEL_QUESTION)
}

/// Check if an issue has already been marked as rodgers-question.
pub fn has_rodgers_question_label(issue: &TriageIssue) -> bool {
    issue.labels.iter().any(|l| l == LABEL_RODGERS_QUESTION)
}

/// Check if an issue has already been marked as rodgers-bug.
pub fn has_rodgers_bug_label(issue: &TriageIssue) -> bool {
    issue.labels.iter().any(|l| l == LABEL_RODGERS_BUG)
}

/// Route a classified issue to the appropriate workflow.
///
/// This is the main entry point for the router. It:
/// 1. Determines the issue category from its labels
/// 2. For question issues: applies `rodgers:question` label and returns
///    `needs_question_routing = true`
/// 3. For bug issues: applies `rodgers:bug` label, assesses severity,
///    maps priority, and returns `FeatureBug` workflow
/// 4. For feature issues: returns `FeatureBug` workflow (no severity)
/// 5. For unclassified issues: returns NoAction
///
/// Severity assessment for bugs:
/// - Keywords: crash/data loss/security→critical, broken feature→high,
///   minor issue→medium, cosmetic→low
/// - Security labels (CVE, GHSA, "security") always critical
/// - Human-set severity labels are respected and not overridden
/// - Default severity when no keywords match is medium (conservative)
///
/// Priority mapping from severity:
/// - critical→P1, high→P2, medium→P3, low→P4
///
/// Per the plan, `rodgers:question` is applied BEFORE routing so that
/// subsequent triage runs know this issue has been handled.
pub fn route_issue(issue: &TriageIssue) -> Result<RouteResult> {
    // Check if this is a question issue
    if is_question_issue(issue) {
        return route_question_issue(issue);
    }

    // Check if this is a bug issue (already classified)
    let is_bug = issue.labels.iter().any(|l| l == "bug");
    if is_bug {
        return route_bug_issue(issue);
    }

    // Check if this is a feature issue (already classified)
    let is_feature = issue.labels.iter().any(|l| l == "feature");
    if is_feature {
        return Ok(RouteResult {
            issue_number: issue.number,
            routed: true,
            workflow: Some(Workflow::FeatureBug),
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
            comment_to_post: None,
            needs_question_routing: false,
            severity: None,
            priority: None,
            is_bug_routed: false,
        });
    }

    // Unclassified issue - no routing action needed
    // Classification happens at the triage level, not the router level
    Ok(RouteResult {
        issue_number: issue.number,
        routed: false,
        workflow: None,
        labels_to_add: Vec::new(),
        labels_to_remove: Vec::new(),
        comment_to_post: None,
        needs_question_routing: false,
        severity: None,
        priority: None,
        is_bug_routed: false,
    })
}

/// Route a bug issue to the feature-bug workflow with severity assessment.
///
/// This function:
/// 1. Applies `rodgers:bug` label (if not already present)
/// 2. Assesses severity via keyword detection and security label checks
/// 3. Maps severity to priority (critical→P1, high→P2, medium→P3, low→P4)
/// 4. Returns `FeatureBug` workflow with severity metadata
///
/// Edge cases:
/// - Security issues (CVE, GHSA, security label) always critical regardless of keywords
/// - Must not override existing severity if human-set
/// - LLM severity assessment validated before acting (in production)
/// - Severity affects backport priority (critical/high = priority 1 for backports)
fn route_bug_issue(issue: &TriageIssue) -> Result<RouteResult> {
    let mut labels_to_add: Vec<String> = Vec::new();

    // Apply rodgers:bug label if not already present
    if !has_rodgers_bug_label(issue) {
        labels_to_add.push(LABEL_RODGERS_BUG.to_string());
    }

    // Assess severity from issue content
    let severity_result = assess_severity(&issue.title, &issue.body, &issue.labels);
    let severity_label = severity_to_label(&severity_result.severity);

    // Add severity label
    labels_to_add.push(severity_label);

    // Map severity to priority
    let priority = severity_result.priority.to_string();

    Ok(RouteResult {
        issue_number: issue.number,
        routed: true,
        workflow: Some(Workflow::FeatureBug),
        labels_to_add,
        labels_to_remove: Vec::new(),
        comment_to_post: None,
        needs_question_routing: false,
        severity: Some(severity_result.severity),
        priority: Some(priority),
        is_bug_routed: true,
    })
}

/// Route a question issue to the question-routing workflow.
///
/// This function:
/// 1. Applies `rodgers:question` label (so other runs know it's been handled)
/// 2. Returns routing result indicating question routing is needed
///
/// Per CRIT-6 of the question-routing plan: non-question issues must
/// NOT enter this workflow. This function is only called for issues
/// that have the `question` label.
fn route_question_issue(issue: &TriageIssue) -> Result<RouteResult> {
    let mut labels_to_add = Vec::new();

    // Apply rodgers:question label BEFORE routing
    if !has_rodgers_question_label(issue) {
        labels_to_add.push(LABEL_RODGERS_QUESTION.to_string());
    }

    Ok(RouteResult {
        issue_number: issue.number,
        routed: true,
        workflow: Some(Workflow::QuestionRouting),
        labels_to_add,
        labels_to_remove: Vec::new(),
        comment_to_post: None,
        needs_question_routing: true,
        severity: None,
        priority: None,
        is_bug_routed: false,
    })
}

/// Process a batch of issues, returning routing results for each.
///
/// This is used by the triage loop to batch-process all issues in a single run.
pub fn route_issues(issues: &[TriageIssue]) -> Vec<RouteResult> {
    issues
        .iter()
        .map(|i| {
            route_issue(i).unwrap_or_else(|e| RouteResult {
                issue_number: i.number,
                routed: false,
                workflow: None,
                labels_to_add: Vec::new(),
                labels_to_remove: Vec::new(),
                comment_to_post: Some(format!("Error routing issue: {e}")),
                needs_question_routing: false,
                severity: None,
                priority: None,
                is_bug_routed: false,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triage::{IssueState, Severity};

    fn create_test_issue(labels: Vec<&str>, title: &str, body: &str) -> TriageIssue {
        TriageIssue {
            number: 1,
            title: title.to_string(),
            body: body.to_string(),
            author: "testuser".to_string(),
            labels: labels.into_iter().map(String::from).collect(),
            state: IssueState::Open,
            url: Some("https://github.com/org/repo/issues/1".to_string()),
        }
    }

    // =============================================================================
    // QUESTION ROUTING TESTS (existing behavior preserved)
    // =============================================================================

    #[test]
    fn test_is_question_issue_with_question_label() {
        let issue = create_test_issue(vec!["question"], "Test", "Test");
        assert!(is_question_issue(&issue));
    }

    #[test]
    fn test_is_question_issue_without_question_label() {
        let issue = create_test_issue(vec!["bug"], "Test", "Test");
        assert!(!is_question_issue(&issue));
    }

    #[test]
    fn test_has_rodgers_question_label_present() {
        let issue = create_test_issue(vec!["question", "rodgers:question"], "Test", "Test");
        assert!(has_rodgers_question_label(&issue));
    }

    #[test]
    fn test_route_question_issue_applies_label() {
        let issue = create_test_issue(vec!["question"], "Test", "Test");
        let result = route_issue(&issue).unwrap();

        assert!(result.routed);
        assert_eq!(result.workflow, Some(Workflow::QuestionRouting));
        assert!(result.needs_question_routing);
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string())
        );
        // Question issues should NOT have severity
        assert!(result.severity.is_none());
        assert!(result.priority.is_none());
        assert!(!result.is_bug_routed);
    }

    #[test]
    fn test_route_question_issue_skips_label_if_already_present() {
        let issue = create_test_issue(vec!["question", "rodgers:question"], "Test", "Test");
        let result = route_issue(&issue).unwrap();

        assert!(result.routed);
        assert!(result.needs_question_routing);
        assert!(result.labels_to_add.is_empty());
    }

    // =============================================================================
    // BUG ROUTING TESTS - CRIT-4
    // =============================================================================

    #[test]
    fn test_bug_issue_applies_rodgers_bug_label() {
        // Unit test: Bug issue gets rodgers:bug label
        let issue = create_test_issue(vec!["bug"], "App crashes", "Application crashes on launch");
        let result = route_issue(&issue).unwrap();

        assert!(result.routed);
        assert_eq!(result.workflow, Some(Workflow::FeatureBug));
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_BUG.to_string())
        );
        assert!(result.is_bug_routed);
    }

    #[test]
    fn test_bug_issue_does_not_reapply_rodgers_bug_label() {
        // If rodgers:bug already present, don't add again
        let issue = create_test_issue(
            vec!["bug", "rodgers:bug"],
            "App crashes",
            "Application crashes on launch",
        );
        let result = route_issue(&issue).unwrap();

        assert!(result.routed);
        // Should NOT add rodgers:bug again
        let bug_label_count = result
            .labels_to_add
            .iter()
            .filter(|l| *l == LABEL_RODGERS_BUG)
            .count();
        assert_eq!(bug_label_count, 0);
    }

    #[test]
    fn test_bug_issue_gets_severity_critical_for_crash() {
        // Severity keywords correctly map: crash→critical
        let issue = create_test_issue(
            vec!["bug"],
            "App crashes on startup",
            "The application crashes immediately when launched",
        );
        let result = route_issue(&issue).unwrap();

        assert_eq!(result.severity, Some(Severity::Critical));
        assert_eq!(result.priority, Some("P1".to_string()));
    }

    #[test]
    fn test_bug_issue_gets_severity_for_data_loss() {
        // Severity keywords correctly map: data loss→critical
        let issue = create_test_issue(
            vec!["bug"],
            "Data loss on save",
            "User data is lost when saving documents",
        );
        let result = route_issue(&issue).unwrap();

        assert_eq!(result.severity, Some(Severity::Critical));
        assert_eq!(result.priority, Some("P1".to_string()));
    }

    #[test]
    fn test_bug_issue_gets_severity_for_security() {
        // Severity keywords correctly map: security→critical
        let issue = create_test_issue(
            vec!["bug"],
            "Security vulnerability",
            "Found security vulnerability in authentication",
        );
        let result = route_issue(&issue).unwrap();

        assert_eq!(result.severity, Some(Severity::Critical));
        assert_eq!(result.priority, Some("P1".to_string()));
    }

    #[test]
    fn test_bug_issue_gets_severity_high_for_broken() {
        // Severity keywords correctly map: broken→high
        let issue = create_test_issue(
            vec!["bug"],
            "Broken login feature",
            "The login feature is broken after the update",
        );
        let result = route_issue(&issue).unwrap();

        assert_eq!(result.severity, Some(Severity::High));
        assert_eq!(result.priority, Some("P2".to_string()));
    }

    #[test]
    fn test_bug_issue_gets_severity_medium_for_minor() {
        // Severity keywords correctly map: minor→medium
        let issue = create_test_issue(
            vec!["bug"],
            "Minor validation issue",
            "There is a minor issue with form validation",
        );
        let result = route_issue(&issue).unwrap();

        assert_eq!(result.severity, Some(Severity::Medium));
        assert_eq!(result.priority, Some("P3".to_string()));
    }

    #[test]
    fn test_bug_issue_gets_severity_low_for_cosmetic() {
        // Severity keywords correctly map: cosmetic→low
        let issue = create_test_issue(
            vec!["bug"],
            "Cosmetic fix needed",
            "Button alignment is off by a few pixels",
        );
        let result = route_issue(&issue).unwrap();

        assert_eq!(result.severity, Some(Severity::Low));
        assert_eq!(result.priority, Some("P4".to_string()));
    }

    #[test]
    fn test_security_label_forces_critical() {
        // Security issues (CVE, GHSA, security label) always critical
        let issue = create_test_issue(
            vec!["bug", "security"],
            "Minor UI issue",
            "This is a small UI change",
        );
        let result = route_issue(&issue).unwrap();

        assert_eq!(result.severity, Some(Severity::Critical));
        assert_eq!(result.priority, Some("P1".to_string()));
    }

    #[test]
    fn test_human_set_severity_respected() {
        // Must not override existing severity if human-set
        let issue = create_test_issue(
            vec!["bug", "severity: low"],
            "App crashes sometimes",
            "The app crashes on startup in rare conditions",
        );
        let result = route_issue(&issue).unwrap();

        // Even with "crashes" keyword, human-set severity is respected
        assert_eq!(result.severity, Some(Severity::Low));
        assert_eq!(result.priority, Some("P4".to_string()));
    }

    #[test]
    fn test_priority_mapped_from_severity() {
        // Unit test: Priority mapped correctly from severity

        // Critical → P1
        let critical_issue = create_test_issue(vec!["bug"], "App crashes", "Crash on startup");
        let result = route_issue(&critical_issue).unwrap();
        assert_eq!(result.severity, Some(Severity::Critical));
        assert_eq!(result.priority, Some("P1".to_string()));

        // High → P2
        let high_issue =
            create_test_issue(vec!["bug"], "Feature broken", "Login feature is broken");
        let result = route_issue(&high_issue).unwrap();
        assert_eq!(result.severity, Some(Severity::High));
        assert_eq!(result.priority, Some("P2".to_string()));

        // Medium → P3
        let medium_issue = create_test_issue(vec!["bug"], "Minor issue", "Minor validation bug");
        let result = route_issue(&medium_issue).unwrap();
        assert_eq!(result.severity, Some(Severity::Medium));
        assert_eq!(result.priority, Some("P3".to_string()));

        // Low → P4
        let low_issue = create_test_issue(vec!["bug"], "Cosmetic issue", "Button alignment off");
        let result = route_issue(&low_issue).unwrap();
        assert_eq!(result.severity, Some(Severity::Low));
        assert_eq!(result.priority, Some("P4".to_string()));
    }

    #[test]
    fn test_bug_issue_has_severity_label() {
        // Verify severity label is included in labels_to_add
        let issue = create_test_issue(vec!["bug"], "App crashes", "Crash on startup");
        let result = route_issue(&issue).unwrap();

        assert!(
            result
                .labels_to_add
                .contains(&"severity: critical".to_string())
        );
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_BUG.to_string())
        );
    }

    // =============================================================================
    // FEATURE ROUTING TESTS (no severity)
    // =============================================================================

    #[test]
    fn test_feature_issue_no_severity() {
        // Feature issues should not have severity/priority
        let issue = create_test_issue(vec!["feature"], "New feature", "Request for new feature");
        let result = route_issue(&issue).unwrap();

        assert!(result.routed);
        assert_eq!(result.workflow, Some(Workflow::FeatureBug));
        assert!(result.severity.is_none());
        assert!(result.priority.is_none());
        assert!(!result.is_bug_routed);
        assert!(!result.needs_question_routing);
    }

    // =============================================================================
    // UNCLASSIFIED ISSUE TESTS
    // =============================================================================

    #[test]
    fn test_route_unclassified_issue() {
        let issue = create_test_issue(vec![], "Unclassified", "Not classified yet");
        let result = route_issue(&issue).unwrap();

        assert!(!result.routed);
        assert_eq!(result.workflow, None);
        assert!(!result.needs_question_routing);
        assert!(result.severity.is_none());
        assert!(result.priority.is_none());
        assert!(!result.is_bug_routed);
    }

    // =============================================================================
    // BATCH ROUTING TESTS
    // =============================================================================

    #[test]
    fn test_batch_routing_with_bug_severity() {
        let issues = vec![
            create_test_issue(vec!["question"], "Question?", "Need help"),
            create_test_issue(vec!["bug"], "App crashes", "Crash on startup"),
            create_test_issue(vec!["feature"], "New feature", "Add feature X"),
            create_test_issue(vec!["bug"], "Cosmetic issue", "Button alignment off"),
            create_test_issue(vec![], "Unclassified", "Not classified"),
        ];

        let results = route_issues(&issues);
        assert_eq!(results.len(), 5);

        // Question issue - rodgers:question label, no severity
        assert!(results[0].routed);
        assert_eq!(results[0].workflow, Some(Workflow::QuestionRouting));
        assert!(results[0].needs_question_routing);
        assert!(results[0].severity.is_none());
        assert!(!results[0].is_bug_routed);

        // Bug issue - crash → critical severity, P1
        assert!(results[1].routed);
        assert_eq!(results[1].workflow, Some(Workflow::FeatureBug));
        assert!(results[1].is_bug_routed);
        assert_eq!(results[1].severity, Some(Severity::Critical));
        assert_eq!(results[1].priority, Some("P1".to_string()));

        // Feature issue - no severity
        assert!(results[2].routed);
        assert_eq!(results[2].workflow, Some(Workflow::FeatureBug));
        assert!(results[2].severity.is_none());
        assert!(!results[2].is_bug_routed);

        // Bug issue - cosmetic → low severity, P4
        assert!(results[3].routed);
        assert_eq!(results[3].workflow, Some(Workflow::FeatureBug));
        assert!(results[3].is_bug_routed);
        assert_eq!(results[3].severity, Some(Severity::Low));
        assert_eq!(results[3].priority, Some("P4".to_string()));

        // Unclassified issue - not routed
        assert!(!results[4].routed);
        assert_eq!(results[4].workflow, None);
    }

    #[test]
    fn test_batch_routing_multiple_bugs_different_severities() {
        let issues = vec![
            create_test_issue(vec!["bug"], "App crashes", "Crash"),
            create_test_issue(vec!["bug"], "Login broken", "Feature broken"),
            create_test_issue(vec!["bug"], "Minor warning", "Minor issue"),
            create_test_issue(vec!["bug"], "Typo fix", "Typo in text"),
        ];

        let results = route_issues(&issues);

        assert_eq!(results.len(), 4);
        assert_eq!(results[0].severity, Some(Severity::Critical));
        assert_eq!(results[0].priority, Some("P1".to_string()));
        assert_eq!(results[1].severity, Some(Severity::High));
        assert_eq!(results[1].priority, Some("P2".to_string()));
        assert_eq!(results[2].severity, Some(Severity::Medium));
        assert_eq!(results[2].priority, Some("P3".to_string()));
        assert_eq!(results[3].severity, Some(Severity::Low));
        assert_eq!(results[3].priority, Some("P4".to_string()));
    }

    // =============================================================================
    // HELPER TESTS
    // =============================================================================

    #[test]
    fn test_route_non_question_issue_not_routed_to_question() {
        // Per CRIT-6: non-question issues must NOT enter question workflow
        let issue = create_test_issue(vec!["bug"], "Test", "Test");
        let result = route_issue(&issue).unwrap();

        assert!(result.routed);
        assert_eq!(result.workflow, Some(Workflow::FeatureBug));
        assert!(!result.needs_question_routing);
    }

    #[test]
    fn test_route_bug_with_rodgers_bug_already_present() {
        // If rodgers:bug already present, don't add it again
        let issue = create_test_issue(
            vec!["bug", "rodgers:bug", "needs-information"],
            "App crashes",
            "Crash on startup",
        );
        let result = route_issue(&issue).unwrap();

        assert!(result.routed);
        assert!(result.is_bug_routed);
        assert_eq!(result.workflow, Some(Workflow::FeatureBug));

        // rodgers:bug should NOT be in labels_to_add (already present)
        let bug_count = result
            .labels_to_add
            .iter()
            .filter(|l| *l == LABEL_RODGERS_BUG)
            .count();
        assert_eq!(bug_count, 0);

        // But severity label should still be added
        assert!(
            result
                .labels_to_add
                .contains(&"severity: critical".to_string())
        );
    }

    #[test]
    fn test_bug_route_result_serialization() {
        let issue = create_test_issue(vec!["bug"], "App crashes", "Crash on startup");
        let result = route_issue(&issue).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("FeatureBug"));
        assert!(json.contains("Critical"));
        assert!(json.contains("rodgers:bug"));
    }

    #[test]
    fn test_is_bug_routed_flag() {
        let bug_issue = create_test_issue(vec!["bug"], "Bug", "Bug body");
        let result = route_issue(&bug_issue).unwrap();
        assert!(result.is_bug_routed);

        let feature_issue = create_test_issue(vec!["feature"], "Feature", "Feature body");
        let result = route_issue(&feature_issue).unwrap();
        assert!(!result.is_bug_routed);

        let question_issue = create_test_issue(vec!["question"], "Question", "Question body");
        let result = route_issue(&question_issue).unwrap();
        assert!(!result.is_bug_routed);
    }
}
