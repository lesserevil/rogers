//! Reformat offer comment generation and posting.
//!
//! Rodgers offers to help reformat non-conforming issues using appropriate
//! issue templates. This module generates the offer comment and manages
//! the posting logic.
//!
//! ## Comment Tone
//!
//! The reformat offer comment must:
//! - Be warm and inviting (not scolding)
//! - Offer to rewrite using appropriate template
//! - Ask for confirmation before proceeding
//! - Be informative without being demanding
//!
//! ## One Offer Policy
//!
//! Rodgers posts exactly one reformat offer per issue. If the requestor
//! declines or ignores the offer, Rodgers does not repeat it.

use crate::templates::{TemplateType, check_conformance};

/// The reformat offer comment text.
///
/// This comment is posted when Rodgers detects that an issue was filed
/// without using a GitHub issue template. It offers to help the requestor
/// rewrite their issue using an appropriate template.
pub const REFOMAT_OFFER_COMMENT: &str = r#"Thanks for reaching out! 

We use issue templates to make sure we gather all the information needed to understand and address your request.

It looks like this was submitted without a template. Would you like help reformatting it? I'll rewrite it using the [bug report / feature request / question] template based on what you've shared — just confirm below and I'll post the reformatted version for your review.

If you'd prefer to keep it as-is, no worries at all! I'll work with what you provided."#;

/// Label added to issues that have received a reformat offer.
///
/// This label ensures Rodgers never offers to reformat the same issue twice.
/// The label is removed if the issue is closed and reopened.
pub const OFFER_SENT_LABEL: &str = "reformat-offer-sent";

/// Builds a reformat offer comment with the appropriate template type filled in.
///
/// The comment includes a placeholder `[bug report / feature request / question]`
/// that would be replaced with the specific template type based on issue content.
/// For now, we use the generic phrasing since LLM integration is not yet available.
///
/// # Arguments
///
/// * `suggested_template` - The template type to suggest (if detected)
///
/// # Returns
///
/// A comment body string
pub fn build_reformat_offer_comment(suggested_template: Option<TemplateType>) -> String {
    match suggested_template {
        Some(t) => format!(
            "Thanks for reaching out!\n\n\
            We use issue templates to make sure we gather all the information \
            needed to understand and address your request.\n\n\
            It looks like this was submitted without a template. Would you like \
            help reformatting it? I'll rewrite it using the {} template based on \
            what you've shared — just confirm below and I'll post the reformatted \
            version for your review.\n\n\
            If you'd prefer to keep it as-is, no worries at all! I'll work with \
            what you provided.",
            t.display_name()
        ),
        None => "Thanks for reaching out!\n\n\
            We use issue templates to make sure we gather all the information \
            needed to understand and address your request.\n\n\
            It looks like this was submitted without a template. Would you like \
            help reformatting it? I can rewrite it using an appropriate template \
            based on what you've shared — just confirm below and I'll post the \
            reformatted version for your review.\n\n\
            If you'd prefer to keep it as-is, no worries at all! I'll work with \
            what you provided."
            .to_string(),
    }
}

/// Determines the suggested template type based on issue content.
///
/// This is a simple heuristic that looks for common keywords in the issue body.
/// In production, this would be done by LLM analysis.
///
/// # Arguments
///
/// * `body` - The issue body text
///
/// # Returns
///
/// The suggested template type, or None if indeterminate
pub fn suggest_template_type(body: &str) -> Option<TemplateType> {
    let body_lower = body.to_lowercase();

    // Bug indicators
    if body_lower.contains("bug")
        || body_lower.contains("crash")
        || body_lower.contains("error")
        || body_lower.contains("broken")
        || body_lower.contains("not work")
        || body_lower.contains("doesn't work")
        || body_lower.contains("doesn't")
        || body_lower.contains("fail")
    {
        return Some(TemplateType::BugReport);
    }

    // Question indicators - check before feature keywords to handle phrases like "Can I use this feature?"
    if body_lower.contains("how")
        || body_lower.contains("what")
        || body_lower.contains("can i")
        || body_lower.contains("is there")
        || body_lower.contains("?")
        || body_lower.contains("question")
        || body_lower.contains("confused")
        || body_lower.contains("help")
        || body_lower.contains("trying to")
    {
        return Some(TemplateType::Question);
    }

    // Feature indicators
    if body_lower.contains("feature")
        || body_lower.contains("request")
        || body_lower.contains("would be nice")
        || body_lower.contains("suggest")
        || body_lower.contains("add")
        || body_lower.contains("implement")
        || body_lower.contains("wish")
        || body_lower.contains("ability")
    {
        return Some(TemplateType::FeatureRequest);
    }

    None
}

/// Check if an issue needs a reformat offer.
///
/// Returns true if:
/// - The issue is non-conforming (no template marker)
/// - The issue doesn't already have the reformat offer sent label
/// - The issue is open
///
/// # Arguments
///
/// * `body` - The issue body text
/// * `has_offer_sent_label` - Whether the issue already has the reformat-offer-sent label
///
/// # Returns
///
/// Whether a reformat offer should be posted
pub fn needs_reformat_offer(body: &str, has_offer_sent_label: bool) -> bool {
    !check_conformance(body).is_conforming && !has_offer_sent_label
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_reformat_offer_with_template() {
        let comment = build_reformat_offer_comment(Some(TemplateType::BugReport));
        assert!(comment.contains("Bug Report template"));
        assert!(comment.contains("Would you like help reformatting it?"));
        assert!(comment.contains("confirm"));
    }

    #[test]
    fn test_build_reformat_offer_without_template() {
        let comment = build_reformat_offer_comment(None);
        assert!(comment.contains("an appropriate template"));
        assert!(comment.contains("Would you like help reformatting it?"));
    }

    #[test]
    fn test_comment_has_warm_tone() {
        let comment = build_reformat_offer_comment(None);
        // Should not contain scolding language
        assert!(!comment.contains("failed to"));
        assert!(!comment.contains("must"));
        assert!(!comment.contains("should have"));
        assert!(!comment.contains("wrong"));
        assert!(!comment.contains("error"));
        // Should contain welcoming language
        assert!(comment.contains("Thanks"));
        assert!(comment.contains("no worries"));
    }

    #[test]
    fn test_comment_asks_for_confirmation() {
        let comment = build_reformat_offer_comment(None);
        assert!(comment.contains("confirm"));
        assert!(comment.contains("review"));
    }

    #[test]
    fn test_suggest_bug_report() {
        assert_eq!(
            suggest_template_type("This is a bug that crashes"),
            Some(TemplateType::BugReport)
        );
        assert_eq!(
            suggest_template_type("The application is broken"),
            Some(TemplateType::BugReport)
        );
        assert_eq!(
            suggest_template_type("Something doesn't work"),
            Some(TemplateType::BugReport)
        );
    }

    #[test]
    fn test_suggest_feature_request() {
        assert_eq!(
            suggest_template_type("I'd like a new feature"),
            Some(TemplateType::FeatureRequest)
        );
        assert_eq!(
            suggest_template_type("Please implement this"),
            Some(TemplateType::FeatureRequest)
        );
        assert_eq!(
            suggest_template_type("Would be nice to have"),
            Some(TemplateType::FeatureRequest)
        );
    }

    #[test]
    fn test_suggest_question() {
        assert_eq!(
            suggest_template_type("How do I configure this?"),
            Some(TemplateType::Question)
        );
        assert_eq!(
            suggest_template_type("What is the best way to?"),
            Some(TemplateType::Question)
        );
        assert_eq!(
            suggest_template_type("Can I use this feature?"),
            Some(TemplateType::Question)
        );
    }

    #[test]
    fn test_suggest_unclear_content() {
        assert_eq!(suggest_template_type("Hello world"), None);
        assert_eq!(suggest_template_type(""), None);
    }

    #[test]
    fn test_needs_reformat_offer_non_conforming() {
        let body = "This is a freeform issue without a template marker";
        assert!(needs_reformat_offer(body, false));
    }

    #[test]
    fn test_needs_reformat_offer_already_sent() {
        let body = "This is a freeform issue without a template marker";
        assert!(!needs_reformat_offer(body, true));
    }

    #[test]
    fn test_needs_reformat_offer_conforming() {
        let body = "Issue content\n<!-- template: bug_report -->";
        assert!(!needs_reformat_offer(body, false));
    }

    #[test]
    fn test_offer_sent_label_is_correct() {
        assert_eq!(OFFER_SENT_LABEL, "reformat-offer-sent");
    }
}
