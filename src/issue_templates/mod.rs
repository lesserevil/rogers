//! Issue Templates Workflow - Entry point for handling docs/template issues.
//!
//! Implements plans/issue-templates-plan.md.
//!
//! When the triage router passes a docs issue here, we handle:
//! 1. Missing templates — file a bead with suggested default templates
//! 2. Template field updates — file a bead requesting template revisions
//! 3. Documentation corrections — file a bead for doc updates
//!
//! Template changes require human review (governance decision).
//! Rodgers files a bead but does not auto-commit.

use crate::triage::{IssueState, TriageAction, TriageIssue};

/// Output from the issue-templates workflow.
#[derive(Debug, Clone)]
pub struct IssueTemplatesOutput {
    /// Whether the issue was processed.
    pub processed: bool,
    /// Action taken.
    pub action: TriageAction,
    /// Comment to post (if any).
    pub comment: Option<String>,
    /// Labels to add.
    pub labels_to_add: Vec<String>,
    /// Labels to remove.
    pub labels_to_remove: Vec<String>,
}

/// Process a docs issue through the issue-templates workflow.
///
/// This function is synchronous and completes within one triage run.
/// It handles template improvements and documentation updates for issues
/// classified as 'docs'.
pub fn process_issue_templates(issue: &TriageIssue) -> IssueTemplatesOutput {
    // Guard: only process open docs issues
    if issue.state == IssueState::Closed {
        return IssueTemplatesOutput {
            processed: false,
            action: TriageAction::NoAction,
            comment: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Analyze what kind of docs issue this is and handle accordingly
    classify_and_route(issue)
}

/// Classify a docs issue and determine the appropriate handling.
///
/// Distinguishes between:
/// - Missing templates → file bead with suggested templates
/// - Template field updates → file bead requesting revisions
/// - Documentation corrections → file bead for doc updates
fn classify_and_route(issue: &TriageIssue) -> IssueTemplatesOutput {
    let body_lower = issue.body.to_lowercase();
    let title_lower = issue.title.to_lowercase();
    let combined = format!("{} {}", title_lower, body_lower);

    // Check for missing templates
    if is_missing_template_issue(&combined) {
        return handle_missing_templates(issue);
    }

    // Check for template field updates
    if is_template_field_update(&combined) {
        return handle_template_field_update(issue);
    }

    // Check for documentation corrections
    if is_documentation_correction(&combined) {
        return handle_documentation_correction(issue);
    }

    // Default: file a generic docs bead for any other docs issue
    handle_generic_docs_issue(issue)
}

/// Detect issues about missing issue templates.
///
/// Indicators include: no templates found, template suggested,
/// ".github/ISSUE_TEMPLATE" mentioned, template creation requested.
fn is_missing_template_issue(combined: &str) -> bool {
    let indicators = [
        "no template",
        "missing template",
        "no templates",
        "missing templates",
        "add template",
        "add a template",
        "suggest template",
        "default template",
        "suggested template",
        "auto_suggest",
        "issue template",
    ];

    indicators
        .iter()
        .any(|indicator| combined.contains(indicator))
}

/// Detect issues about updating template fields.
///
/// Indicators include: template field changes, template revision,
/// completeness anchor updates.
fn is_template_field_update(combined: &str) -> bool {
    let indicators = [
        "template change",
        "template revision",
        "template field",
        "template needs",
        "new field in template",
        "update template",
        "edit template",
        "completeness anchor",
    ];

    indicators
        .iter()
        .any(|indicator| combined.contains(indicator))
}

/// Detect issues about documentation corrections.
///
/// Indicators include: doc correction, documentation update,
/// doc gap, outdated documentation.
fn is_documentation_correction(combined: &str) -> bool {
    let indicators = [
        "doc correction",
        "fix documentation",
        "documentation fix",
        "outdated documentation",
        "doc update",
        "documentation gap",
    ];

    indicators
        .iter()
        .any(|indicator| combined.contains(indicator))
}

/// Handle missing templates issue.
///
/// Per issue-templates-plan.md: Rodgers files a bead with suggested
/// default templates. Template changes require human review.
fn handle_missing_templates(issue: &TriageIssue) -> IssueTemplatesOutput {
    let comment = format!(
        r#"Hi @{author}, thanks for flagging this!

We use GitHub issue templates to make sure we gather all the information needed to understand and address requests. It looks like the project is missing issue templates (or some are missing).

I've filed a bead with suggested default templates. A human reviewer will need to review and commit the suggested templates — this is a project governance decision, so they are not applied automatically.

The suggested templates include:
- Bug Report template (with Environment, Steps to Reproduce, Expected/Actual Behavior sections)
- Feature Request template (with Use Case, Proposed Behavior, Acceptance Criteria sections)
- Question template (with Question and Context sections)

Thanks for helping improve the issue filing experience!"#,
        author = issue.author
    );

    IssueTemplatesOutput {
        processed: true,
        action: TriageAction::RoutedToIssueTemplates,
        comment: Some(comment),
        labels_to_add: vec!["needs-documentation".to_string()],
        labels_to_remove: Vec::new(),
    }
}

/// Handle template field update issue.
///
/// Per issue-templates-plan.md: Rodgers detects template changes and
/// files a bead for human review of completeness anchors.
fn handle_template_field_update(issue: &TriageIssue) -> IssueTemplatesOutput {
    let comment = format!(
        r#"Hi @{author}, thanks for the template update suggestion!

I've noted the proposed template field changes. Since template choices are a project governance decision, a human reviewer will need to evaluate whether these changes align with our completeness requirements.

Template fields must map directly to the completeness requirements in our triage workflow, so any changes need careful review to ensure they still cover all required information.

Thanks for helping improve our issue templates!"#,
        author = issue.author
    );

    IssueTemplatesOutput {
        processed: true,
        action: TriageAction::RoutedToIssueTemplates,
        comment: Some(comment),
        labels_to_add: Vec::new(),
        labels_to_remove: Vec::new(),
    }
}

/// Handle documentation correction issue.
///
/// Per issue-templates-plan.md: Documentation corrections are tracked
/// via the issue-templates workflow for systematic addressing.
fn handle_documentation_correction(issue: &TriageIssue) -> IssueTemplatesOutput {
    let comment = format!(
        r#"Hi @{author}, thanks for identifying this documentation issue!

I've routed this to the documentation workflow for tracking. A human reviewer will determine the appropriate action — whether to update existing documentation, create new documentation, or add templates to prevent similar issues.

Documentation gaps from questions route to the question workflow separately, but documentation corrections for templates and filing experience come here.

Thanks for helping keep our documentation in good shape!"#,
        author = issue.author
    );

    IssueTemplatesOutput {
        processed: true,
        action: TriageAction::RoutedToIssueTemplates,
        comment: Some(comment),
        labels_to_add: Vec::new(),
        labels_to_remove: Vec::new(),
    }
}

/// Handle a generic docs issue (catch-all).
///
/// Any docs-labeled issue that doesn't match specific categories
/// gets routed through the issue-templates workflow for tracking.
fn handle_generic_docs_issue(issue: &TriageIssue) -> IssueTemplatesOutput {
    let comment = format!(
        r#"Hi @{author}, thanks for the documentation-related issue!

I've routed this to the issue-templates workflow for systematic tracking. A human reviewer will evaluate the suggested improvements and determine the appropriate action.

Template changes and documentation corrections require human review as they are project governance decisions. Rodgers files a bead to track this but does not auto-commit any changes.

Thanks for helping improve the issue filing experience!"#,
        author = issue.author
    );

    IssueTemplatesOutput {
        processed: true,
        action: TriageAction::RoutedToIssueTemplates,
        comment: Some(comment),
        labels_to_add: Vec::new(),
        labels_to_remove: Vec::new(),
    }
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

    // =============================================================================
    // Missing template detection
    // =============================================================================

    #[test]
    fn test_detects_missing_template_issue() {
        let issue = create_test_issue(
            vec!["docs"],
            "The project has no issue templates. We should add default templates.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        assert!(output.processed);
        assert_eq!(output.action, TriageAction::RoutedToIssueTemplates);
        assert!(
            output
                .comment
                .as_ref()
                .unwrap()
                .contains("missing issue templates")
        );
    }

    #[test]
    fn test_detects_missing_templates_with_suggested() {
        let issue = create_test_issue(
            vec!["docs"],
            "No templates found in .github/ISSUE_TEMPLATE. File a bead with suggested default templates.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        assert!(output.processed);
        assert_eq!(output.action, TriageAction::RoutedToIssueTemplates);
    }

    // =============================================================================
    // Template field update detection
    // =============================================================================

    #[test]
    fn test_detects_template_field_update() {
        let issue = create_test_issue(
            vec!["docs"],
            "The bug report template needs a new field for GPU model.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        assert!(output.processed);
        assert_eq!(output.action, TriageAction::RoutedToIssueTemplates);
        assert!(output.comment.as_ref().unwrap().contains("template field"));
    }

    // =============================================================================
    // Documentation correction detection
    // =============================================================================

    #[test]
    fn test_detects_documentation_correction() {
        let issue = create_test_issue(
            vec!["docs"],
            "The documentation has an outdated API reference that needs correction.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        assert!(output.processed);
        assert_eq!(output.action, TriageAction::RoutedToIssueTemplates);
    }

    // =============================================================================
    // Generic docs issue handling
    // =============================================================================

    #[test]
    fn test_generic_docs_issue_routed() {
        let issue = create_test_issue(
            vec!["docs"],
            "We should improve the contribution guidelines.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        assert!(output.processed);
        assert_eq!(output.action, TriageAction::RoutedToIssueTemplates);
        assert!(
            output
                .comment
                .as_ref()
                .unwrap()
                .contains("issue-templates workflow")
        );
    }

    // =============================================================================
    // Closed issue handling
    // =============================================================================

    #[test]
    fn test_closed_docs_issue_is_noop() {
        let issue = create_test_issue(vec!["docs"], "Missing templates", IssueState::Closed);
        let output = process_issue_templates(&issue);

        assert!(!output.processed);
        assert_eq!(output.action, TriageAction::NoAction);
    }

    // =============================================================================
    // Comment includes author mention
    // =============================================================================

    #[test]
    fn test_missing_templates_comment_mentions_author() {
        let issue = create_test_issue(vec!["docs"], "No templates found.", IssueState::Open);
        let output = process_issue_templates(&issue);

        assert!(output.comment.as_ref().unwrap().contains("@testuser"));
    }

    #[test]
    fn test_template_update_comment_mentions_author() {
        let issue = create_test_issue(
            vec!["docs"],
            "Add a GPU field to templates.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        assert!(output.comment.as_ref().unwrap().contains("@testuser"));
    }

    #[test]
    fn test_doc_correction_comment_mentions_author() {
        let issue = create_test_issue(
            vec!["docs"],
            "Fix outdated documentation.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        assert!(output.comment.as_ref().unwrap().contains("@testuser"));
    }

    // =============================================================================
    // Template changes require human review messaging
    // =============================================================================

    #[test]
    fn test_missing_templates_human_review_message() {
        let issue = create_test_issue(
            vec!["docs"],
            "No templates found. Suggested templates available.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        let comment = output.comment.as_ref().unwrap();
        assert!(
            comment.contains("human reviewer") || comment.contains("governance"),
            "missing templates should mention human review or governance"
        );
    }

    #[test]
    fn test_template_update_human_review_message() {
        let issue = create_test_issue(
            vec!["docs"],
            "Update template fields for completeness.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        let comment = output.comment.as_ref().unwrap();
        assert!(
            comment.contains("human reviewer") || comment.contains("governance"),
            "template updates should mention human review or governance"
        );
    }

    #[test]
    fn test_generic_docs_human_review_message() {
        let issue = create_test_issue(
            vec!["docs"],
            "General docs improvement needed.",
            IssueState::Open,
        );
        let output = process_issue_templates(&issue);

        let comment = output.comment.as_ref().unwrap();
        assert!(
            comment.contains("human reviewer") || comment.contains("governance"),
            "generic docs should mention human review or governance"
        );
    }

    // =============================================================================
    // Edge case: docs issue with no specific category matches → generic handler
    // =============================================================================

    #[test]
    fn test_docs_issue_with_simple_message_goes_to_generic() {
        let issue = create_test_issue(vec!["docs"], "Documentation needs work.", IssueState::Open);
        let output = process_issue_templates(&issue);

        assert!(output.processed);
        assert_eq!(output.action, TriageAction::RoutedToIssueTemplates);
        // Should get the generic message, not a specific category message
        let comment = output.comment.as_ref().unwrap();
        assert!(comment.contains("issue-templates workflow"));
        assert!(!comment.contains("missing issue templates"));
    }
}
