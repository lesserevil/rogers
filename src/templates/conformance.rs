//! Conformance detection for GitHub issues.
//!
//! This module detects issues that were filed without using a GitHub issue template
//! by checking for a template marker (hidden comment) in the issue body.
//!
//! ## Detection Logic
//!
//! An issue is **non-conforming** when:
//! - The template marker is absent from the issue body
//! - The issue appears to be a GitHub email reply (detected by sender pattern)
//!
//! ## Template Marker
//!
//! GitHub issue templates use YAML frontmatter with a `name:` field. Rodgers embeds
//! a hidden marker comment like `<!-- template: bug_report -->` at the end of each
//! template. Issues filed using a template will contain this marker; freeform
//! submissions will not.

use serde::{Deserialize, Serialize};

/// Template types that Rodgers recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateType {
    BugReport,
    FeatureRequest,
    Question,
}

impl TemplateType {
    /// Returns the display name for this template type.
    pub fn display_name(&self) -> &'static str {
        match self {
            TemplateType::BugReport => "Bug Report",
            TemplateType::FeatureRequest => "Feature Request",
            TemplateType::Question => "Question",
        }
    }

    /// Returns the template marker for this template type.
    pub fn marker(&self) -> &'static str {
        match self {
            TemplateType::BugReport => "<!-- template: bug_report -->",
            TemplateType::FeatureRequest => "<!-- template: feature_request -->",
            TemplateType::Question => "<!-- template: question -->",
        }
    }
}

/// Result of conformance checking for an issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceResult {
    /// Whether the issue conforms to a template.
    pub is_conforming: bool,
    /// The detected template type, if conforming.
    pub template_type: Option<TemplateType>,
}

/// Marker comments placed at the end of default templates.
///
/// These markers allow Rodgers to detect whether an issue was filed
/// using a template or submitted as freeform text.
pub const TEMPLATE_MARKERS: &[(&str, TemplateType)] = &[
    ("<!-- template: bug_report -->", TemplateType::BugReport),
    ("<!-- template: feature_request -->", TemplateType::FeatureRequest),
    ("<!-- template: question -->", TemplateType::Question),
];

/// Email reply patterns that indicate an issue was submitted via GitHub email.
///
/// GitHub email replies typically include a prefix in the body that identifies
/// them as automated responses rather than original submissions.
pub const EMAIL_REPLY_PATTERNS: &[&str] = &[
    "GitHub Email Reply",
    "Sent from the GitHub API",
    "On <day>",
    "<https://github.com/",
];

impl ConformanceResult {
    /// Create a conforming result with the given template type.
    pub fn conforming(template: TemplateType) -> Self {
        Self {
            is_conforming: true,
            template_type: Some(template),
        }
    }

    /// Create a non-conforming result.
    pub fn non_conforming() -> Self {
        Self {
            is_conforming: false,
            template_type: None,
        }
    }
}

/// Check if an issue body is conforming.
///
/// An issue is conforming if it contains any of the template markers.
/// If the markers are absent, the issue was filed without a template.
///
/// # Arguments
///
/// * `body` - The issue body text to check
///
/// # Returns
///
/// A `ConformanceResult` indicating whether the issue conforms and which template was used.
pub fn check_conformance(body: &str) -> ConformanceResult {
    for (marker, template) in TEMPLATE_MARKERS {
        if body.contains(marker) {
            return ConformanceResult::conforming(*template);
        }
    }
    ConformanceResult::non_conforming()
}

/// Check if an issue body appears to be a GitHub email reply.
///
/// GitHub email replies are non-conforming because they create issues without any
/// template context. The email reply indicator is found in the body.
///
/// # Arguments
///
/// * `body` - The issue body text to check
///
/// # Returns
///
/// `true` if the body appears to be an email reply
pub fn is_email_reply(body: &str) -> bool {
    // Check for common GitHub email reply patterns
    for pattern in EMAIL_REPLY_PATTERNS {
        if body.contains(pattern) {
            return true;
        }
    }
    false
}

/// Check if an issue is non-conforming due to missing template or email reply.
///
/// This is a convenience function that combines conformance checking with
/// email reply detection.
///
/// # Arguments
///
/// * `body` - The issue body text to check
///
/// # Returns
///
/// `true` if the issue is non-conforming (missing template or email reply)
pub fn is_non_conforming(body: &str) -> bool {
    !check_conformance(body).is_conforming || is_email_reply(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conforming_bug_report() {
        let body = r#"
## Bug Summary
Something is broken

## Environment
- OS: Ubuntu 22.04

## Steps to Reproduce
1. Do this
2. Do that

<!-- template: bug_report -->
"#;
        let result = check_conformance(body);
        assert!(result.is_conforming);
        assert_eq!(result.template_type, Some(TemplateType::BugReport));
    }

    #[test]
    fn test_conforming_feature_request() {
        let body = r#"
## Feature Summary
I want this feature

## Use Case
This would help me

## Proposed Behavior
It should work this way

<!-- template: feature_request -->
"#;
        let result = check_conformance(body);
        assert!(result.is_conforming);
        assert_eq!(result.template_type, Some(TemplateType::FeatureRequest));
    }

    #[test]
    fn test_conforming_question() {
        let body = r#"
## Question
How do I do this?

## Context
I'm trying to configure...

<!-- template: question -->
"#;
        let result = check_conformance(body);
        assert!(result.is_conforming);
        assert_eq!(result.template_type, Some(TemplateType::Question));
    }

    #[test]
    fn test_non_conforming_missing_marker() {
        let body = r#"
## My Issue
This is a bug where something doesn't work

I tried doing X but Y happened instead.
"#;
        let result = check_conformance(body);
        assert!(!result.is_conforming);
        assert!(result.template_type.is_none());
    }

    #[test]
    fn test_non_conforming_empty_body() {
        let body = "";
        let result = check_conformance(body);
        assert!(!result.is_conforming);
    }

    #[test]
    fn test_is_email_reply_github_api() {
        let body = r#"
Sent from the GitHub API
Issue content here
"#;
        assert!(is_email_reply(body));
    }

    #[test]
    fn test_is_email_reply_on_day_month() {
        let body = "
On Wed, May 20, 2026 someone wrote:
Issue content here
";
        assert!(is_email_reply(body));
    }

    #[test]
    fn test_is_not_email_reply_normal_issue() {
        let body = r#"
## Bug Summary
Something broken

## Steps to Reproduce
1. Do this

<!-- template: bug_report -->
"#;
        assert!(!is_email_reply(body));
    }

    #[test]
    fn test_is_non_conforming_with_marker() {
        let body = "Some content\n<!-- template: bug_report -->";
        assert!(!is_non_conforming(body));
    }

    #[test]
    fn test_is_non_conforming_without_marker() {
        let body = "Some freeform content without a template marker";
        assert!(is_non_conforming(body));
    }

    #[test]
    fn test_is_non_conforming_email_reply() {
        let body = "GitHub Email Reply\nSome content";
        assert!(is_non_conforming(body));
    }

    #[test]
    fn test_template_type_display_name() {
        assert_eq!(TemplateType::BugReport.display_name(), "Bug Report");
        assert_eq!(TemplateType::FeatureRequest.display_name(), "Feature Request");
        assert_eq!(TemplateType::Question.display_name(), "Question");
    }

    #[test]
    fn test_template_type_marker() {
        assert_eq!(TemplateType::BugReport.marker(), "<!-- template: bug_report -->");
        assert_eq!(TemplateType::FeatureRequest.marker(), "<!-- template: feature_request -->");
        assert_eq!(TemplateType::Question.marker(), "<!-- template: question -->");
    }

    #[test]
    fn test_conformance_result_conforming() {
        let result = ConformanceResult::conforming(TemplateType::BugReport);
        assert!(result.is_conforming);
        assert_eq!(result.template_type, Some(TemplateType::BugReport));
    }

    #[test]
    fn test_conformance_result_non_conforming() {
        let result = ConformanceResult::non_conforming();
        assert!(!result.is_conforming);
        assert!(result.template_type.is_none());
    }
}