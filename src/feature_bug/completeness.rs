#![allow(dead_code)]

//! Completeness verification for bug and feature issues.
//!
//! This module implements the completeness check defined in plans/feature-bug-plan.md.
//! It verifies that bug/feature issues have all required information fields present
//! before they can transition to ready-for-review.
//!
//! ## Bug Report Requirements
//!
//! A bug report is complete when all of the following are present:
//! 1. Environment - OS, version, relevant context
//! 2. Steps to Reproduce - reproducible steps (or justified N/A)
//! 3. Expected Behavior - what should have happened
//! 4. Actual Behavior - what actually happened
//!
//! ## Feature Request Requirements
//!
//! A feature request is complete when all of the following are present:
//! 1. Use Case - why this feature is needed
//! 2. Proposed Behavior - how the feature should work
//! 3. Acceptance Criteria - testable, enumerated list

use serde::{Deserialize, Serialize};

use crate::templates::mapping::{
    CanonicalField, extract_section_content, is_section_populated, map_heading_to_field,
};

/// Result of a completeness check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletenessResult {
    /// Whether the issue is complete.
    pub is_complete: bool,
    /// Which specific fields are missing (if any).
    pub missing_fields: Vec<CanonicalField>,
}

impl CompletenessResult {
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
            "Hi! To help us move forward, we need a bit more information:".to_string(),
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
            "Once this is added, we'll be able to review your submission. Thanks!"
                .to_string(),
        );

        Some(lines.join("\n"))
    }
}

// ======================== Bug Completeness ========================

/// Required fields for bug completeness.
///
/// All four fields must be present and populated for a bug to be ready for review.
const BUG_REQUIRED_FIELDS: &[CanonicalField] = &[
    CanonicalField::Environment,
    CanonicalField::StepsToReproduce,
    CanonicalField::ExpectedBehavior,
    CanonicalField::ActualBehavior,
];

/// Check if a bug report is complete.
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
/// A `CompletenessResult` indicating completeness and any missing fields
pub fn check_bug_completeness(body: &str) -> CompletenessResult {
    if body.is_empty() {
        return CompletenessResult::incomplete(BUG_REQUIRED_FIELDS.to_vec());
    }

    let mut missing_fields = Vec::new();

    for field in BUG_REQUIRED_FIELDS {
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
        CompletenessResult::complete()
    } else {
        CompletenessResult::incomplete(missing_fields)
    }
}

/// Check if a bug report is complete using semantic field mapping.
///
/// This variant uses the full semantic mapping (including custom template
/// field names like "System" → Environment, "What Happened" → Actual Behavior).
///
/// # Arguments
///
/// * `body` - The issue body text to check
///
/// # Returns
///
/// A `CompletenessResult` indicating completeness and any missing fields
pub fn check_bug_completeness_semantic(body: &str) -> CompletenessResult {
    if body.is_empty() {
        return CompletenessResult::incomplete(BUG_REQUIRED_FIELDS.to_vec());
    }

    let mut missing_fields = Vec::new();

    for field in BUG_REQUIRED_FIELDS {
        let heading = field.heading();
        let content = extract_section_content(body, heading);

        // If standard heading not found, try semantic mapping
        let content = if content.is_none() {
            find_semantic_field_content(body, field)
        } else {
            content
        };

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
        CompletenessResult::complete()
    } else {
        CompletenessResult::incomplete(missing_fields)
    }
}

/// Find content for a canonical field using semantic matching.
///
/// Searches the body for any section that semantically matches the given field.
/// Returns the content if found, None otherwise.
fn find_semantic_field_content(body: &str, field: &CanonicalField) -> Option<String> {
    let sections: Vec<&str> = body.split("\n## ").collect();

    for section in &sections[1..] {
        let heading = section.lines().next().unwrap_or("");

        if let Some(canonical) = map_heading_to_field(heading) {
            if canonical == *field {
                let content = &section[heading.len()..];
                return if is_section_populated(content.trim()) {
                    Some(content.trim().to_string())
                } else {
                    None
                };
            }
        }
    }

    None
}

// ======================== Feature Completeness ========================

/// Required fields for feature request completeness.
///
/// All three fields must be present and populated for a feature request
/// to be ready for review.
const FEATURE_REQUIRED_FIELDS: &[CanonicalField] = &[
    CanonicalField::UseCase,
    CanonicalField::ProposedBehavior,
    CanonicalField::AcceptanceCriteria,
];

/// Check if a feature request is complete.
///
/// Scans the issue body for template sections (using standard headings)
/// and verifies each required field is present and populated.
///
/// # Arguments
///
/// * `body` - The issue body text to check
///
/// # Returns
///
/// A `CompletenessResult` indicating completeness and any missing fields
pub fn check_feature_completeness(body: &str) -> CompletenessResult {
    if body.is_empty() {
        return CompletenessResult::incomplete(FEATURE_REQUIRED_FIELDS.to_vec());
    }

    let mut missing_fields = Vec::new();

    for field in FEATURE_REQUIRED_FIELDS {
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
        CompletenessResult::complete()
    } else {
        CompletenessResult::incomplete(missing_fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Bug Completeness Tests ===

    #[test]
    fn test_complete_bug_report_all_fields_present() {
        let body = r#"
## Bug Summary
App crashes on submit

## Environment
- OS: Ubuntu 22.04
- Version: 1.0.0

## Steps to Reproduce
1. Open the application
2. Navigate to the settings page
3. Click the submit button
4. Application crashes

## Expected Behavior
The form should save and show a success message

## Actual Behavior
The application crashes with a NullPointerException
"#;

        let result = check_bug_completeness(body);
        assert!(result.is_complete);
        assert!(result.missing_fields.is_empty());
    }

    #[test]
    fn test_complete_bug_report_semantic_fields() {
        let body = r#"
## Bug Summary
App crashes

## System
- OS: Windows 11

## Reproduction Steps
1. Open app

## Expected
Should not crash

## What Happened
It crashes
"#;

        let result = check_bug_completeness_semantic(body);
        assert!(result.is_complete);
        assert!(result.missing_fields.is_empty());
    }

    #[test]
    fn test_complete_bug_report_ready_for_review_no_comment() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(result.is_complete);
        assert!(result.to_request_comment().is_none());
    }

    // === Bug Missing Fields Tests ===

    #[test]
    fn test_missing_environment_field() {
        let body = r#"
## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Environment));
    }

    #[test]
    fn test_missing_steps_to_reproduce() {
        let body = r#"
## Environment
Ubuntu 22.04

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::StepsToReproduce)
        );
    }

    #[test]
    fn test_missing_expected_behavior() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ExpectedBehavior)
        );
    }

    #[test]
    fn test_missing_actual_behavior() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ActualBehavior)
        );
    }

    #[test]
    fn test_missing_multiple_fields() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 2);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ExpectedBehavior)
        );
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ActualBehavior)
        );
    }

    #[test]
    fn test_missing_all_fields() {
        let body = "App doesn't work";

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 4);
    }

    // === Bug Empty Field Tests ===

    #[test]
    fn test_empty_environment_field() {
        let body = r#"
## Environment

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Environment));
    }

    #[test]
    fn test_whitespace_only_field() {
        let body = r#"
## Environment
   
## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Environment));
    }

    #[test]
    fn test_placeholder_content_treated_as_missing() {
        let body = r#"
## Environment
[type here]

## Steps to Reproduce
1. Open app

## Expected Behavior
[fill in]

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::Environment));
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ExpectedBehavior)
        );
    }

    #[test]
    fn test_na_alone_treated_as_missing() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
N/A

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::StepsToReproduce)
        );
    }

    #[test]
    fn test_na_with_explanation_is_valid() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
N/A: The bug is a crash on startup, cannot reproduce

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(result.is_complete);
    }

    #[test]
    fn test_empty_body_all_fields_missing() {
        let result = check_bug_completeness("");
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 4);
    }

    #[test]
    fn test_request_comment_for_missing_fields() {
        let body = r#"
## Environment
Ubuntu 22.04
"#;

        let result = check_bug_completeness(body);
        let comment = result.to_request_comment();
        assert!(comment.is_some());

        let comment = comment.unwrap();
        assert!(comment.contains("Steps to Reproduce"));
        assert!(comment.contains("Expected Behavior"));
        assert!(comment.contains("Actual Behavior"));
    }

    #[test]
    fn test_no_request_comment_when_complete() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(result.to_request_comment().is_none());
    }

    #[test]
    fn test_completeness_result_methods() {
        let complete = CompletenessResult::complete();
        assert!(complete.is_complete);
        assert!(!complete.has_missing_fields());

        let incomplete = CompletenessResult::incomplete(vec![CanonicalField::Environment]);
        assert!(!incomplete.is_complete);
        assert!(incomplete.has_missing_fields());
    }

    // === Bug Semantic Mapping Tests ===

    #[test]
    fn test_semantic_mapping_custom_environment_field() {
        let body = r#"
## System
- OS: macOS 14

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness_semantic(body);
        assert!(result.is_complete);
    }

    #[test]
    fn test_semantic_mapping_custom_actual_behavior_field() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## What Happened
The app freezes
"#;

        let result = check_bug_completeness_semantic(body);
        assert!(result.is_complete);
    }

    #[test]
    fn test_semantic_mapping_custom_steps_field() {
        let body = r#"
## Environment
Ubuntu 22.04

## How to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness_semantic(body);
        assert!(result.is_complete);
    }

    // === Bug Integration: All 4 fields → ready-for-review ===

    #[test]
    fn test_bug_with_all_4_fields_ready_for_review() {
        let body = r#"
## Bug Summary
Network request fails with 500 error

## Environment
- OS: Ubuntu 22.04
- Version: 2.1.0
- Browser: Chrome 120

## Steps to Reproduce
1. Log in to the dashboard
2. Navigate to the reports page
3. Click "Export CSV"
4. See 500 error

## Expected Behavior
The CSV should download successfully

## Actual Behavior
A 500 Internal Server Error appears
"#;

        let result = check_bug_completeness(body);

        assert!(
            result.is_complete,
            "Bug report with all 4 fields should be complete"
        );
        assert!(
            result.missing_fields.is_empty(),
            "No fields should be missing"
        );
        assert!(
            result.to_request_comment().is_none(),
            "Should NOT post a needs-information comment when complete"
        );
        assert_eq!(result.missing_fields.len(), 0);
    }

    // === Bug Missing individual field → needs-information ===

    #[test]
    fn test_missing_environment_needs_info_for_environment_only() {
        let body = r#"
## Steps to Reproduce
1. Open app

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 1);
        assert!(result.missing_fields.contains(&CanonicalField::Environment));
        assert!(
            !result
                .missing_fields
                .contains(&CanonicalField::StepsToReproduce)
        );
        assert!(
            !result
                .missing_fields
                .contains(&CanonicalField::ExpectedBehavior)
        );
        assert!(
            !result
                .missing_fields
                .contains(&CanonicalField::ActualBehavior)
        );
    }

    #[test]
    fn test_missing_steps_needs_info_for_steps_only() {
        let body = r#"
## Environment
Ubuntu 22.04

## Expected Behavior
Should work

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 1);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::StepsToReproduce)
        );
    }

    #[test]
    fn test_missing_expected_needs_info_for_expected_only() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Actual Behavior
Crashes
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 1);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ExpectedBehavior)
        );
    }

    #[test]
    fn test_missing_actual_needs_info_for_actual_only() {
        let body = r#"
## Environment
Ubuntu 22.04

## Steps to Reproduce
1. Open app

## Expected Behavior
Should work
"#;

        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 1);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ActualBehavior)
        );
    }

    // === Feature Completeness Tests ===

    #[test]
    fn test_complete_feature_request_all_fields_present() {
        let body = r#"
## Use Case
As a user, I want to export my data to CSV so I can analyze it in Excel.

## Proposed Behavior
When clicking the export button, the system should generate a CSV file with
all visible data and prompt the user to download it.

## Acceptance Criteria
- [ ] Export button appears in the toolbar
- [ ] Clicking the button generates a CSV file
- [ ] The CSV contains all visible columns
- [ ] User is prompted to download the file
"#;

        let result = check_feature_completeness(body);
        assert!(result.is_complete);
        assert!(result.missing_fields.is_empty());
    }

    #[test]
    fn test_feature_missing_use_case() {
        let body = r#"
## Proposed Behavior
An export button should generate a CSV file.

## Acceptance Criteria
- [ ] Export button works
"#;

        let result = check_feature_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::UseCase));
        assert!(
            !result
                .missing_fields
                .contains(&CanonicalField::ProposedBehavior)
        );
        assert!(
            !result
                .missing_fields
                .contains(&CanonicalField::AcceptanceCriteria)
        );
    }

    #[test]
    fn test_feature_missing_proposed_behavior() {
        let body = r#"
## Use Case
I need a way to export data.

## Acceptance Criteria
- [ ] Export button works
"#;

        let result = check_feature_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ProposedBehavior)
        );
    }

    #[test]
    fn test_feature_missing_acceptance_criteria() {
        let body = r#"
## Use Case
I need to export data to CSV.

## Proposed Behavior
A button should generate and download the file.
"#;

        let result = check_feature_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::AcceptanceCriteria)
        );
    }

    #[test]
    fn test_feature_missing_all_fields() {
        let body = "I have an idea";

        let result = check_feature_completeness(body);
        assert!(!result.is_complete);
        assert_eq!(result.missing_fields.len(), 3);
    }

    #[test]
    fn test_feature_empty_use_case() {
        let body = r#"
## Use Case

## Proposed Behavior
Something cool.

## Acceptance Criteria
- [ ] It works
"#;

        let result = check_feature_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::UseCase));
    }

    #[test]
    fn test_feature_placeholder_treated_as_missing() {
        let body = r#"
## Use Case
[type here]

## Proposed Behavior
[fill in]

## Acceptance Criteria
- [ ] TODO
"#;

        let result = check_feature_completeness(body);
        assert!(!result.is_complete);
        assert!(result.missing_fields.contains(&CanonicalField::UseCase));
        assert!(
            result
                .missing_fields
                .contains(&CanonicalField::ProposedBehavior)
        );
    }

    #[test]
    fn test_feature_with_all_3_fields_ready_for_review() {
        let body = r#"
## Use Case
I want to track tasks across multiple projects.

## Proposed Behavior
A unified task list that aggregates tasks from all connected projects.

## Acceptance Criteria
- [ ] Tasks from all projects appear in unified list
- [ ] Filtering by project is supported
- [ ] Changes sync in real-time
"#;

        let result = check_feature_completeness(body);
        assert!(
            result.is_complete,
            "Feature with all 3 fields should be complete"
        );
        assert!(
            result.missing_fields.is_empty(),
            "No fields should be missing"
        );
        assert!(
            result.to_request_comment().is_none(),
            "No request comment when complete"
        );
    }
}
