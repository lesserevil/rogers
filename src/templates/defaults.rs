//! Default issue templates embedded in the Rodgers binary.
//!
//! These templates are used when a project has no `.github/ISSUE_TEMPLATE/`
//! directory or lacks required template files. They are designed to collect
//! the information Rodgers needs for completeness checking.

/// Bug report template - for reporting bugs and unexpected behavior.
pub const BUG_REPORT_TEMPLATE: &str = r#"---
name: Bug Report
about: Report something that isn't working as expected
labels: bug
---

## Bug Summary
<!-- One-line description of the bug -->

## Environment
- OS: [e.g. Ubuntu 22.04, Windows 11, macOS 14]
- Version: [software version if known]
- Other relevant context: [GPU model, driver version, etc.]

## Steps to Reproduce
<!-- Numbered list of steps. Be specific. -->
1.
2.
3.

## Expected Behavior
<!-- What you expected to happen instead -->

## Actual Behavior
<!-- What actually happened -->

## Relevant Logs / Error Messages
<!-- Paste or describe any error output. Leave blank if none. -->

## Possible Cause
<!-- Optional: your theory on why this is happening. Leave blank if unknown. -->

<!-- template: bug_report -->
"#;

/// Feature request template - for suggesting new capabilities.
pub const FEATURE_REQUEST_TEMPLATE: &str = r#"---
name: Feature Request
about: Suggest a new capability or behavioral change
labels: feature
---

## Feature Summary
<!-- One-line description of the requested feature -->

## Use Case
<!-- Why do you need this? What problem does it solve? -->

## Proposed Behavior
<!-- How should this feature work once implemented? Be specific. -->

## Acceptance Criteria
<!-- Numbered list of conditions that prove the feature is correctly implemented. -->
<!-- Each criterion must be testable — "it works well" is not a criterion. -->
1.
2.
3.

## Alternatives Considered
<!-- Optional: other approaches you considered and why they don't work -->

<!-- template: feature_request -->
"#;

/// Question template - for asking about usage or configuration.
pub const QUESTION_TEMPLATE: &str = r#"---
name: Question
about: Ask about how to use or configure the project
labels: question
---

## Question
<!-- State your question clearly. Be specific about what you've tried and what you're trying to achieve. -->

## Context
<!-- Provide enough context for someone to answer without来回往返. Include: -->
<!-- - What you were trying to do -->
<!-- - What you already tried -->
<!-- - Relevant version / configuration -->

<!-- template: question -->
"#;

/// All default templates as a slice for iteration.
pub const ALL_DEFAULT_TEMPLATES: &[(&str, &str, &str)] = &[
    ("bug_report.md", "Bug Report Template", BUG_REPORT_TEMPLATE),
    (
        "feature_request.md",
        "Feature Request Template",
        FEATURE_REQUEST_TEMPLATE,
    ),
    ("question.md", "Question Template", QUESTION_TEMPLATE),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bug_report_has_required_sections() {
        let template = BUG_REPORT_TEMPLATE;
        assert!(template.contains("## Environment"));
        assert!(template.contains("## Steps to Reproduce"));
        assert!(template.contains("## Expected Behavior"));
        assert!(template.contains("## Actual Behavior"));
    }

    #[test]
    fn test_feature_request_has_required_sections() {
        let template = FEATURE_REQUEST_TEMPLATE;
        assert!(template.contains("## Use Case"));
        assert!(template.contains("## Proposed Behavior"));
        assert!(template.contains("## Acceptance Criteria"));
    }

    #[test]
    fn test_question_has_required_sections() {
        let template = QUESTION_TEMPLATE;
        assert!(template.contains("## Question"));
        assert!(template.contains("## Context"));
    }

    #[test]
    fn test_all_templates_contain_frontmatter() {
        for (_, _, template) in ALL_DEFAULT_TEMPLATES {
            assert!(
                template.starts_with("---\nname:"),
                "Template should start with YAML frontmatter"
            );
        }
    }

    #[test]
    fn test_all_templates_contain_label() {
        for (_, _, template) in ALL_DEFAULT_TEMPLATES {
            assert!(
                template.contains("labels:"),
                "Template should contain labels field"
            );
        }
    }

    #[test]
    fn test_templates_match_plan_definitions() {
        // Verify templates from the plan are correctly embedded
        // Plan: plans/issue-templates-plan.md

        // Bug report fields map to completeness requirements
        assert!(BUG_REPORT_TEMPLATE.contains("## Environment"));
        assert!(BUG_REPORT_TEMPLATE.contains("## Steps to Reproduce"));
        assert!(BUG_REPORT_TEMPLATE.contains("## Expected Behavior"));
        assert!(BUG_REPORT_TEMPLATE.contains("## Actual Behavior"));

        // Feature request fields map to completeness requirements
        assert!(FEATURE_REQUEST_TEMPLATE.contains("## Use Case"));
        assert!(FEATURE_REQUEST_TEMPLATE.contains("## Proposed Behavior"));
        assert!(FEATURE_REQUEST_TEMPLATE.contains("## Acceptance Criteria"));

        // Question fields map to completeness requirements
        assert!(QUESTION_TEMPLATE.contains("## Question"));
        assert!(QUESTION_TEMPLATE.contains("## Context"));
    }

    #[test]
    fn test_all_templates_have_conformance_markers() {
        // Templates must include hidden markers for conformance detection
        // See: src/templates/conformance.rs
        for (_, _, template) in ALL_DEFAULT_TEMPLATES {
            assert!(
                template.contains("<!-- template:"),
                "Template should contain conformance marker"
            );
        }
    }
}
