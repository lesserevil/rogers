//! Question Router - handles classified 'question' issues.
//!
//! This module implements the question-routing workflow defined in
//! plans/question-routing-plan.md. When a question issue arrives:
//!
//! 1. Check if Rodgers has already commented (if yes, no-op)
//! 2. Determine if the question can be answered from docs or code
//! 3a. If docs have the answer: post doc link and close
//! 3a-ii. If code has the answer: post code explanation and close
//! 3b. If no answer found: file doc-gap bead, post acknowledgment, label needs-documentation
//!
//! Plan: plans/question-routing-plan.md

use crate::error::Result;
use crate::triage::TriageIssue;
use serde::{Deserialize, Serialize};

/// Label constant for question classification.
pub const LABEL_QUESTION: &str = "question";

/// Label constant for needs-information classification.
pub const LABEL_NEEDS_INFORMATION: &str = "needs-information";

/// Label constant for needs-documentation classification.
pub const LABEL_NEEDS_DOCUMENTATION: &str = "needs-documentation";

/// Label constant marking question issues that have been routed.
pub const LABEL_RODGERS_QUESTION: &str = "rodgers:question";

/// The outcome of routing a question issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionRouterResult {
    /// Issue number processed
    pub issue_number: u64,
    /// Whether the issue was processed
    pub processed: bool,
    /// Action taken
    pub action: QuestionAction,
    /// Comment to post on the issue
    pub comment_to_post: Option<String>,
    /// Labels to add
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
}

/// Actions the question router can take.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuestionAction {
    /// Already handled by Rodgers - no-op
    AlreadyHandled,
    /// Doc search found an answer - posted link comment
    DocFound,
    /// Code search found an answer - posted code explanation
    CodeFound,
    /// No answer found - filed doc-gap bead
    DocGap,
    /// Question too vague - needs clarification
    NeedsClarification,
    /// Question is not a question (bug/feature in disguise)
    Reclassified,
    /// Question issue needs full routing
    NeedsRouting,
}

impl QuestionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlreadyHandled => "already_handled",
            Self::DocFound => "doc_found",
            Self::CodeFound => "code_found",
            Self::DocGap => "doc_gap",
            Self::NeedsClarification => "needs_clarification",
            Self::Reclassified => "reclassified",
            Self::NeedsRouting => "needs_routing",
        }
    }
}

/// Check if Rodgers has already commented on this issue.
///
/// This implements Step 1 of the question routing plan: if Rodgers
/// has already commented on the issue, we skip processing (no-op).
fn has_rodgers_comment(issue: &TriageIssue) -> bool {
    // Heuristic: if the body or any content mentions "Rodgers" or
    // the issue has already been processed (has rodgers:question label),
    // consider it already handled.
    issue.labels.iter().any(|l| l == LABEL_RODGERS_QUESTION)
}

/// Check if a question can be answered from docs or code, or if it
/// needs clarification.
///
/// Step 2 of the question routing plan. Uses keyword heuristics to
/// determine search strategy since we don't have direct LLM access
/// in this module (the triage layer handles LLM classification).
fn classify_question_need(issue: &TriageIssue) -> QuestionClassification {
    let body_lower = issue.body.to_lowercase();
    let title_lower = issue.title.to_lowercase();
    let combined = format!("{} {}", title_lower, body_lower);

    // Check for clarification signals FIRST - vague questions need clarification
    // regardless of whether they also have bug/feature keywords
    let has_clarification_signal = is_vague_question(&combined);

    // Check if this is about code-level/implementation details
    let needs_code_search = asks_about_implementation(&combined);

    // Check if this is a question that should actually be reclassified
    // Only reclassify if it's NOT too vague (vague questions need clarification first)
    let is_reclassified = !has_clarification_signal && looks_like_bug_or_feature(&combined);

    if is_reclassified {
        QuestionClassification::Reclassified
    } else if has_clarification_signal {
        QuestionClassification::NeedsClarification
    } else if needs_code_search {
        QuestionClassification::NeedsCodeSearch
    } else {
        QuestionClassification::NeedsDocSearch
    }
}

/// Classification of what kind of question it is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum QuestionClassification {
    /// Question can be answered from documentation
    NeedsDocSearch,
    /// Question is about implementation details - needs code search
    NeedsCodeSearch,
    /// Question is too vague - needs clarification
    NeedsClarification,
    /// Actually a bug or feature request in disguise
    Reclassified,
}

/// Check if a question is too vague to answer.
///
/// Vague questions that are impossible to answer without clarification:
/// - Very short questions with no specifics
/// - Questions like "how do I..." with no target
/// - Questions missing all context
fn is_vague_question(content: &str) -> bool {
    let trimmed = content.trim();

    // Questions that are too short and lack specific targets
    if trimmed.len() < 30 {
        return true;
    }

    // Check for vague patterns without a specific target
    let vague_patterns = [
        "how do i",
        "how does",
        "what is the",
        "how can i",
        "can you",
        "i have a problem",
        "it doesn't work",
        "not working",
    ];

    let has_vague_pattern = vague_patterns
        .iter()
        .any(|p| content.to_lowercase().contains(p));
    let has_specific_target = [
        "the api",
        "the cli",
        "the docs",
        "the function",
        "the module",
        "the config",
        "how to",
        "how does",
        "explain",
        "describe",
        "what does",
        "what is",
        "difference between",
        "compare",
    ]
    .iter()
    .any(|p| content.to_lowercase().contains(p));

    has_vague_pattern && !has_specific_target
}

/// Check if a question is about implementation/code-level details.
///
/// Keywords from the plan: "how does", "what function", "which module",
/// "internals", "implementation", "source code", "can you walk me through",
/// "flow of", "under the hood"
fn asks_about_implementation(content: &str) -> bool {
    let implementation_keywords = [
        "how does",
        "what function",
        "which module",
        "internals",
        "implementation",
        "source code",
        "walk me through",
        "flow of",
        "under the hood",
        "how is",
        "how was",
        "where is",
        "what handles",
        "how does the",
        "how it works",
        "how it is",
        "what data structure",
        "data structure",
        "what function handles",
        "how the",
    ];

    content.to_lowercase().contains("how does")
        || content.to_lowercase().contains("what function")
        || content.to_lowercase().contains("which module")
        || content.to_lowercase().contains("internals")
        || content.to_lowercase().contains("implementation")
        || content.to_lowercase().contains("source code")
        || content.to_lowercase().contains("walk me through")
        || content.to_lowercase().contains("flow of")
        || content.to_lowercase().contains("under the hood")
        || implementation_keywords
            .iter()
            .any(|k| content.to_lowercase().contains(k))
}

/// Check if a question is actually a bug or feature request in disguise.
///
/// Per the plan: if Rodgers determines the issue is actually a bug report
/// or feature request in disguise, re-label and hand off to the
/// Feature/Bug workflow.
fn looks_like_bug_or_feature(content: &str) -> bool {
    let bug_signals = [
        "it crashes",
        "it fails",
        "error",
        "exception",
        "panic",
        "traceback",
        "broken",
        "bug",
        "defect",
        "not working",
        "doesn't work",
        "is broken",
        "regression",
        "severity",
    ];

    let feature_signals = [
        "add feature",
        "i want",
        "please add",
        "new feature",
        "enhancement",
        "request",
        "would be nice",
        "feature request",
    ];

    let content_lower = content.to_lowercase();
    let has_bug_signal = bug_signals.iter().any(|s| content_lower.contains(s));
    let has_feature_signal = feature_signals.iter().any(|s| content_lower.contains(s));

    // If it has bug/feature signals AND lacks question-specific markers,
    // it's likely misclassified
    let question_markers = [
        "question",
        "what is",
        "how does",
        "how to",
        "explain",
        "describe",
        "understand",
        "difference",
        "vs ",
        "versus",
    ];

    let has_question_marker = question_markers.iter().any(|m| content_lower.contains(m));

    (has_bug_signal || has_feature_signal) && !has_question_marker
}

/// Search documentation for answers to a question.
///
/// Step 2 of the question routing plan: search `docs/**/*.md` for
/// content relevant to the question. Search is keyword-based over
/// the full text of documentation files.
///
/// Returns a list of (file_path, matched_keyword) tuples.
pub fn search_docs(_question_title: &str, _question_body: &str) -> Result<Vec<(String, String)>> {
    // In production, this would scan docs/ recursively.
    // For the router module, we return an empty result (doc search is
    // handled at the triage/processor level with actual file access).
    // The router's job is to coordinate routing, not perform the search.
    //
    // The triage loop layer handles the actual file I/O for doc search.
    Ok(Vec::new())
}

/// Search source code for answers to a question.
///
/// When a question asks about code-level or implementation-level details,
/// search the project source code directly. This covers questions about
/// how X works under the hood, which function handles Y, etc.
///
/// Returns a list of (file_path, function_name, explanation) tuples.
pub fn search_code(
    _question_title: &str,
    _question_body: &str,
) -> Result<Vec<(String, String, String)>> {
    // In production, this would scan source files recursively.
    // For the router module, we return an empty result.
    Ok(Vec::new())
}

/// Generate a doc-found comment for a question.
///
/// Step 3a of the question routing plan: post a comment with a link
/// to the documentation that answers the question.
pub fn generate_doc_found_comment(
    doc_path: &str,
    doc_section: &str,
    summary: &str,
    requestor: &str,
) -> String {
    format!(
        "Hi @{requestor}, thanks for the question!\n\n\
         The answer to your question is covered in {doc_path}#{section}().\n\n\
         {summary}\n\n\
         If this doesn't fully answer your question, please let us know and we will follow up.",
        section = doc_section
    )
}

/// Generate a code-found comment for a question.
///
/// Step 3a-ii of the question routing plan: post a comment with a
/// plain-language explanation of the code, citing relevant files and functions.
pub fn generate_code_found_comment(
    explanation: &str,
    source_file: &str,
    function_name: &str,
    requestor: &str,
) -> String {
    format!(
        "Hi @{requestor}, thanks for this question! I took a look at the source code to find the answer.\n\n\
         {explanation}\n\n\
         Relevant source: {source_file}, {function}\n\n\
         If you'd like to dig further, the full implementation is at {source_file}:{function}.",
        function = function_name
    )
}

/// Generate a doc-gap comment for a question.
///
/// Step 3b of the question routing plan: when no answer is found in
/// docs or code, file a doc-gap bead and post an acknowledgment.
pub fn generate_doc_gap_comment(requestor: &str) -> String {
    format!(
        "Hi @{requestor}, thanks for the question! We do not currently have documentation that answers this. We have opened a task to add an answer to our documentation — it will be linked here when complete."
    )
}

/// Generate a clarification request comment for a vague question.
pub fn generate_clarification_request(issue_title: &str, requestor: &str) -> String {
    format!(
        "Hi @{requestor}, thanks for your question! Could you provide a bit more detail about what you're asking?\n\n\
         Your question about \"{title}\" is a bit vague - we need more context to be able to help.\n\n\
         Please include:\n\
         - What specifically you're trying to understand or accomplish\n\
         - Any relevant context (what you've tried, what you expected)\n\n\
         Once we have more details, we'll be happy to help!",
        title = issue_title
    )
}

/// Generate a reclassification comment for a bug/feature disguised as a question.
pub fn generate_reclassification_comment(
    _old_type: &str,
    new_type: &str,
    requestor: &str,
) -> String {
    format!(
        "Hi @{requestor}, I noticed your issue seems to be a {new_type} rather than a question. I've updated the labels accordingly and routed it to the appropriate workflow.\n\n\
         Thanks for filing this!",
        new_type = new_type
    )
}

/// Route a question issue through the question router.
///
/// This is the main entry point for question routing. It:
/// 1. Checks if Rodgers has already commented (no-op if yes)
/// 2. Classifies the question type
/// 3. Takes appropriate action based on classification
///
/// Per the plan, this must complete within one triage run.
pub fn route_question_issue(issue: &TriageIssue) -> Result<QuestionRouterResult> {
    // Step 1: Check if Rodgers already handled this
    if has_rodgers_comment(issue) {
        return Ok(QuestionRouterResult {
            issue_number: issue.number,
            processed: false,
            action: QuestionAction::AlreadyHandled,
            comment_to_post: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        });
    }

    // Step 2: Classify the question type
    let classification = classify_question_need(issue);

    match classification {
        QuestionClassification::NeedsDocSearch => {
            // Route to doc search - this is handled by the caller
            // by invoking the actual doc search mechanism
            Ok(QuestionRouterResult {
                issue_number: issue.number,
                processed: true,
                action: QuestionAction::NeedsRouting,
                comment_to_post: None,
                labels_to_add: Vec::new(),
                labels_to_remove: Vec::new(),
            })
        }
        QuestionClassification::NeedsCodeSearch => {
            // Route to code search - this is handled by the caller
            Ok(QuestionRouterResult {
                issue_number: issue.number,
                processed: true,
                action: QuestionAction::NeedsRouting,
                comment_to_post: None,
                labels_to_add: Vec::new(),
                labels_to_remove: Vec::new(),
            })
        }
        QuestionClassification::NeedsClarification => {
            // Post clarification request and apply needs-information
            Ok(QuestionRouterResult {
                issue_number: issue.number,
                processed: true,
                action: QuestionAction::NeedsClarification,
                comment_to_post: Some(generate_clarification_request(&issue.title, &issue.author)),
                labels_to_add: vec![
                    LABEL_NEEDS_INFORMATION.to_string(),
                    LABEL_RODGERS_QUESTION.to_string(),
                ],
                labels_to_remove: Vec::new(),
            })
        }
        QuestionClassification::Reclassified => {
            // Determine if it looks more like a bug or feature
            let new_type = if looks_like_bug_or_feature_as_bug(&issue.body) {
                "bug"
            } else {
                "feature"
            };

            Ok(QuestionRouterResult {
                issue_number: issue.number,
                processed: true,
                action: QuestionAction::Reclassified,
                comment_to_post: Some(generate_reclassification_comment(
                    "question",
                    new_type,
                    &issue.author,
                )),
                labels_to_add: vec![LABEL_RODGERS_QUESTION.to_string(), new_type.to_string()],
                labels_to_remove: vec![LABEL_QUESTION.to_string()],
            })
        }
    }
}

/// Helper to determine if a reclassified issue is a bug or feature.
fn looks_like_bug_or_feature_as_bug(content: &str) -> bool {
    let bug_keywords = [
        "crash",
        "error",
        "exception",
        "panic",
        "broken",
        "fails",
        "doesn't work",
        "not working",
        "regression",
        "unexpected",
        "wrong",
        "incorrect",
    ];

    let content_lower = content.to_lowercase();
    bug_keywords.iter().any(|k| content_lower.contains(k))
}

/// Process a batch of question issues.
pub fn route_questions(issues: &[TriageIssue]) -> Vec<QuestionRouterResult> {
    issues
        .iter()
        .map(|i| {
            route_question_issue(i).unwrap_or_else(|e| QuestionRouterResult {
                issue_number: i.number,
                processed: false,
                action: QuestionAction::AlreadyHandled,
                comment_to_post: Some(format!("Error routing question: {e}")),
                labels_to_add: Vec::new(),
                labels_to_remove: Vec::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triage::IssueState;

    fn create_test_issue(title: &str, body: &str, labels: Vec<&str>) -> TriageIssue {
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
    // is_vague_question tests
    // =============================================================================

    #[test]
    fn test_vague_question_too_short() {
        assert!(is_vague_question("help"));
        assert!(is_vague_question("how does it work"));
    }

    #[test]
    fn test_vague_question_has_vague_pattern_no_target() {
        let content = "how do i make this work the way i want it to";
        assert!(is_vague_question(content));
    }

    #[test]
    fn test_not_vague_with_specific_target() {
        let content = "how does the API authentication work";
        assert!(!is_vague_question(content));
    }

    #[test]
    fn test_not_vague_with_question_markers() {
        let content = "how does this work - what does this function do";
        assert!(!is_vague_question(content));
    }

    // =============================================================================
    // asks_about_implementation tests
    // =============================================================================

    #[test]
    fn test_asks_about_implementation_how_does() {
        assert!(asks_about_implementation("how does the system work"));
    }

    #[test]
    fn test_asks_about_implementation_what_function() {
        assert!(asks_about_implementation("what function handles requests"));
    }

    #[test]
    fn test_asks_about_implementation_internals() {
        assert!(asks_about_implementation(
            "what are the internals of this module"
        ));
    }

    #[test]
    fn test_asks_about_implementation_under_the_hood() {
        assert!(asks_about_implementation(
            "what happens under the hood when"
        ));
    }

    #[test]
    fn test_not_implementation_question() {
        assert!(!asks_about_implementation("how do I configure the tool"));
    }

    // =============================================================================
    // looks_like_bug_or_feature tests
    // =============================================================================

    #[test]
    fn test_looks_like_bug() {
        assert!(looks_like_bug_or_feature("it crashes when clicking submit"));
    }

    #[test]
    fn test_looks_like_feature() {
        assert!(looks_like_bug_or_feature("add feature to export data"));
    }

    #[test]
    fn test_not_reclassified_with_question_marker() {
        let content = "how does it work - is this a bug";
        assert!(!looks_like_bug_or_feature(content));
    }

    // =============================================================================
    // classify_question_need tests
    // =============================================================================

    #[test]
    fn test_classify_doc_search() {
        let issue = create_test_issue(
            "How to configure",
            "how do I configure the database connection",
            vec!["question"],
        );
        let classification = classify_question_need(&issue);
        assert_eq!(classification, QuestionClassification::NeedsDocSearch);
    }

    #[test]
    fn test_classify_code_search() {
        let issue = create_test_issue(
            "How does auth work",
            "how does the authentication module work under the hood",
            vec!["question"],
        );
        let classification = classify_question_need(&issue);
        assert_eq!(classification, QuestionClassification::NeedsCodeSearch);
    }

    #[test]
    fn test_classify_needs_clarification() {
        let issue = create_test_issue("help", "it doesn't work", vec!["question"]);
        let classification = classify_question_need(&issue);
        assert_eq!(classification, QuestionClassification::NeedsClarification);
    }

    #[test]
    fn test_classify_reclassified_bug() {
        let issue = create_test_issue(
            "crash report",
            "it crashes when I click the button",
            vec!["question"],
        );
        let classification = classify_question_need(&issue);
        assert_eq!(classification, QuestionClassification::Reclassified);
    }

    // =============================================================================
    // route_question_issue tests
    // =============================================================================

    #[test]
    fn test_route_question_already_handled() {
        let issue = create_test_issue(
            "Test question",
            "How do I configure?",
            vec!["question", "rodgers:question"],
        );
        let result = route_question_issue(&issue).unwrap();

        assert_eq!(result.action, QuestionAction::AlreadyHandled);
        assert!(!result.processed);
        assert!(result.comment_to_post.is_none());
    }

    #[test]
    fn test_route_question_needs_clarification() {
        let issue = create_test_issue("help", "it doesn't work", vec!["question"]);
        let result = route_question_issue(&issue).unwrap();

        assert_eq!(result.action, QuestionAction::NeedsClarification);
        assert!(result.processed);
        assert!(result.comment_to_post.is_some());
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_NEEDS_INFORMATION.to_string())
        );
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string())
        );
    }

    #[test]
    fn test_route_question_reclassified() {
        let issue = create_test_issue(
            "crash bug",
            "it crashes when clicking submit",
            vec!["question"],
        );
        let result = route_question_issue(&issue).unwrap();

        assert_eq!(result.action, QuestionAction::Reclassified);
        assert!(result.processed);
        assert!(result.comment_to_post.is_some());
        assert!(result.labels_to_add.contains(&"bug".to_string()));
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string())
        );
        assert!(
            result
                .labels_to_remove
                .contains(&LABEL_QUESTION.to_string())
        );
    }

    #[test]
    fn test_route_question_needs_routing_doc_search() {
        let issue = create_test_issue(
            "How to configure",
            "how do I configure the database connection",
            vec!["question"],
        );
        let result = route_question_issue(&issue).unwrap();

        assert_eq!(result.action, QuestionAction::NeedsRouting);
        assert!(result.processed);
    }

    #[test]
    fn test_route_question_needs_routing_code_search() {
        let issue = create_test_issue(
            "How does auth work",
            "how does the authentication module work under the hood",
            vec!["question"],
        );
        let result = route_question_issue(&issue).unwrap();

        assert_eq!(result.action, QuestionAction::NeedsRouting);
        assert!(result.processed);
    }

    // =============================================================================
    // Comment generation tests
    // =============================================================================

    #[test]
    fn test_generate_doc_found_comment() {
        let comment = generate_doc_found_comment(
            "docs/configuration.md",
            "database",
            "This section covers database connection configuration.",
            "user1",
        );

        assert!(comment.contains("user1"));
        assert!(comment.contains("docs/configuration.md"));
        assert!(comment.contains("database"));
    }

    #[test]
    fn test_generate_code_found_comment() {
        let comment = generate_code_found_comment(
            "The authentication is handled by the auth module which checks JWT tokens.",
            "src/auth/mod.rs",
            "authenticate",
            "user1",
        );

        assert!(comment.contains("user1"));
        assert!(comment.contains("src/auth/mod.rs"));
        assert!(comment.contains("authenticate"));
    }

    #[test]
    fn test_generate_doc_gap_comment() {
        let comment = generate_doc_gap_comment("user1");

        assert!(comment.contains("user1"));
        assert!(comment.contains("documentation"));
        assert!(comment.contains("opened a task"));
    }

    #[test]
    fn test_generate_clarification_request() {
        let comment = generate_clarification_request("Test question", "user1");

        assert!(comment.contains("user1"));
        assert!(comment.contains("Test question"));
        assert!(comment.contains("more detail"));
    }

    #[test]
    fn test_generate_reclassification_comment() {
        let comment = generate_reclassification_comment("question", "bug", "user1");

        assert!(comment.contains("user1"));
        assert!(comment.contains("bug"));
        assert!(comment.contains("routed"));
    }

    // =============================================================================
    // Batch routing tests
    // =============================================================================

    #[test]
    fn test_batch_route_questions() {
        let issues = vec![
            create_test_issue("vague", "it doesn't work", vec!["question"]),
            create_test_issue(
                "already handled",
                "How do I configure?",
                vec!["question", "rodgers:question"],
            ),
            create_test_issue("actually a bug", "it crashes on submit", vec!["question"]),
        ];

        let results = route_questions(&issues);
        assert_eq!(results.len(), 3);

        // First: needs clarification
        assert_eq!(results[0].action, QuestionAction::NeedsClarification);
        assert!(results[0].processed);

        // Second: already handled
        assert_eq!(results[1].action, QuestionAction::AlreadyHandled);
        assert!(!results[1].processed);

        // Third: reclassified as bug
        assert_eq!(results[2].action, QuestionAction::Reclassified);
        assert!(results[2].processed);
    }

    // =============================================================================
    // Search function tests
    // =============================================================================

    #[test]
    fn test_search_docs_returns_empty() {
        let result = search_docs("how to configure", "config question");
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_search_code_returns_empty() {
        let result = search_code("how does auth work", "auth question");
        assert!(result.unwrap().is_empty());
    }

    // =============================================================================
    // Label constant tests
    // =============================================================================

    #[test]
    fn test_label_constants() {
        assert_eq!(LABEL_QUESTION, "question");
        assert_eq!(LABEL_NEEDS_INFORMATION, "needs-information");
        assert_eq!(LABEL_NEEDS_DOCUMENTATION, "needs-documentation");
        assert_eq!(LABEL_RODGERS_QUESTION, "rodgers:question");
    }
}
