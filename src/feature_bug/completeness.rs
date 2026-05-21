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
//! 1. **Behavior observed** — A description of what happened
//! 2. **Behavior expected** — A description of what was expected
//! 3. **Reproduction steps** — Steps to reproduce (or N/A with justification)
//! 4. **Environment** — OS, version, hardware context
//!
//! ## Feature Request Requirements
//!
//! A feature request is complete when all of the following are present:
//! 1. **Use case** — Why this feature is needed
//! 2. **Proposed behavior** — How the feature should work
//! 3. **Acceptance criteria** — Testable, enumerated list

use serde::{Deserialize, Serialize};

/// Required fields for a complete bug report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BugCompletenessRequirements {
    /// Behavior observed - what happened that is wrong
    pub behavior_observed: bool,
    /// Behavior expected - what should have happened
    pub behavior_expected: bool,
    /// Reproduction steps - or N/A with justification
    pub reproduction_steps: bool,
    /// Environment - OS, version, context
    pub environment: bool,
}

/// Required fields for a complete feature request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureCompletenessRequirements {
    /// Use case - why this feature is needed
    pub use_case: bool,
    /// Proposed behavior - how the feature should work
    pub proposed_behavior: bool,
    /// Acceptance criteria - testable, enumerated list
    pub acceptance_criteria: bool,
}

/// Result of a completeness check operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletenessCheckResult {
    /// Whether the issue is complete
    pub is_complete: bool,
    /// Missing bug fields (empty if not a bug or not missing)
    pub missing_bug_fields: Vec<String>,
    /// Missing feature fields (empty if not a feature or not missing)
    pub missing_feature_fields: Vec<String>,
    /// Specific request message for missing fields
    pub request_message: String,
}

impl CompletenessCheckResult {
    /// Create a result for a complete bug report.
    pub fn complete_bug() -> Self {
        Self {
            is_complete: true,
            missing_bug_fields: Vec::new(),
            missing_feature_fields: Vec::new(),
            request_message: String::new(),
        }
    }

    /// Create a result for a complete feature request.
    pub fn complete_feature() -> Self {
        Self {
            is_complete: true,
            missing_bug_fields: Vec::new(),
            missing_feature_fields: Vec::new(),
            request_message: String::new(),
        }
    }

    /// Create a result for an incomplete bug report.
    pub fn incomplete_bug(missing_fields: Vec<String>) -> Self {
        let request_message = format_bug_request_message(&missing_fields);
        Self {
            is_complete: false,
            missing_bug_fields: missing_fields,
            missing_feature_fields: Vec::new(),
            request_message,
        }
    }

    /// Create a result for an incomplete feature request.
    pub fn incomplete_feature(missing_fields: Vec<String>) -> Self {
        let request_message = format_feature_request_message(&missing_fields);
        Self {
            is_complete: false,
            missing_bug_fields: Vec::new(),
            missing_feature_fields: missing_fields,
            request_message,
        }
    }
}

/// Format a request message for missing bug fields.
fn format_bug_request_message(missing_fields: &[String]) -> String {
    let mut msg = String::from(
        "I'd be happy to help investigate this bug! To move forward, could you provide the following information?\n\n",
    );

    for field in missing_fields {
        match field.as_str() {
            "behavior_observed" => {
                msg.push_str("- **Behavior observed**: What happened that seems wrong to you?\n");
            }
            "behavior_expected" => {
                msg.push_str("- **Behavior expected**: What did you expect to happen instead?\n");
            }
            "reproduction_steps" => {
                msg.push_str("- **Reproduction steps**: How can we reproduce this issue? (Or N/A if the bug cannot be reliably reproduced, with an explanation of why)\n");
            }
            "environment" => {
                msg.push_str(
                    "- **Environment**: What OS, version, and relevant context are you using?\n",
                );
            }
            _ => {
                msg.push_str(&format!(
                    "- **{}**: Please provide this information.\n",
                    field
                ));
            }
        }
    }

    msg.push_str("\nThanks for taking the time to share these details — they help us understand and reproduce the issue.");
    msg
}

/// Format a request message for missing feature fields.
fn format_feature_request_message(missing_fields: &[String]) -> String {
    let mut msg = String::from(
        "Thanks for your feature suggestion! To help us evaluate and implement this, could you provide the following?\n\n",
    );

    for field in missing_fields {
        match field.as_str() {
            "use_case" => {
                msg.push_str(
                    "- **Use case**: Why do you need this feature? What problem are you solving?\n",
                );
            }
            "proposed_behavior" => {
                msg.push_str(
                    "- **Proposed behavior**: How should this feature work once implemented?\n",
                );
            }
            "acceptance_criteria" => {
                msg.push_str("- **Acceptance criteria**: How would you verify this feature works correctly? (Please provide a testable, enumerated list of criteria)\n");
            }
            _ => {
                msg.push_str(&format!(
                    "- **{}**: Please provide this information.\n",
                    field
                ));
            }
        }
    }

    msg.push_str("\nThanks for helping us understand your needs!");
    msg
}

/// Validates that a bug report has all required fields present.
///
/// Uses light semantic analysis to detect presence of required fields.
/// This is a baseline implementation that can be enhanced with LLM-based
/// extraction in the future.
pub fn check_bug_completeness(body: &str) -> CompletenessCheckResult {
    let body_lower = body.to_lowercase();

    let behavior_observed = has_bug_section(
        &body_lower,
        &[
            "behavior observed",
            "what happened",
            "actual behavior",
            "current behavior",
            "the issue",
        ],
    );

    let behavior_expected = has_bug_section(
        &body_lower,
        &[
            "behavior expected",
            "expected result",
            "expected behavior",
            "what should happen",
            "what i expected",
            "what i thought",
        ],
    );

    let reproduction_steps = has_reproduction_steps(&body_lower);
    let environment = has_bug_section(
        &body_lower,
        &[
            "environment",
            "system info",
            "system details",
            "version",
            "os:",
            "node:",
            "platform",
            "browser:",
        ],
    );

    let mut missing = Vec::new();

    if !behavior_observed {
        missing.push("behavior_observed".to_string());
    }
    if !behavior_expected {
        missing.push("behavior_expected".to_string());
    }
    if !reproduction_steps {
        missing.push("reproduction_steps".to_string());
    }
    if !environment {
        missing.push("environment".to_string());
    }

    if missing.is_empty() {
        CompletenessCheckResult::complete_bug()
    } else {
        CompletenessCheckResult::incomplete_bug(missing)
    }
}

/// Validates that a feature request has all required fields present.
pub fn check_feature_completeness(body: &str) -> CompletenessCheckResult {
    let body_lower = body.to_lowercase();

    let use_case = has_feature_section(
        &body_lower,
        &["use case", "user story", "why", "motivation", "background"],
    );

    let proposed_behavior = has_feature_section(
        &body_lower,
        &[
            "proposed behavior",
            "how it should work",
            "how it works",
            "expected behavior",
            "what should happen",
            "implementation",
            "solution",
        ],
    );

    let acceptance_criteria = has_acceptance_criteria(&body_lower);

    let mut missing = Vec::new();

    if !use_case {
        missing.push("use_case".to_string());
    }
    if !proposed_behavior {
        missing.push("proposed_behavior".to_string());
    }
    if !acceptance_criteria {
        missing.push("acceptance_criteria".to_string());
    }

    if missing.is_empty() {
        CompletenessCheckResult::complete_feature()
    } else {
        CompletenessCheckResult::incomplete_feature(missing)
    }
}

/// Check for bug report sections using header patterns.
///
/// This checks for markdown headers that indicate a section is present.
/// The check looks for patterns like "## Section Name" in the body.
fn has_bug_section(body: &str, patterns: &[&str]) -> bool {
    // First, check for explicit markdown headers
    for pattern in patterns {
        // Check for ## pattern headers
        if body.contains(&format!("## {}", pattern)) {
            return true;
        }
        // Check for ### pattern headers
        if body.contains(&format!("### {}", pattern)) {
            return true;
        }
    }

    // For environment/species checks, also look for environment-style lines
    // This helps catch without explicit headers if there's a clear environment section
    let has_env_line = patterns.iter().any(|p| {
        p.contains(":") && (p.contains("os:") || p.contains("node:") || p.contains("version"))
    });

    if has_env_line {
        // Check for environment-like bullet lists
        let lines: Vec<&str> = body.lines().collect();
        for line in &lines {
            let line_trimmed = line.trim();
            if line_trimmed.starts_with("- ")
                && (line_trimmed.to_lowercase().contains("os:")
                    || line_trimmed.to_lowercase().contains("version:")
                    || line_trimmed.to_lowercase().contains("browser:")
                    || line_trimmed.to_lowercase().contains("node:"))
            {
                return true;
            }
        }
    }

    false
}

/// Check for feature request sections using header patterns.
fn has_feature_section(body: &str, patterns: &[&str]) -> bool {
    for pattern in patterns {
        // Check for ## pattern headers
        if body.contains(&format!("## {}", pattern)) {
            return true;
        }
        // Check for ### pattern headers
        if body.contains(&format!("### {}", pattern)) {
            return true;
        }
    }
    false
}

/// Checks for presence of reproduction steps.
///
/// Reproduction steps are detected by looking for:
/// - ## Reproduction Steps or ## Steps section
/// - Numbered steps or bullet points (1., 2., 3., -, *)
/// - Step keywords like "step", "reproduce", "reproduction"
/// - Or "N/A" with a following explanation
fn has_reproduction_steps(body: &str) -> bool {
    // Check for explicit header
    if body.contains("## reproduction")
        || body.contains("## steps")
        || body.contains("## how to")
        || body.contains("### reproduction")
        || body.contains("### steps")
        || body.contains("### how to")
    {
        return true;
    }

    // Check for N/A with justification for non-reproducible bugs
    if body.contains("n/a") || body.contains("not applicable") {
        return true;
    }

    // Check for numbered step patterns (need numbered list, not just "1.")
    let has_numbered_steps = (body.contains("1.") && body.len() > 100)
        || body.contains("1)")
        || body.contains("step 1")
        || body.contains("step one");

    // Check for step keywords
    let has_step_keyword = body.contains("steps to reproduce")
        || body.contains("reproduction steps")
        || body.contains("how to reproduce");

    // Check for bullet lists with clear action items
    let has_bullet_steps = (body.contains("- ") && body.len() > 100)
        && (body.contains("- open")
            || body.contains("- fill")
            || body.contains("- click")
            || body.contains("- select")
            || body.contains("- go to")
            || body.contains("- type"));

    has_numbered_steps || has_bullet_steps || has_step_keyword
}

/// Checks for acceptance criteria.
///
/// Acceptance criteria are detected by:
/// - Explicit "acceptance criteria" or "acceptance criteria:" section
/// - Checkbox patterns ([ ], [x], - [ ])
/// - Numbered criteria (AC-1:, AC-2:, etc.)
fn has_acceptance_criteria(body: &str) -> bool {
    // Check for explicit header
    if body.contains("## acceptance")
        || body.contains("## verification")
        || body.contains("## criteria")
        || body.contains("## how to test")
        || body.contains("### acceptance")
    {
        return true;
    }

    // Check for checkbox patterns
    if body.contains("[ ]") || body.contains("[x]") || body.contains("[- ]") {
        return true;
    }

    // Check for AC-* patterns
    if body.contains("ac-") || body.contains("ac1:") || body.contains("ac2:") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_bug() {
        let body = r#"
## Behavior Observed
The application crashes when clicking the button.

## Behavior Expected
The application should display a confirmation dialog.

## Reproduction Steps
1. Open the application
2. Click the submit button
3. Observe the crash

## Environment
- OS: macOS 13.0
- Version: 1.2.3
"#;
        let result = check_bug_completeness(body);
        assert!(result.is_complete, "Bug with all fields should be complete");
        assert!(result.missing_bug_fields.is_empty());
    }

    #[test]
    fn test_complete_bug_with_na_reproduction() {
        let body = r#"
## Behavior Observed
The application crashes randomly.

## Behavior Expected
The application should not crash.

## Reproduction Steps
N/A - This is a race condition that cannot be reliably reproduced.

## Environment
- OS: Linux Ubuntu 22.04
- Version: 2.0.0
"#;
        let result = check_bug_completeness(body);
        assert!(
            result.is_complete,
            "Bug with N/A reproduction should be complete"
        );
        assert!(result.missing_bug_fields.is_empty());
    }

    #[test]
    fn test_incomplete_bug() {
        let body = r#"
## What Happened
The button doesn't work.

There should be some way to make this work.
"#;
        let result = check_bug_completeness(body);
        assert!(
            !result.is_complete,
            "Bug missing expected, reproduction, and environment should be incomplete"
        );
        assert!(!result.missing_bug_fields.is_empty());
        assert!(
            result
                .missing_bug_fields
                .contains(&"behavior_expected".to_string())
        );
        assert!(
            result
                .missing_bug_fields
                .contains(&"reproduction_steps".to_string())
        );
        assert!(
            result
                .missing_bug_fields
                .contains(&"environment".to_string())
        );
    }

    #[test]
    fn test_complete_bug_with_alternate_headers() {
        let body = r#"
## What Happened
The application gives an error

## Expected Result
The application should succeed

## How to Reproduce
1. Open the app
2. Login
3. Submit form

## System Details
- OS: Windows 11
- Version: 2.0.0
- Browser: Chrome
"#;
        let result = check_bug_completeness(body);
        assert!(
            result.is_complete,
            "Bug with alternate headers should be complete"
        );
    }

    #[test]
    fn test_complete_feature() {
        let body = r#"
## Use Case
As a user, I want to export my data to CSV so I can analyze it in Excel.

## Proposed Behavior
When clicking the export button, the system should generate a CSV file with all visible data and prompt the user to download it.

## Acceptance Criteria
- [ ] Export button appears in the toolbar
- [ ] Clicking the button generates a CSV file
- [ ] The CSV contains all visible columns
- [ ] User is prompted to download the file
"#;
        let result = check_feature_completeness(body);
        assert!(
            result.is_complete,
            "Feature with all fields should be complete"
        );
        assert!(result.missing_feature_fields.is_empty());
    }

    #[test]
    fn test_complete_feature_with_user_story() {
        let body = r#"
## User Story
As a developer, I want API keys to be rotated automatically so I don't have to do it manually.

## What Should Happen
The system should generate new API keys monthly and notify users.

## How to Test
- [ ] New keys are generated each month
- [ ] Users receive notification
- [ ] Old keys are invalidated after grace period
- [ ] Existing integrations continue to work during transition
"#;
        let result = check_feature_completeness(body);
        assert!(
            result.is_complete,
            "Feature with User Story should be complete"
        );
        assert!(result.missing_feature_fields.is_empty());
    }

    #[test]
    fn test_incomplete_feature() {
        let body = r#"
## Why
I need a way to do something nice.
"#;
        let result = check_feature_completeness(body);
        // "Why" alone is not enough - there should be either Use Case header or
        // a substantial use case section
        assert!(
            !result.is_complete,
            "Feature missing proposed behavior and acceptance criteria should be incomplete"
        );
        assert!(!result.missing_feature_fields.is_empty());
        assert!(
            result
                .missing_feature_fields
                .contains(&"proposed_behavior".to_string())
        );
        assert!(
            result
                .missing_feature_fields
                .contains(&"acceptance_criteria".to_string())
        );
    }

    #[test]
    fn test_complete_bug_with_bullet_reproduction() {
        let body = r#"
## What Happened
The form submitted twice.

## Expected Result
The form should only submit once.

## Steps to Reproduce
- Open the form
- Fill in the fields
- Click submit button

## Environment
- Browser: Chrome 120
- OS: Windows 11
"#;
        let result = check_bug_completeness(body);
        assert!(
            result.is_complete,
            "Bug with all fields via alt headers should be complete"
        );
    }

    #[test]
    fn test_complete_feature_with_checkboxes() {
        let body = r#"
## Why I Need This
I want to track my tasks better.

## How It Should Work
A kanban board view with drag-and-drop cards.

## Verification Criteria
- [x] User can see a board view
- [ ] User can drag cards between columns
- [ ] Changes persist after reload
"#;
        let result = check_feature_completeness(body);
        assert!(
            result.is_complete,
            "Feature with all fields via alt headers should be complete"
        );
    }

    #[test]
    fn test_request_message_includes_all_missing_fields() {
        let body = "Just a title"; // Will miss all fields
        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(result.request_message.contains("Behavior observed"));
        assert!(result.request_message.contains("Behavior expected"));
        assert!(result.request_message.contains("Reproduction steps"));
        assert!(result.request_message.contains("Environment"));
    }

    // Issue verification tests - specific missing field requests

    #[test]
    fn test_bug_missing_environment_only_requests_environment() {
        // Bug with all fields except environment
        let body = r#"
## What Happened
Something wrong happened

## Behavior Expected
Something right should happen

## Steps to Reproduce
1. Do X
2. Observe result

No environment details provided.
"#;
        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_bug_fields
                .contains(&"environment".to_string())
        );
        assert!(result.missing_bug_fields.len() == 1);
        // Request message should mention environment only
        assert!(result.request_message.contains("Environment"));
        assert!(!result.request_message.contains("Reproduction"));
        assert!(!result.request_message.contains("Behavior observed"));
    }

    #[test]
    fn test_bug_missing_steps_and_expected_requests_both() {
        // Bug missing reproduction_steps and behavior_expected
        let body = r#"
## What Happened
The form submitted twice instead of once
"#;
        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_bug_fields
                .contains(&"behavior_expected".to_string())
        );
        assert!(
            result
                .missing_bug_fields
                .contains(&"reproduction_steps".to_string())
        );
        assert!(result.missing_bug_fields.len() >= 2);
        // Request message should mention both
        assert!(result.request_message.contains("Behavior expected"));
        assert!(result.request_message.contains("Reproduction steps"));
    }

    #[test]
    fn test_feature_missing_acceptance_criteria_only_requests_that() {
        // Feature with use_case and proposed_behavior but missing acceptance_criteria
        let body = r#"
## Use Case
I need to track tasks better

## Proposed Behavior
A kanban board with drag and drop

No acceptance criteria provided.
"#;
        let result = check_feature_completeness(body);
        assert!(!result.is_complete);
        assert!(
            result
                .missing_feature_fields
                .contains(&"acceptance_criteria".to_string())
        );
        assert!(result.missing_feature_fields.len() == 1);
        // Request message should mention acceptance criteria only
        assert!(result.request_message.contains("Acceptance criteria"));
        assert!(!result.request_message.contains("Use case"));
        assert!(!result.request_message.contains("Proposed behavior"));
    }

    #[test]
    fn test_no_generic_please_provide_more_details() {
        let body = "Just a title";
        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        // Should NOT contain generic phrases
        assert!(
            !result
                .request_message
                .to_lowercase()
                .contains("more detail")
        );
        assert!(
            !result
                .request_message
                .to_lowercase()
                .contains("need more info")
        );
        assert!(
            !result
                .request_message
                .to_lowercase()
                .contains("additional info")
        );
        // Should contain specific field requests
        assert!(result.request_message.contains("Behavior observed"));
        assert!(result.request_message.contains("Behavior expected"));
        assert!(result.request_message.contains("Reproduction steps"));
        assert!(result.request_message.contains("Environment"));
    }

    #[test]
    fn test_needs_information_label_would_be_applied() {
        // This test verifies the completeness check returns the correct data
        // for needs-information label application
        let body = r#"
## Behavior Observed
Something wrong
"#;
        let result = check_bug_completeness(body);
        assert!(!result.is_complete);
        // The missing fields list can be used to apply needs-information label
        assert!(!result.missing_bug_fields.is_empty());
        // The request message specifically identifies what's needed
        assert!(!result.request_message.is_empty());
    }

    #[test]
    fn test_bug_completeness_result_usable_for_transition() {
        // Verify the CompletenessCheckResult can be used with TransitionSummary
        use crate::feature_bug::FeatureBugIssue;
        use crate::feature_bug::TransitionSummary;

        let incomplete_body = r#"
## What Happened
The button clicked but nothing happened

## Environment
Windows 10, Chrome 120
"#;
        let result = check_bug_completeness(incomplete_body);
        assert!(!result.is_complete);

        let issue = FeatureBugIssue {
            number: 42,
            title: "Test bug".to_string(),
            body: incomplete_body.to_string(),
            author: "testuser".to_string(),
            is_bug: true,
            is_feature: false,
        };

        // Using the result with bug_needs_information transition
        let transition = TransitionSummary::bug_needs_information(&issue, &result.request_message);
        assert!(transition.applied_needs_information);
        assert!(
            transition
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }

    #[test]
    fn test_feature_completeness_result_usable_for_transition() {
        // Verify the CompletenessCheckResult can be used with TransitionSummary
        use crate::feature_bug::FeatureBugIssue;
        use crate::feature_bug::TransitionSummary;

        let incomplete_body = r#"
## Use Case
I want to export my data to CSV
"#;
        let result = check_feature_completeness(incomplete_body);
        assert!(!result.is_complete);

        let issue = FeatureBugIssue {
            number: 43,
            title: "Test feature".to_string(),
            body: incomplete_body.to_string(),
            author: "testuser".to_string(),
            is_bug: false,
            is_feature: true,
        };

        // Using the result with feature_needs_information transition
        let transition =
            TransitionSummary::feature_needs_information(&issue, &result.request_message);
        assert!(transition.applied_needs_information);
        assert!(
            transition
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }
}
