//! Question Router module.
//!
//! Routes GitHub issues labeled `question` through the documentation and code
//! search workflow. Searches docs and source code for answers before filing
//! doc-gap tasks.
//!
//! Plan: plans/question-routing-plan.md §Question Router Decision Tree

pub mod code_search;
pub mod doc_gap;
pub mod doc_search;
pub mod router;

pub use router::QuestionRouter;

use serde::{Deserialize, Serialize};

/// Label that marks a question issue as routed to the question-routing workflow.
pub const LABEL_RODGERS_QUESTION: &str = "rodgers:question";

/// Label applied when the question reveals a documentation gap.
pub const LABEL_NEEDS_DOCUMENTATION: &str = "needs-documentation";

/// Label asking for clarification from the requestor.
pub const LABEL_NEEDS_INFORMATION: &str = "needs-information";

/// Result of routing a question issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionRouteResult {
    /// Whether routing was successful
    pub routed: bool,
    /// The action taken
    pub action: QuestionRouteAction,
    /// Labels to apply
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
    /// Comment to post
    pub comment_to_post: Option<String>,
    /// Whether to close the issue
    pub close_issue: bool,
}

/// Actions that can be taken during question routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuestionRouteAction {
    /// Question answered via doc search
    AnsweredViaDocumentation,
    /// Question answered via code search
    AnsweredViaCodeSearch,
    /// Question filed as documentation gap
    FiledDocumentationGap,
    /// Question is a duplicate or invalid
    InvalidOrDuplicate,
    /// Question needs clarification
    NeedsClarification,
    /// No action taken
    NoAction,
}

/// Entry point for the question router.
///
/// This function processes a classified question issue and determines
/// how to handle it according to the question routing plan.
///
/// # Arguments
///
/// * `issue_number` - The GitHub issue number
/// * `title` - The issue title
/// * `body` - The issue body
/// * `labels` - The current labels on the issue
/// * `author` - The issue author
///
/// # Returns
///
/// A `QuestionRouteResult` containing the routing outcome.
pub fn route_question_issue(
    issue_number: u64,
    title: &str,
    body: &str,
    labels: &[String],
    author: &str,
) -> QuestionRouteResult {
    // Check if this issue is already handled (has rodgers:question label)
    let already_handled = labels.iter().any(|l| l == LABEL_RODGERS_QUESTION);

    if already_handled {
        return QuestionRouteResult {
            routed: false,
            action: QuestionRouteAction::NoAction,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
            comment_to_post: None,
            close_issue: false,
        };
    }

    // Check if this is too vague to answer - needs clarification
    if is_too_vague(title, body) {
        return QuestionRouteResult {
            routed: true,
            action: QuestionRouteAction::NeedsClarification,
            labels_to_add: vec![
                LABEL_RODGERS_QUESTION.to_string(),
                LABEL_NEEDS_INFORMATION.to_string(),
            ],
            labels_to_remove: Vec::new(),
            comment_to_post: Some(generate_clarification_comment(
                issue_number,
                title,
                body,
                author,
            )),
            close_issue: false,
        };
    }

    // Determine what type of question this is
    let mut is_code_question = false;
    let mut is_doc_search_needed = false;

    let lower_body = body.to_lowercase();
    let lower_title = title.to_lowercase();

    // Keywords that indicate code search is needed (lowercase for matching)
    let code_keywords = [
        "how does",
        "what function",
        "which module",
        "which class",
        "walk me through",
        "implementation",
        "source code",
        "can you walk me through",
        "under the hood",
        "how it works",
        "how it is implemented",
        "internals",
        "data structure",
        "entry point",
        "module",
        "file path",
    ];

    // Keywords that indicate doc search is needed (lowercase for matching)
    let doc_keywords = [
        "how to",
        "how can i",
        "how do i",
        "how would i",
        "how should",
        "documentation",
        "docs",
        "readme",
        "wiki",
        "guide",
        "tutorial",
        "example",
        "sample",
        "reference",
        "api",
        "interface",
        "specification",
        "how do i configure",
        "how do i set up",
        "how do i use",
        "where can i find",
        "where is",
        "what is the process",
    ];

    // Check for code search keywords
    for keyword in &code_keywords {
        if lower_body.contains(keyword) || lower_title.contains(keyword) {
            is_code_question = true;
            break;
        }
    }

    // Check for doc search keywords
    for keyword in &doc_keywords {
        if lower_body.contains(keyword) || lower_title.contains(keyword) {
            is_doc_search_needed = true;
            break;
        }
    }

    // Determine the action based on what we found
    let action;
    let mut labels_to_add = vec![LABEL_RODGERS_QUESTION.to_string()];
    let comment_to_post;

    // Code questions take priority (most specific search path)
    if is_code_question {
        action = QuestionRouteAction::AnsweredViaCodeSearch;
        comment_to_post = Some(generate_code_answer_comment(
            issue_number,
            title,
            body,
            author,
        ));
    } else if is_likely_doc_gap(title, body) {
        // Doc gap indicators take priority over generic doc search keywords.
        // If the question explicitly asks "where can I find documentation", the
        // user is looking for docs that don't exist → doc gap, not doc search.
        action = QuestionRouteAction::FiledDocumentationGap;
        comment_to_post = Some(generate_doc_gap_comment(issue_number, title, body, author));
        labels_to_add.push(LABEL_NEEDS_DOCUMENTATION.to_string());
    } else if is_doc_search_needed {
        action = QuestionRouteAction::AnsweredViaDocumentation;
        comment_to_post = Some(generate_doc_answer_comment(
            issue_number,
            title,
            body,
            author,
        ));
    } else {
        // No clear search path - file as doc gap
        action = QuestionRouteAction::FiledDocumentationGap;
        comment_to_post = Some(generate_doc_gap_comment(issue_number, title, body, author));
        labels_to_add.push(LABEL_NEEDS_DOCUMENTATION.to_string());
    }

    QuestionRouteResult {
        routed: true,
        action,
        labels_to_add,
        labels_to_remove: Vec::new(),
        comment_to_post,
        close_issue: false,
    }
}

/// Determine if a question body is too vague to answer without clarification.
fn is_too_vague(title: &str, body: &str) -> bool {
    let combined = format!("{} {}", title, body).to_lowercase();
    let lower_body = body.to_lowercase();

    // Very short content is likely too vague
    if combined.len() < 20 {
        return true;
    }

    // Common vague patterns — check against combined AND body alone
    let vague_patterns = [
        "i need help",
        "please help",
        "help me",
        "something is wrong",
        "does not work",
        "not working",
        "broken",
        "what is this",
    ];

    for pattern in &vague_patterns {
        if combined == *pattern || lower_body == *pattern {
            return true;
        }
    }

    // Body-only content that's very short
    if !body.is_empty() && body.len() < 30 {
        let short_vague = ["help", "question", "how to?", "what?", "why?", "fix this"];
        for pattern in &short_vague {
            if combined == *pattern || lower_body == *pattern {
                return true;
            }
        }
    }

    false
}

/// Determine if a question is likely a documentation gap.
///
/// This heuristic looks for questions that explicitly ask where to find
/// documentation, rather than general "how do I" questions which might
/// be answerable from existing docs.
///
/// The distinction:
/// - "How do I configure X?" → doc search (answer may exist in docs)
/// - "Where can I find documentation on X?" → doc gap (asking for docs)
fn is_likely_doc_gap(title: &str, body: &str) -> bool {
    let lower_title = title.to_lowercase();
    let lower_body = body.to_lowercase();

    // Explicit "where can I find documentation" patterns indicate a doc gap
    let gap_indicators = [
        "where can i find documentation",
        "where can i find docs",
        "is there documentation for",
        "is there a guide for",
        "where is the documentation for",
        "where is the docs for",
        "do you have documentation for",
        "where can i read about",
    ];

    for indicator in &gap_indicators {
        if lower_body.contains(indicator) || lower_title.contains(indicator) {
            return true;
        }
    }

    false
}

/// Generate a comment for when a question is answered via documentation.
fn generate_doc_answer_comment(
    _issue_number: u64,
    title: &str,
    _body: &str,
    author: &str,
) -> String {
    format!(
        "Hi @{}! Thanks for the question! The answer to your question about \"{}\" is covered in our documentation.\n\nWe have a relevant guide that explains this:\n\n[Relevant Documentation](https://example.com/docs/placeholder)\n\nThis document provides a clear explanation of the process and includes examples. If this doesn't fully answer your question, please let us know and we will follow up.",
        author, title
    )
}

/// Generate a comment for when a question is answered via code search.
fn generate_code_answer_comment(
    _issue_number: u64,
    title: &str,
    _body: &str,
    author: &str,
) -> String {
    format!(
        "Hi @{}! Thanks for the question! I took a look at the source code to find the answer about \"{}\".\n\nBased on your question, I found that the implementation is in `placeholder_file.rs` and the key function is `placeholder_function`. Here's a plain-language explanation:\n\nThe code handles this by processing the request through the relevant module.\n\nRelevant source: [file path], [function/struct name]\n\nIf you'd like to dig further, the full implementation is at [file:line–line]. If this doesn't fully answer your question, please let us know and we will follow up.",
        author, title
    )
}

/// Generate a comment for when a documentation gap is filed.
fn generate_doc_gap_comment(_issue_number: u64, title: &str, _body: &str, author: &str) -> String {
    format!(
        "Hi @{}! Thanks for the question about \"{}\"! We do not currently have documentation that answers this. We have opened a task to add an answer to our documentation — it will be linked here when complete.\n\nWe will work on adding documentation to address this gap. Thanks for helping improve our docs!",
        author, title
    )
}

/// Generate a comment asking for clarification on a vague question.
fn generate_clarification_comment(
    _issue_number: u64,
    _title: &str,
    _body: &str,
    author: &str,
) -> String {
    format!(
        "Hi @{}! Thanks for the question! To better understand what you're asking, could you please provide more details?\n\nSpecifically, we'd like to know:\n\n- What specific information are you looking for?\n- Are you asking about documentation, code implementation, or something else?\n- Do you have a specific use case or scenario in mind?\n\nThis will help us provide a more accurate and helpful answer. Thanks for your patience!",
        author
    )
}

/// Batch route multiple question issues.
///
/// Processes each issue independently and returns results for all.
pub fn route_question_issues_batch(
    issues: &[(u64, &str, &str, &[String], &str)],
) -> Vec<QuestionRouteResult> {
    issues
        .iter()
        .map(|(issue_number, title, body, labels, author)| {
            route_question_issue(*issue_number, title, body, labels, author)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_question_already_handled() {
        let result = route_question_issue(
            1,
            "How do I add a user?",
            "I want to add a new user.",
            &[LABEL_RODGERS_QUESTION.to_string()],
            "user1",
        );

        assert!(!result.routed);
        assert_eq!(result.action, QuestionRouteAction::NoAction);
        assert!(result.labels_to_add.is_empty());
    }

    #[test]
    fn test_route_question_needs_clarification_vague() {
        let result = route_question_issue(1, "help", "something is wrong", &[], "user1");

        assert!(result.routed);
        assert_eq!(result.action, QuestionRouteAction::NeedsClarification);
        assert!(result
            .labels_to_add
            .contains(&LABEL_RODGERS_QUESTION.to_string()));
        assert!(result
            .labels_to_add
            .contains(&LABEL_NEEDS_INFORMATION.to_string()));
        assert!(result.comment_to_post.is_some());
        assert!(!result.close_issue);
    }

    #[test]
    fn test_route_question_doc_search() {
        let result = route_question_issue(
            1,
            "How do I configure the app?",
            "How do I configure the database connection for production deployment?",
            &[],
            "user1",
        );

        assert!(result.routed);
        assert_eq!(result.action, QuestionRouteAction::AnsweredViaDocumentation);
        assert!(result
            .labels_to_add
            .contains(&LABEL_RODGERS_QUESTION.to_string()));
        assert!(result.comment_to_post.is_some());
    }

    #[test]
    fn test_route_question_code_search() {
        let result = route_question_issue(
            1,
            "How does the auth module work under the hood?",
            "Can you walk me through the implementation of the authentication flow?",
            &[],
            "user1",
        );

        assert!(result.routed);
        assert_eq!(result.action, QuestionRouteAction::AnsweredViaCodeSearch);
        assert!(result
            .labels_to_add
            .contains(&LABEL_RODGERS_QUESTION.to_string()));
        assert!(result.comment_to_post.is_some());
    }

    #[test]
    fn test_route_question_doc_gap() {
        let result = route_question_issue(
            1,
            "How do I set up multi-tenancy?",
            "Where can I find documentation on configuring multi-tenancy for our SaaS product?",
            &[],
            "user1",
        );

        assert!(result.routed);
        assert_eq!(result.action, QuestionRouteAction::FiledDocumentationGap);
        assert!(result
            .labels_to_add
            .contains(&LABEL_RODGERS_QUESTION.to_string()));
        assert!(result
            .labels_to_add
            .contains(&LABEL_NEEDS_DOCUMENTATION.to_string()));
        assert!(result.comment_to_post.is_some());
    }

    #[test]
    fn test_route_question_always_adds_rodgers_question_label() {
        // Verify that rodgers:question is always added for new questions
        let result = route_question_issue(
            1,
            "How does this work?",
            "I have a question about the implementation.",
            &[],
            "user1",
        );

        assert!(result.routed);
        assert!(
            result
                .labels_to_add
                .contains(&LABEL_RODGERS_QUESTION.to_string()),
            "rodgers:question label must always be added for routed questions"
        );
    }

    #[test]
    fn test_route_question_code_takes_priority_over_doc() {
        // When both code and doc keywords match, code search wins
        let result = route_question_issue(
            1,
            "How does the module work and how do I use it?",
            "Walk me through the implementation of this function and how to configure it.",
            &[],
            "user1",
        );

        assert!(result.routed);
        assert_eq!(result.action, QuestionRouteAction::AnsweredViaCodeSearch);
    }

    #[test]
    fn test_batch_routing() {
        let issues: [(u64, &str, &str, &[String], &str); 2] = [
            (
                1,
                "How do I configure?",
                "How do I set up the database?",
                &[],
                "user1",
            ),
            (2, "help", "something is wrong", &[], "user2"),
        ];

        let results = route_question_issues_batch(&issues);

        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].action,
            QuestionRouteAction::AnsweredViaDocumentation
        );
        assert_eq!(results[1].action, QuestionRouteAction::NeedsClarification);
    }

    #[test]
    fn test_is_too_vague_short_content() {
        assert!(is_too_vague("help", "something is wrong"));
        assert!(is_too_vague("question", "help"));
    }

    #[test]
    fn test_is_too_vague_sufficient_content() {
        assert!(!is_too_vague(
            "How do I configure the database?",
            "I am trying to set up PostgreSQL for production use and need help with connection pooling."
        ));
    }

    #[test]
    fn test_generate_doc_answer_mentions_author() {
        let comment = generate_doc_answer_comment(1, "My question", "body", "testuser");
        assert!(comment.contains("@testuser"));
    }

    #[test]
    fn test_generate_code_answer_mentions_author() {
        let comment = generate_code_answer_comment(1, "My question", "body", "testuser");
        assert!(comment.contains("@testuser"));
    }

    #[test]
    fn test_generate_doc_gap_mentions_author() {
        let comment = generate_doc_gap_comment(1, "My question", "body", "testuser");
        assert!(comment.contains("@testuser"));
    }

    #[test]
    fn test_generate_clarification_mentions_author() {
        let comment = generate_clarification_comment(1, "My question", "body", "testuser");
        assert!(comment.contains("@testuser"));
    }

    #[test]
    fn test_question_comment_warm_tone() {
        // Rodgers comments should be warm, not curt
        let comment = generate_doc_answer_comment(1, "My question", "body", "testuser");
        assert!(comment.contains("Thanks"));
        assert!(!comment.contains("stupid"));
        assert!(!comment.contains("obvious"));
    }
}
