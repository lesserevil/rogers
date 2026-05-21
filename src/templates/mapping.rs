//! Semantic field mapping for issue templates.
//!
//! Maps template section headings (which may use various synonyms) to
//! Rodgers' canonical field names used in completeness checking.
//!
//! ## Mapping Strategy
//!
//! Field matching is **semantic**, not exact-match:
//! - "Environment", "System", "Platform", "OS/Environment" all map to `Environment`
//! - "Steps to Reproduce", "Reproduction Steps", "Steps" map to `StepsToReproduce`
//! - "Expected Behavior", "Expected", "What I Expected" map to `ExpectedBehavior`
//! - "Actual Behavior", "Actual", "What Happened", "Observed" map to `ActualBehavior`
//!
//! This allows custom templates with slightly different field names to still
//! be processed correctly.

use serde::{Deserialize, Serialize};

/// Canonical field names used in completeness checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanonicalField {
    /// Environment/OS information
    Environment,
    /// Steps to reproduce the issue
    StepsToReproduce,
    /// What the reporter expected to happen
    ExpectedBehavior,
    /// What actually happened
    ActualBehavior,
    /// Why this feature is needed
    UseCase,
    /// How the feature should work
    ProposedBehavior,
    /// Testable, enumerated criteria for feature completion
    AcceptanceCriteria,
    /// Question being asked
    Question,
    /// Context for the question
    Context,
}

impl CanonicalField {
    /// Returns the display name for this field.
    pub fn display_name(&self) -> &'static str {
        match self {
            CanonicalField::Environment => "Environment",
            CanonicalField::StepsToReproduce => "Steps to Reproduce",
            CanonicalField::ExpectedBehavior => "Expected Behavior",
            CanonicalField::ActualBehavior => "Actual Behavior",
            CanonicalField::UseCase => "Use Case",
            CanonicalField::ProposedBehavior => "Proposed Behavior",
            CanonicalField::AcceptanceCriteria => "Acceptance Criteria",
            CanonicalField::Question => "Question",
            CanonicalField::Context => "Context",
        }
    }

    /// Returns the template heading for this field.
    pub fn heading(&self) -> &'static str {
        match self {
            CanonicalField::Environment => "## Environment",
            CanonicalField::StepsToReproduce => "## Steps to Reproduce",
            CanonicalField::ExpectedBehavior => "## Expected Behavior",
            CanonicalField::ActualBehavior => "## Actual Behavior",
            CanonicalField::UseCase => "## Use Case",
            CanonicalField::ProposedBehavior => "## Proposed Behavior",
            CanonicalField::AcceptanceCriteria => "## Acceptance Criteria",
            CanonicalField::Question => "## Question",
            CanonicalField::Context => "## Context",
        }
    }
}

/// A mapping from template field patterns to canonical fields.
///
/// Each entry contains:
/// - patterns: substrings/headings to search for
/// - canonical: the canonical field it maps to
#[derive(Debug, Clone)]
pub struct FieldMapping {
    /// Patterns to search for in template headings/sections
    pub patterns: &'static [&'static str],
    /// The canonical field this maps to
    pub canonical: CanonicalField,
}

/// Complete field mapping for bug report template.
///
/// This is the "rogers-agw" semantic mapping. It maps template section
/// headings to canonical completeness fields.
pub const BUG_REPORT_FIELD_MAPPINGS: &[FieldMapping] = &[
    FieldMapping {
        patterns: &["environment", "system", "platform", "os"],
        canonical: CanonicalField::Environment,
    },
    FieldMapping {
        patterns: &[
            "steps to reproduce",
            "reproduction steps",
            "how to reproduce",
        ],
        canonical: CanonicalField::StepsToReproduce,
    },
    FieldMapping {
        patterns: &[
            "expected",
            "what i expected",
            "expected behavior",
            "expected result",
        ],
        canonical: CanonicalField::ExpectedBehavior,
    },
    FieldMapping {
        patterns: &[
            "actual",
            "what actually",
            "what happened",
            "observed behavior",
            "actual behavior",
        ],
        canonical: CanonicalField::ActualBehavior,
    },
];

/// Field mapping for feature request template.
///
/// Maps template section headings to canonical completeness fields for
/// feature requests. Each field is required for feature completeness.
pub const FEATURE_REQUEST_FIELD_MAPPINGS: &[FieldMapping] = &[
    FieldMapping {
        patterns: &["use case", "user story", "why", "motivation", "background"],
        canonical: CanonicalField::UseCase,
    },
    FieldMapping {
        patterns: &[
            "proposed behavior",
            "how it should work",
            "solution",
            "implementation",
            "how it works",
        ],
        canonical: CanonicalField::ProposedBehavior,
    },
    FieldMapping {
        patterns: &[
            "acceptance criteria",
            "verification",
            "criteria",
            "how to test",
        ],
        canonical: CanonicalField::AcceptanceCriteria,
    },
];

/// Field mapping for question template.
///
/// Maps template section headings to canonical completeness fields for
/// questions. Each field is required to proceed with doc search.
pub const QUESTION_FIELD_MAPPINGS: &[FieldMapping] = &[
    FieldMapping {
        patterns: &["question", "what", "inquiry"],
        canonical: CanonicalField::Question,
    },
    FieldMapping {
        patterns: &["context", "background", "additional info"],
        canonical: CanonicalField::Context,
    },
];

/// Check if a heading matches any of the given patterns.
///
/// Case-insensitive matching with word-boundary awareness. A pattern matches
/// if it appears as a complete word (bounded by whitespace, punctuation,
/// or start/end of string).
pub fn heading_matches(heading: &str, patterns: &'static [&'static str]) -> bool {
    let lower = heading.to_lowercase();
    for pattern in patterns {
        if has_word_boundary_match(&lower, pattern) {
            return true;
        }
    }
    false
}

/// Check if a pattern appears as a word in the text.
/// A word boundary is whitespace, punctuation, or start/end of string.
fn has_word_boundary_match(text: &str, pattern: &str) -> bool {
    let text_bytes = text.as_bytes();
    let pattern_bytes = pattern.as_bytes();

    // Need at least as many bytes as the pattern
    if text_bytes.len() < pattern_bytes.len() {
        return false;
    }

    let mut idx = 0;
    while idx + pattern_bytes.len() <= text_bytes.len() {
        // Check if pattern matches at current position
        if text_bytes[idx..idx + pattern_bytes.len()] == *pattern_bytes {
            let start_ok = idx == 0 || !text_bytes[idx - 1].is_ascii_alphanumeric();
            let end_ok = idx + pattern_bytes.len() >= text_bytes.len()
                || !text_bytes[idx + pattern_bytes.len()].is_ascii_alphanumeric();

            if start_ok && end_ok {
                return true;
            }
        }
        idx += 1;
    }
    false
}

/// All field mappings combined (bug, feature, question).
pub const ALL_FIELD_MAPPINGS: &[&[FieldMapping]] = &[
    BUG_REPORT_FIELD_MAPPINGS,
    FEATURE_REQUEST_FIELD_MAPPINGS,
    QUESTION_FIELD_MAPPINGS,
];

/// Find the canonical field for a given heading string.
///
/// Uses semantic matching against all registered mappings (bug, feature, question).
/// Returns `Some(canonical_field)` if a match is found, `None` otherwise.
pub fn map_heading_to_field(heading: &str) -> Option<CanonicalField> {
    let lower = heading.to_lowercase();
    for mapping_set in ALL_FIELD_MAPPINGS {
        for mapping in *mapping_set {
            if mapping.patterns.iter().any(|p| lower.contains(p)) {
                return Some(mapping.canonical);
            }
        }
    }
    None
}

/// Find the canonical field using a specific mapping set.
///
/// Returns `Some(canonical_field)` if a match is found within the given
/// mapping set, `None` otherwise.
pub fn map_heading_with_mappings(
    heading: &str,
    mappings: &'static [FieldMapping],
) -> Option<CanonicalField> {
    let lower = heading.to_lowercase();
    for mapping in mappings {
        if mapping.patterns.iter().any(|p| lower.contains(p)) {
            return Some(mapping.canonical);
        }
    }
    None
}

/// Extract section content from an issue body for a given heading.
///
/// Finds the section starting with the heading and returns its content
/// (everything until the next `##` heading or end of document).
pub fn extract_section_content(body: &str, heading: &str) -> Option<String> {
    let lower = heading.to_lowercase();
    let heading_pos = body.to_lowercase().find(&lower)?;

    let remaining = &body[heading_pos..];
    let sections: Vec<&str> = remaining.split("\n## ").collect();

    // sections[0] contains the heading line + its content (up to next \n## or end)
    // Example: "## Environment\n- OS: Ubuntu\n- Version: 1.0"
    let section_0 = sections[0];

    // Find the end of the heading line (first newline)
    if let Some(newline_pos) = section_0.find('\n') {
        // Content is everything after the heading line
        let content = &section_0[newline_pos + 1..];
        let trimmed = content.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    } else {
        // No newline: the entire section is the heading with possible inline content
        let content = section_0.strip_prefix(&lower)?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

/// Check if a section's content is meaningfully populated.
///
/// Empty content, whitespace-only, "N/A" without explanation, or
/// placeholder text like "[example]" are considered missing.
pub fn is_section_populated(content: &str) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_lowercase();

    // "N/A" alone is not populated (but "N/A: <explanation>" is)
    if lower == "n/a" || lower == "n.a." {
        return false;
    }

    // Placeholder patterns
    let placeholders = [
        "[",
        "example",
        "insert here",
        "type here",
        "description",
        "fill in",
        "todo",
        "xxx",
    ];
    for placeholder in &placeholders {
        if lower.contains(placeholder) {
            return false;
        }
    }

    // Minimum content threshold - at least 3 characters of real content
    trimmed.len() >= 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_matches_environment() {
        assert!(heading_matches(
            "## Environment",
            BUG_REPORT_FIELD_MAPPINGS[0].patterns
        ));
        assert!(heading_matches(
            "## System",
            BUG_REPORT_FIELD_MAPPINGS[0].patterns
        ));
        assert!(heading_matches(
            "## Platform",
            BUG_REPORT_FIELD_MAPPINGS[0].patterns
        ));
        assert!(heading_matches(
            "## OS",
            BUG_REPORT_FIELD_MAPPINGS[0].patterns
        ));
    }

    #[test]
    fn test_heading_matches_steps_to_reproduce() {
        assert!(heading_matches(
            "## Steps to Reproduce",
            BUG_REPORT_FIELD_MAPPINGS[1].patterns
        ));
        assert!(heading_matches(
            "## Reproduction Steps",
            BUG_REPORT_FIELD_MAPPINGS[1].patterns
        ));
        assert!(heading_matches(
            "## How to Reproduce",
            BUG_REPORT_FIELD_MAPPINGS[1].patterns
        ));
    }

    #[test]
    fn test_heading_matches_expected_behavior() {
        assert!(heading_matches(
            "## Expected Behavior",
            BUG_REPORT_FIELD_MAPPINGS[2].patterns
        ));
        assert!(heading_matches(
            "## Expected",
            BUG_REPORT_FIELD_MAPPINGS[2].patterns
        ));
    }

    #[test]
    fn test_heading_matches_actual_behavior() {
        assert!(heading_matches(
            "## Actual Behavior",
            BUG_REPORT_FIELD_MAPPINGS[3].patterns
        ));
        assert!(heading_matches(
            "## Actual",
            BUG_REPORT_FIELD_MAPPINGS[3].patterns
        ));
        assert!(heading_matches(
            "## What Happened",
            BUG_REPORT_FIELD_MAPPINGS[3].patterns
        ));
    }

    #[test]
    fn test_heading_matches_unknown_section() {
        assert!(!heading_matches(
            "## Bug Summary",
            BUG_REPORT_FIELD_MAPPINGS[0].patterns
        ));
        assert!(!heading_matches(
            "## Possible Cause",
            BUG_REPORT_FIELD_MAPPINGS[0].patterns
        ));
    }

    #[test]
    fn test_map_heading_to_field_environment() {
        assert_eq!(
            map_heading_to_field("## Environment"),
            Some(CanonicalField::Environment)
        );
        assert_eq!(
            map_heading_to_field("## System Information"),
            Some(CanonicalField::Environment)
        );
    }

    #[test]
    fn test_map_heading_to_field_steps() {
        assert_eq!(
            map_heading_to_field("## Steps to Reproduce"),
            Some(CanonicalField::StepsToReproduce)
        );
        assert_eq!(
            map_heading_to_field("## Reproduction Steps"),
            Some(CanonicalField::StepsToReproduce)
        );
    }

    #[test]
    fn test_map_heading_to_field_expected() {
        assert_eq!(
            map_heading_to_field("## Expected Behavior"),
            Some(CanonicalField::ExpectedBehavior)
        );
    }

    #[test]
    fn test_map_heading_to_field_actual() {
        assert_eq!(
            map_heading_to_field("## Actual Behavior"),
            Some(CanonicalField::ActualBehavior)
        );
        assert_eq!(
            map_heading_to_field("## What Happened"),
            Some(CanonicalField::ActualBehavior)
        );
    }

    #[test]
    fn test_map_heading_to_field_unknown() {
        assert_eq!(map_heading_to_field("## Bug Summary"), None);
    }

    #[test]
    fn test_is_section_populated_valid_content() {
        assert!(is_section_populated("Ubuntu 22.04 with Docker container"));
        assert!(is_section_populated(
            "1. Open the app\n2. Click submit\n3. See crash"
        ));
        assert!(is_section_populated("The app should not crash"));
    }

    #[test]
    fn test_is_section_populated_empty() {
        assert!(!is_section_populated(""));
        assert!(!is_section_populated("   "));
        assert!(!is_section_populated("\n\n"));
    }

    #[test]
    fn test_is_section_populated_na_alone() {
        assert!(!is_section_populated("N/A"));
        assert!(!is_section_populated("n/a"));
        assert!(!is_section_populated("N.A."));
    }

    #[test]
    fn test_is_section_populated_na_with_explanation_valid() {
        // N/A with explanation is valid per plan: "'N/A' with explanation for Steps - valid if justified"
        assert!(is_section_populated(
            "N/A: The bug is a crash on startup, cannot reproduce"
        ));
        assert!(is_section_populated(
            "n/a: data corruption, no reliable reproduction"
        ));
    }

    #[test]
    fn test_is_section_populated_placeholder() {
        assert!(!is_section_populated("[example]"));
        assert!(!is_section_populated("[type here]"));
        assert!(!is_section_populated("[fill in details]"));
        assert!(!is_section_populated("todo"));
    }

    #[test]
    fn test_is_section_populated_too_short() {
        assert!(!is_section_populated("a"));
        assert!(!is_section_populated("ab"));
    }

    #[test]
    fn test_extract_section_content() {
        let body = r#"
## Environment
- OS: Ubuntu 22.04
- Version: 1.0

## Steps to Reproduce
1. Open app
2. Click button

## Expected Behavior
Should work

## Actual Behavior
It crashes
"#;

        let env = extract_section_content(body, "## Environment");
        assert!(env.is_some());
        assert!(env.unwrap().contains("Ubuntu 22.04"));

        let steps = extract_section_content(body, "## Steps to Reproduce");
        assert!(steps.is_some());
        assert!(steps.unwrap().contains("Open app"));
    }

    #[test]
    fn test_extract_section_content_missing() {
        let body = "## Bug Summary\nSomething broken";
        assert_eq!(extract_section_content(body, "## Environment"), None);
    }

    #[test]
    fn test_extract_section_content_last_section() {
        let body = r#"
## Environment
Ubuntu 22.04

## Actual Behavior
It crashes
"#;

        let actual = extract_section_content(body, "## Actual Behavior");
        assert!(actual.is_some());
        assert!(actual.unwrap().contains("crashes"));
    }

    #[test]
    fn test_canonical_field_display_names() {
        assert_eq!(CanonicalField::Environment.display_name(), "Environment");
        assert_eq!(
            CanonicalField::StepsToReproduce.display_name(),
            "Steps to Reproduce"
        );
        assert_eq!(
            CanonicalField::ExpectedBehavior.display_name(),
            "Expected Behavior"
        );
        assert_eq!(
            CanonicalField::ActualBehavior.display_name(),
            "Actual Behavior"
        );
    }

    #[test]
    fn test_canonical_field_headings() {
        assert_eq!(CanonicalField::Environment.heading(), "## Environment");
        assert_eq!(
            CanonicalField::StepsToReproduce.heading(),
            "## Steps to Reproduce"
        );
        assert_eq!(
            CanonicalField::ExpectedBehavior.heading(),
            "## Expected Behavior"
        );
        assert_eq!(
            CanonicalField::ActualBehavior.heading(),
            "## Actual Behavior"
        );
        // Feature request headings
        assert_eq!(CanonicalField::UseCase.heading(), "## Use Case");
        assert_eq!(
            CanonicalField::ProposedBehavior.heading(),
            "## Proposed Behavior"
        );
        assert_eq!(
            CanonicalField::AcceptanceCriteria.heading(),
            "## Acceptance Criteria"
        );
        // Question headings
        assert_eq!(CanonicalField::Question.heading(), "## Question");
        assert_eq!(CanonicalField::Context.heading(), "## Context");
    }

    // === Feature Request Mapping Tests ===

    #[test]
    fn test_feature_request_mapping_use_case() {
        assert_eq!(
            map_heading_with_mappings("## Use Case", FEATURE_REQUEST_FIELD_MAPPINGS),
            Some(CanonicalField::UseCase)
        );
        assert_eq!(
            map_heading_with_mappings("## User Story", FEATURE_REQUEST_FIELD_MAPPINGS),
            Some(CanonicalField::UseCase)
        );
        assert_eq!(
            map_heading_with_mappings("## Why", FEATURE_REQUEST_FIELD_MAPPINGS),
            Some(CanonicalField::UseCase)
        );
        assert_eq!(
            map_heading_with_mappings("## Motivation", FEATURE_REQUEST_FIELD_MAPPINGS),
            Some(CanonicalField::UseCase)
        );
    }

    #[test]
    fn test_feature_request_mapping_proposed_behavior() {
        assert_eq!(
            map_heading_with_mappings(
                "## Proposed Behavior",
                FEATURE_REQUEST_FIELD_MAPPINGS
            ),
            Some(CanonicalField::ProposedBehavior)
        );
        assert_eq!(
            map_heading_with_mappings("## Solution", FEATURE_REQUEST_FIELD_MAPPINGS),
            Some(CanonicalField::ProposedBehavior)
        );
    }

    #[test]
    fn test_feature_request_mapping_acceptance_criteria() {
        assert_eq!(
            map_heading_with_mappings(
                "## Acceptance Criteria",
                FEATURE_REQUEST_FIELD_MAPPINGS
            ),
            Some(CanonicalField::AcceptanceCriteria)
        );
        assert_eq!(
            map_heading_with_mappings("## Verification", FEATURE_REQUEST_FIELD_MAPPINGS),
            Some(CanonicalField::AcceptanceCriteria)
        );
    }

    #[test]
    fn test_feature_fields_map_to_3_requirements() {
        // CRIT-6: Feature template fields map to 3 feature requirements
        assert_eq!(FEATURE_REQUEST_FIELD_MAPPINGS.len(), 3);
        let canonicals: Vec<CanonicalField> = FEATURE_REQUEST_FIELD_MAPPINGS
            .iter()
            .map(|m| m.canonical)
            .collect();
        assert!(canonicals.contains(&CanonicalField::UseCase));
        assert!(canonicals.contains(&CanonicalField::ProposedBehavior));
        assert!(canonicals.contains(&CanonicalField::AcceptanceCriteria));
    }

    // === Question Mapping Tests ===

    #[test]
    fn test_question_mapping_question() {
        assert_eq!(
            map_heading_with_mappings("## Question", QUESTION_FIELD_MAPPINGS),
            Some(CanonicalField::Question)
        );
    }

    #[test]
    fn test_question_mapping_context() {
        assert_eq!(
            map_heading_with_mappings("## Context", QUESTION_FIELD_MAPPINGS),
            Some(CanonicalField::Context)
        );
        assert_eq!(
            map_heading_with_mappings("## Background", QUESTION_FIELD_MAPPINGS),
            Some(CanonicalField::Context)
        );
    }

    #[test]
    fn test_question_fields_map_to_2_requirements() {
        // CRIT-6: Question template fields map to 2 question requirements
        assert_eq!(QUESTION_FIELD_MAPPINGS.len(), 2);
        let canonicals: Vec<CanonicalField> =
            QUESTION_FIELD_MAPPINGS.iter().map(|m| m.canonical).collect();
        assert!(canonicals.contains(&CanonicalField::Question));
        assert!(canonicals.contains(&CanonicalField::Context));
    }

    // === Bug Mapping Test ===

    #[test]
    fn test_bug_fields_map_to_4_requirements() {
        // CRIT-6: Bug template fields map to 4 bug requirements
        assert_eq!(BUG_REPORT_FIELD_MAPPINGS.len(), 4);
        let canonicals: Vec<CanonicalField> = BUG_REPORT_FIELD_MAPPINGS
            .iter()
            .map(|m| m.canonical)
            .collect();
        assert!(canonicals.contains(&CanonicalField::Environment));
        assert!(canonicals.contains(&CanonicalField::StepsToReproduce));
        assert!(canonicals.contains(&CanonicalField::ExpectedBehavior));
        assert!(canonicals.contains(&CanonicalField::ActualBehavior));
    }
}
