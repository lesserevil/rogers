//! Completeness verification for question issues.
//!
//! This module implements the completeness check defined in
//! plans/question-routing-plan.md. It verifies that question issues have
//! all required information fields present before proceeding with doc search.
//!
//! ## Question Requirements
//!
//! A question is complete when all of the following are present:
//! 1. **Question** — The actual question being asked (required to proceed with doc search)
//! 2. **Context** — Context to avoid 循环往返 (back-and-forth loops)
//!
//! ## Integration
//!
//! This module is called by the question router. When completeness is verified:
//! - Proceed with documentation search
//! - Proceed with code search
//! - NO needs-information label
//! - NO additional info requests

use serde::{Deserialize, Serialize};

use crate::templates::mapping::{
    CanonicalField, extract_section_content, is_section_populated,
};

/// Required fields for question completeness.
///
/// Both fields must be present and populated for the question router
/// to proceed with doc/code search.
pub const QUESTION_REQUIRED_FIELDS: &[CanonicalField] = &[
    CanonicalField::Question,
    CanonicalField::Context,
];

/// Result of a question completeness check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuestionCompletenessResult {
    /// Whether the question is complete.
    pub is_complete: bool,
    /// Which specific fields are missing (if any).
    pub missing_fields: Vec<CanonicalField>,
}

impl QuestionCompletenessResult {
    /// Create a complete result (no missing fields).
    pub fn complete() -> Self {
        Self {
            is_complete: true,
            missing_fields: Vec::new(),
        }
    }

    /// Create an incomplete result with the given missing fields.
    pub fn incomplete(fields: Vec<CanonicalField>) -> Self {
        Self {
            is_complete: false,
            missing_fields: fields,
        }
    }

    /// Check if there are any missing fields.
    pub fn has_missing_fields(&self) -> bool {
        !self.missing_fields.is_empty()
    }

    /// Generate a comment requesting the missing fields.
    pub fn to_request_comment(&self) -> Option<String> {
        if self.is_complete {
            return None;
        }

        let mut lines = vec![
            "Hi! To help us answer your question, we need a bit more information:".to_string(),
        ];
        lines.push("".to_string());

        for field in &self.missing_fields {
            lines.push(format!(
                "- **{}**: Please provide this information",
                field.display_name()
            ));
        }

        lines.push("".to_string());
        lines.push(
            "Once this is added, we'll search our docs and code for an answer. Thanks!"
                .to_string(),
        );

        Some(lines.join("\n"))
    }
}

/// Check if a question is complete.
///
/// Scans the issue body for template sections (using standard headings)
/// and verifies each required field is present and populated.
///
/// ## Edge Cases Handled
///
/// - Empty section content → treated as missing
/// - "N/A" without explanation → treated as missing
/// - "N/A: <explanation>" → valid (justified)
/// - Placeholder text like "[example]" → treated as missing
///
/// # Arguments
///
/// * `body` - The issue body text to check
///
/// # Returns
///
/// A `QuestionCompletenessResult` indicating completeness and any missing fields
pub fn check_question_completeness(body: &str) -> QuestionCompletenessResult {
    if body.is_empty() {
        return QuestionCompletenessResult::incomplete(QUESTION_REQUIRED_FIELDS.to_vec());
    }

    let mut missing_fields = Vec::new();

    for field in QUESTION_REQUIRED_FIELDS {
        let heading = field.heading();
        let content = extract_section_content(body, heading);

        match content {
            Some(text) if is_section_populated(&text) => {
                // Field is present and populated
            }
            _ => {
                missing_fields.push(*field);
            }
        }
    }

    if missing_fields.is_empty() {
        QuestionCompletenessResult::complete()
    } else {
        QuestionCompletenessResult::incomplete(missing_fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_question_all_fields_present() {
        let body = r#"
## Question
How do I configure the GitHub token for Rodgers?

## Context
I'm setting up Rodgers for the first time on a self-hosted GitHub Enterprise
instance and the default auth flow doesn't work.
"#;

        let result = check_question_completeness(body);
        assert!(result.is_complete);
        assert!(result.missing_fields.is_empty());
    }

    #[test]
    fn test_question_missing_question() {
        let body = r#"
## Context
I'm having trouble with the setup process.
"#;

        let result = check_question_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Question));
        assert!(
            !result
                .missing_fields
                .contains(&CanonicalField::Context)
        );
    }

    #[test]
    fn test_question_missing_context() {
        let body = r#"
## Question
How do I configure Rodgers?
"#;

        let result = check_question_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Context));
        assert!(
            !result
                .missing_fields
                .contains(&CanonicalField::Question)
        );
    }

    #[test]
    fn test_question_missing_all_fields() {
        let body = "I need help";

        let result = check_question_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 2);
    }

    #[test]
    fn test_question_empty_question_field() {
        let body = r#"
## Question

## Context
I'm new to this project.
"#;

        let result = check_question_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Question));
    }

    #[test]
    fn test_question_empty_context_field() {
        let body = r#"
## Question
How do I use this?

## Context

"#;

        let result = check_question_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Context));
    }

    #[test]
    fn test_question_placeholder_treated_as_missing() {
        let body = r#"
## Question
[type your question here]

## Context
[provide context]
"#;

        let result = check_question_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Question));
        assert!(result.missing_fields.contains(&CanonicalField::Context));
    }

    #[test]
    fn test_question_with_all_2_fields_ready() {
        // CRIT-6: Question with all 2 fields = ready for doc search
        let body = r#"
## Question
What is the recommended way to handle rate limiting in the GitHub API client?

## Context
We're hitting rate limits when running batch operations on a repo with
thousands of issues. Currently using the default polling interval.
"#;

        let result = check_question_completeness(body);
        assert!(
            result.is_complete,
            "Question with all 2 fields should be complete"
        );
        assert!(
            result.missing_fields.is_empty(),
            "No fields should be missing"
        );
        assert!(
            result.to_request_comment().is_none(),
            "Should NOT post a needs-information comment when complete"
        );
    }

    #[test]
    fn test_question_na_with_explanation_is_valid() {
        let body = r#"
## Question
Is there a CLI flag for dry-run mode?

## Context
N/A: This is a straightforward question about existing functionality
"#;

        let result = check_question_completeness(body);
        assert!(result.is_complete);
    }

    #[test]
    fn test_question_empty_body() {
        let result = check_question_completeness("");
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 2);
    }

    #[test]
    fn test_question_completeness_result_methods() {
        let complete = QuestionCompletenessResult::complete();
        assert!(complete.is_complete);
        assert!(!complete.has_missing_fields());

        let incomplete =
            QuestionCompletenessResult::incomplete(vec![CanonicalField::Question]);
        assert!(!incomplete.is_complete);
        assert!(incomplete.has_missing_fields());
    }

    #[test]
    fn test_question_request_comment_for_missing_fields() {
        let body = r#"
## Question
How do I configure auth?
"#;

        let result = check_question_completeness(body);
        let comment = result.to_request_comment();
        assert!(comment.is_some());

        let comment = comment.unwrap();
        assert!(comment.contains("Context"));
        assert!(!comment.contains("Question"));
    }

    #[test]
    fn test_question_no_request_comment_when_complete() {
        let body = r#"
## Question
How do I start?

## Context
First time user
"#;

        let result = check_question_completeness(body);
        assert!(result.to_request_comment().is_none());
    }
}
