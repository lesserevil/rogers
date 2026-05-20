//! Feature and bug issue handling for Rodgers.
//!
//! This module implements the transition logic for bug and feature issues
//! as defined in plans/feature-bug-plan.md. It handles:
//!
//! - Completeness verification (see `completeness` module)
//! - Will-not-do handling (see `will_not_do` module)
//! - Breakdown for ready-for-work (see `breakdown` module)
//! - Transition to ready-for-review when complete
//! - Application of needs-information when incomplete
//! - Generating summary comments and acceptance criteria
//!
//! ## Standalone Bead Validation (CRIT-5)
//!
//! Every child bead is validated for standalone criteria per AGENTS.md §Beads must stand alone.
//! Beads used for implementation MUST be standalone-ready before being filed.

mod breakdown;
mod completeness;
pub mod will_not_do;

pub use breakdown::{
    BeadValidationResult, BreakdownResult, StandaloneBead, StandaloneIssue, StandaloneValidation,
    analyze_epic_scale, execute_breakdown, validate_beads_standalone,
};
pub use completeness::{
    CompletenessCheckResult, check_bug_completeness, check_feature_completeness,
};

// =============================================================================
// Standalone Bead Validation (CRIT-5)
// =============================================================================

/// Validate that a bead description contains all 5 required standalone sections.
///
/// Required sections per AGENTS.md §Beads must stand alone:
/// - WHAT TO DO: Concrete files, packages, functions, or commands
/// - WHY: User-visible behavior, constraint, or design rule
/// - HOW TO VERIFY: Test, command, or observable result
/// - EDGE CASES: Non-obvious constraints a careful reader could miss
/// - PROJECT-SPECIFIC TERMINOLOGY: Terms that only make sense in context
pub fn validate_standalone_sections(description: &str) -> StandaloneSectionValidation {
    let description_upper = description.to_uppercase();

    let has_what = description_upper.contains("WHAT TO DO");
    let has_why = description_upper.contains("WHY");
    let has_how = description_upper.contains("HOW TO VERIFY");
    let has_edge = description_upper.contains("EDGE CASES")
        || description_upper.contains("EDGE CASES AND PITFALLS");
    let has_terms = description_upper.contains("PROJECT-SPECIFIC TERMINOLOGY")
        || description_upper.contains("TERMINOLOGY");

    let missing_sections: Vec<String> = {
        let mut v = Vec::new();
        if !has_what {
            v.push("WHAT TO DO".to_string());
        }
        if !has_why {
            v.push("WHY".to_string());
        }
        if !has_how {
            v.push("HOW TO VERIFY".to_string());
        }
        if !has_edge {
            v.push("EDGE CASES".to_string());
        }
        if !has_terms {
            v.push("TERMINOLOGY".to_string());
        }
        v
    };

    let all_present = missing_sections.is_empty();

    StandaloneSectionValidation {
        has_what,
        has_why,
        has_how,
        has_edge,
        has_terms,
        missing_sections,
        all_present,
    }
}

/// Result of validating standalone sections in a bead description.
#[derive(Debug, Clone, Default)]
pub struct StandaloneSectionValidation {
    /// Whether WHAT TO DO section is present
    pub has_what: bool,
    /// Whether WHY section is present
    pub has_why: bool,
    /// Whether HOW TO VERIFY section is present
    pub has_how: bool,
    /// Whether EDGE CASES section is present
    pub has_edge: bool,
    /// Whether TERMINOLOGY section is present
    pub has_terms: bool,
    /// List of missing section headers
    pub missing_sections: Vec<String>,
    /// Whether all sections are present
    pub all_present: bool,
}

impl StandaloneSectionValidation {
    /// Check if validation passed.
    pub fn passed(&self) -> bool {
        self.all_present
    }
}

/// Validate that a bead description has no compound "...and then..." patterns.
///
/// Compound beads should be split into separate beads per the breakdown rules.
pub fn validate_no_compound_pattern(description: &str) -> CompoundPatternValidation {
    let desc_lower = description.to_lowercase();

    // Check for various compound patterns
    let has_and_then = desc_lower.contains("and then");
    let has_sequential_first_second =
        desc_lower.contains("first ") && desc_lower.contains("second ");
    // More strict - require explicit "step N:" pattern
    let has_step_pattern = (desc_lower.contains("step 1:")
        || desc_lower.contains("step one:")
        || desc_lower.contains("\nstep 1 "))
        && (desc_lower.contains("step 2:")
            || desc_lower.contains("step two:")
            || desc_lower.contains("\nstep 2 ")
            || desc_lower.contains("step three:"));

    let has_compound = has_and_then || has_sequential_first_second || has_step_pattern;

    let suggestion = if has_compound {
        "Split this bead into separate beads. Each bead should cover one logical unit of work."
            .to_string()
    } else {
        String::new()
    };

    let patterns_found: Vec<String> = {
        let mut v = Vec::new();
        if has_and_then {
            v.push("'and then' sequential pattern".to_string());
        }
        if has_sequential_first_second {
            v.push("'first/second' sequential pattern".to_string());
        }
        if has_step_pattern {
            v.push("'step 1:/step 2:' sequential pattern".to_string());
        }
        v
    };

    CompoundPatternValidation {
        has_compound_pattern: has_compound,
        patterns_found,
        suggestion,
    }
}

/// Result of validating for compound patterns.
#[derive(Debug, Clone, Default)]
pub struct CompoundPatternValidation {
    /// Whether a compound pattern was detected
    pub has_compound_pattern: bool,
    /// List of compound patterns found
    pub patterns_found: Vec<String>,
    /// Suggestion for fixing issues
    pub suggestion: String,
}

impl CompoundPatternValidation {
    /// Check if validation passed (no compound patterns).
    pub fn passed(&self) -> bool {
        !self.has_compound_pattern
    }
}

/// Validate that a bead touches a single codebase part.
///
/// A compound bead touching multiple areas (CLI + API + DB) should be split.
pub fn validate_single_codebase_part(description: &str) -> SinglePartValidation {
    let desc_lower = description.to_lowercase();

    let mut areas = Vec::new();

    if desc_lower.contains("cli")
        || desc_lower.contains("command-line")
        || desc_lower.contains("command-")
    {
        areas.push("CLI".to_string());
    }
    if desc_lower.contains("api") || desc_lower.contains("rest") || desc_lower.contains("endpoint")
    {
        areas.push("API".to_string());
    }
    if desc_lower.contains("database")
        || desc_lower.contains("db ")
        || desc_lower.contains("storage")
        || desc_lower.contains("persist")
    {
        areas.push("Database".to_string());
    }
    if desc_lower.contains("ui")
        || desc_lower.contains("dashboard")
        || desc_lower.contains("frontend")
        || desc_lower.contains("interface")
    {
        areas.push("UI".to_string());
    }
    if desc_lower.contains("config") || desc_lower.contains("settings") {
        areas.push("Config".to_string());
    }
    if desc_lower.contains("auth")
        || desc_lower.contains("permission")
        || desc_lower.contains("login")
    {
        areas.push("Auth".to_string());
    }

    let area_count = areas.len();
    let is_single = area_count <= 1
        || (area_count == 2
            && ((areas.contains(&"API".to_string()) || areas.contains(&"REST".to_string()))
                && (areas.contains(&"Database".to_string())
                    || areas.contains(&"storage".to_string())
                    || areas.contains(&"persist".to_string()))));

    let suggestion = if !is_single && area_count > 1 {
        "Split into separate beads per codebase area (CLI, API, DB, UI, Config, Auth)".to_string()
    } else {
        String::new()
    };

    SinglePartValidation {
        areas_detected: areas,
        is_single_codebase_part: is_single,
        suggestion,
    }
}

/// Result of validating for single codebase part.
#[derive(Debug, Clone, Default)]
pub struct SinglePartValidation {
    /// Distinct codebase areas detected
    pub areas_detected: Vec<String>,
    /// Whether only one area is present
    pub is_single_codebase_part: bool,
    /// Suggestion for fixing issues
    pub suggestion: String,
}

impl SinglePartValidation {
    /// Check if validation passed.
    pub fn passed(&self) -> bool {
        self.is_single_codebase_part
    }
}

/// Full standalone validation for a bead description.
///
/// Runs all standalone checks and returns combined results.
pub fn validate_bead_standalone(description: &str) -> FullStandaloneValidation {
    let sections = validate_standalone_sections(description);
    let compound = validate_no_compound_pattern(description);
    let single_part = validate_single_codebase_part(description);

    let all_passed = sections.passed() && compound.passed() && single_part.passed();

    FullStandaloneValidation {
        sections,
        compound,
        single_part,
        all_passed,
    }
}

/// Combined results of all standalone validations.
#[derive(Debug, Clone, Default)]
pub struct FullStandaloneValidation {
    /// Section presence validation
    pub sections: StandaloneSectionValidation,
    /// Compound pattern validation
    pub compound: CompoundPatternValidation,
    /// Single codebase part validation
    pub single_part: SinglePartValidation,
    /// Whether all checks passed
    pub all_passed: bool,
}

impl FullStandaloneValidation {
    /// Check if validation passed.
    pub fn passed(&self) -> bool {
        self.all_passed
    }

    /// Get human-readable summary of all issues.
    pub fn issue_summary(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if !self.sections.passed() {
            issues.push(format!(
                "Missing sections: {}",
                self.sections.missing_sections.join(", ")
            ));
        }

        if !self.compound.passed() {
            issues.push(format!(
                "Compound pattern detected: {}",
                self.compound.patterns_found.join(", ")
            ));
            issues.push(self.compound.suggestion.clone());
        }

        if !self.single_part.passed() {
            issues.push(format!(
                "Multiple codebase areas: {}",
                self.single_part.areas_detected.join(", ")
            ));
            issues.push(self.single_part.suggestion.clone());
        }

        issues
    }
}

use serde::{Deserialize, Serialize};

/// Represents a bug or feature issue that needs triaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureBugIssue {
    /// GitHub issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body (description)
    pub body: String,
    /// Author username
    pub author: String,
    /// Whether this is a bug report
    pub is_bug: bool,
    /// Whether this is a feature request
    pub is_feature: bool,
}

/// Represents the summary comment to be posted on the issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSummary {
    /// The comment body to post
    pub comment: String,
    /// Whether ready-for-review was applied
    pub applied_ready_for_review: bool,
    /// Whether needs-information was applied
    pub applied_needs_information: bool,
    /// Labels to add
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
}

impl TransitionSummary {
    /// Create a summary for a bug that is now ready for review.
    pub fn bug_ready_for_review(issue: &FeatureBugIssue) -> Self {
        let comment = generate_bug_summary(issue);
        Self {
            comment,
            applied_ready_for_review: true,
            applied_needs_information: false,
            labels_to_add: vec!["ready-for-review".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
        }
    }

    /// Create a summary for a feature that is now ready for review.
    pub fn feature_ready_for_review(issue: &FeatureBugIssue) -> Self {
        let comment = generate_feature_summary(issue);
        Self {
            comment,
            applied_ready_for_review: true,
            applied_needs_information: false,
            labels_to_add: vec!["ready-for-review".to_string()],
            labels_to_remove: vec!["needs-information".to_string()],
        }
    }

    /// Create a summary for an incomplete bug that needs more information.
    pub fn bug_needs_information(issue: &FeatureBugIssue, request: &str) -> Self {
        let comment = generate_needs_information_comment(issue, request);
        Self {
            comment,
            applied_ready_for_review: false,
            applied_needs_information: true,
            labels_to_add: vec!["needs-information".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
        }
    }

    /// Create a summary for an incomplete feature that needs more information.
    pub fn feature_needs_information(issue: &FeatureBugIssue, request: &str) -> Self {
        let comment = generate_needs_information_comment(issue, request);
        Self {
            comment,
            applied_ready_for_review: false,
            applied_needs_information: true,
            labels_to_add: vec!["needs-information".to_string()],
            labels_to_remove: vec!["ready-for-review".to_string()],
        }
    }
}

/// Generate the summary comment for a complete bug report.
fn generate_bug_summary(issue: &FeatureBugIssue) -> String {
    format!(
        r#"## Rodgers Triage Summary

Thank you for the detailed bug report, @{author}! I've reviewed the information provided and everything looks complete.

### Summary
- **Reported issue**: {title}
- **Status**: Ready for human review

I'll now mark this as ready for review. A human maintainer will evaluate this and either:
- Work on a fix if it fits the project priorities
- Or close it with an explanation if it's not something we can address

Thanks again for taking the time to report this! "#,
        author = issue.author,
        title = issue.title
    )
}

/// Generate the summary comment for a complete feature request.
fn generate_feature_summary(issue: &FeatureBugIssue) -> String {
    format!(
        r#"## Rodgers Triage Summary

Thanks for the feature request, @{author}! I've reviewed the information provided and everything looks complete.

### Summary
- **Requested feature**: {title}
- **Status**: Ready for human review

### Rodgers Generated Acceptance Criteria

{criteria}

I'll mark this as ready for review. A human maintainer will evaluate this request and either:
- Accept it for implementation if it aligns with project goals
- Or close it with an explanation if it's not something we can prioritize right now

Thanks for taking the time to share your ideas! "#,
        author = issue.author,
        title = issue.title,
        criteria = generate_acceptance_criteria(issue)
    )
}

/// Generate a preliminary acceptance criteria section.
///
/// This generates draft acceptance criteria from the issue content.
/// A human reviewer may accept, reject, or modify these before marking ready-for-work.
fn generate_acceptance_criteria(issue: &FeatureBugIssue) -> String {
    // This is a placeholder that would be enhanced with LLM-based extraction
    // For now, generate basic criteria based on whether it's a bug or feature
    if issue.is_bug {
        String::from(
            r#"- [ ] AC-1: The reported behavior is verified and understood
- [ ] AC-2: A fix is implemented that resolves the issue
- [ ] AC-3: Existing functionality is not broken by the fix"#,
        )
    } else {
        String::from(
            r#"- [ ] AC-1: The feature is implemented with the proposed behavior
- [ ] AC-2: Existing functionality is not broken
- [ ] AC-3: The feature meets the stated use case"#,
        )
    }
}

/// Generate the needs-information comment for incomplete issues.
fn generate_needs_information_comment(issue: &FeatureBugIssue, request: &str) -> String {
    let issue_type = if issue.is_bug {
        "bug report"
    } else {
        "feature request"
    };

    format!(
        r#"Hi @{author}, thanks for this {issue_type}!

To help us understand and work on this, could you provide a bit more information?

{request}

Thanks for taking the time to fill this out — the more context you provide, the better we can evaluate and address this!"#,
        author = issue.author,
        issue_type = issue_type,
        request = request
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue(is_bug: bool, is_feature: bool) -> FeatureBugIssue {
        FeatureBugIssue {
            number: 42,
            title: "Test Issue".to_string(),
            body: "Test body content".to_string(),
            author: "testuser".to_string(),
            is_bug,
            is_feature,
        }
    }

    #[test]
    fn test_bug_ready_for_review_transition() {
        let issue = create_test_issue(true, false);
        let summary = TransitionSummary::bug_ready_for_review(&issue);

        assert!(summary.applied_ready_for_review);
        assert!(!summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(
            summary
                .labels_to_remove
                .contains(&"needs-information".to_string())
        );
        assert!(summary.comment.contains("@testuser"));
    }

    #[test]
    fn test_feature_ready_for_review_transition() {
        let issue = create_test_issue(false, true);
        let summary = TransitionSummary::feature_ready_for_review(&issue);

        assert!(summary.applied_ready_for_review);
        assert!(!summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"ready-for-review".to_string())
        );
        assert!(summary.comment.contains("Acceptance Criteria"));
    }

    #[test]
    fn test_bug_needs_information_transition() {
        let issue = create_test_issue(true, false);
        let request = "Please add reproduction steps";
        let summary = TransitionSummary::bug_needs_information(&issue, request);

        assert!(!summary.applied_ready_for_review);
        assert!(summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
        assert!(
            summary
                .labels_to_remove
                .contains(&"ready-for-review".to_string())
        );
        assert!(summary.comment.contains("reproduction steps"));
    }

    #[test]
    fn test_feature_needs_information_transition() {
        let issue = create_test_issue(false, true);
        let request = "Please describe the use case";
        let summary = TransitionSummary::feature_needs_information(&issue, request);

        assert!(!summary.applied_ready_for_review);
        assert!(summary.applied_needs_information);
        assert!(
            summary
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
    }

    #[test]
    fn test_generated_acceptance_criteria_includes_bug_criteria() {
        let bug_issue = create_test_issue(true, false);
        let criteria = generate_acceptance_criteria(&bug_issue);

        assert!(criteria.contains("AC-1"));
        assert!(criteria.contains("fix"));
    }

    #[test]
    fn test_generated_acceptance_criteria_includes_feature_criteria() {
        let feature_issue = create_test_issue(false, true);
        let criteria = generate_acceptance_criteria(&feature_issue);

        assert!(criteria.contains("AC-1"));
        assert!(criteria.contains("feature"));
    }

    #[test]
    fn test_complete_bug_workflow() {
        use completeness::check_bug_completeness;

        let complete_body = r#"
## Behavior Observed
It crashes when X happens.

## Behavior Expected
It should not crash.

## Reproduction Steps
1. Do X
2. Observe crash

## Environment
macOS 13.0
"#;

        let issue = FeatureBugIssue {
            number: 1,
            title: "Test bug".to_string(),
            body: complete_body.to_string(),
            author: "reporter".to_string(),
            is_bug: true,
            is_feature: false,
        };

        let result = check_bug_completeness(&issue.body);
        assert!(result.is_complete);

        let transition = TransitionSummary::bug_ready_for_review(&issue);
        assert!(transition.applied_ready_for_review);
    }

    #[test]
    fn test_complete_feature_workflow() {
        use completeness::check_feature_completeness;

        let complete_body = r#"
## Use Case
I need this to solve problem X.

## Proposed Behavior
It should do Y.

## Acceptance Criteria
- [ ] It does Y
- [ ] It works well
"#;

        let issue = FeatureBugIssue {
            number: 2,
            title: "Test feature".to_string(),
            body: complete_body.to_string(),
            author: "requester".to_string(),
            is_bug: false,
            is_feature: true,
        };

        let result = check_feature_completeness(&issue.body);
        assert!(result.is_complete);

        let transition = TransitionSummary::feature_ready_for_review(&issue);
        assert!(transition.applied_ready_for_review);
    }

    #[test]
    fn test_incomplete_bug_workflow_in_one_run() {
        use completeness::check_bug_completeness;

        let incomplete_body = r#"
## Behavior Observed
It crashes sometimes.
"#;

        let issue = FeatureBugIssue {
            number: 3,
            title: "Incomplete bug".to_string(),
            body: incomplete_body.to_string(),
            author: "reporter".to_string(),
            is_bug: true,
            is_feature: false,
        };

        let result = check_bug_completeness(&issue.body);
        assert!(!result.is_complete);
        assert!(!result.missing_bug_fields.is_empty());

        let transition = TransitionSummary::bug_needs_information(&issue, &result.request_message);
        assert!(transition.applied_needs_information);
    }

    #[test]
    fn test_no_delay_in_transition() {
        // This test verifies that the transition logic completes within a single run
        // by ensuring all operations are synchronous and deterministic

        let issue = create_test_issue(true, false);
        let complete_body = r#"
## What Happened
Something wrong.

## What Should Happen
Something right.

## Steps
1. Step 1

## Environment
- OS: Linux
"#;

        let issue = FeatureBugIssue {
            number: 1,
            title: issue.title,
            body: complete_body.to_string(),
            author: issue.author,
            is_bug: true,
            is_feature: false,
        };

        // Completeness check
        let result = check_bug_completeness(&issue.body);

        // Transition decision - should be immediate
        let transition = if result.is_complete {
            TransitionSummary::bug_ready_for_review(&issue)
        } else {
            TransitionSummary::bug_needs_information(&issue, &result.request_message)
        };

        // Both operations should complete in this single run
        assert!(result.is_complete);
        assert!(transition.applied_ready_for_review);
    }

    // =============================================================================
    // Standalone Bead Validation Tests (CRIT-5)
    // =============================================================================

    #[test]
    fn test_validate_standalone_sections_all_present() {
        let desc = r#"
WHAT TO DO
Create the user API endpoint.

WHY
Users need to view their profiles.

HOW TO VERIFY
curl /api/users/1 returns JSON.

EDGE CASES AND PITFALLS
Handle missing user gracefully.

PROJECT-SPECIFIC TERMINOLOGY
None.
"#;
        let result = validate_standalone_sections(desc);
        assert!(result.passed());
        assert!(result.all_present);
        assert!(result.missing_sections.is_empty());
    }

    #[test]
    fn test_validate_standalone_sections_missing_one() {
        let desc = r#"
WHAT TO DO
Create the API.

WHY
Users need this.

HOW TO VERIFY
curl works.

PROJECT-SPECIFIC TERMINOLOGY
None.
"#;
        let result = validate_standalone_sections(desc);
        assert!(!result.passed());
        assert!(!result.all_present);
        assert!(result.missing_sections.contains(&"EDGE CASES".to_string()));
    }

    #[test]
    fn test_validate_standalone_sections_case_insensitive() {
        let desc = r#"
what to do
Create this.

why
Because.

how to verify
Run test.

edge cases
None.

terminology
None.
"#;
        let result = validate_standalone_sections(desc);
        // Should detect uppercase headers even when description uses different case
        assert!(result.has_what);
    }

    #[test]
    fn test_validate_no_compound_pattern_clean() {
        let desc = "Implement the database schema for user storage.";
        let result = validate_no_compound_pattern(desc);
        assert!(result.passed());
        assert!(!result.has_compound_pattern);
    }

    #[test]
    fn test_validate_no_compound_pattern_and_then() {
        let desc = "First, create the database schema, and then add the API endpoints.";
        let result = validate_no_compound_pattern(desc);
        assert!(!result.passed());
        assert!(result.has_compound_pattern);
        assert!(
            result
                .patterns_found
                .contains(&"'and then' sequential pattern".to_string())
        );
    }

    #[test]
    fn test_validate_no_compound_pattern_first_second() {
        let desc = "First implement auth, second add the UI.";
        let result = validate_no_compound_pattern(desc);
        assert!(!result.passed());
        assert!(result.has_compound_pattern);
    }

    #[test]
    fn test_validate_no_compound_pattern_step_numbers() {
        let desc = "Step 1: Create schema. Step 2: Add data.";
        let result = validate_no_compound_pattern(desc);
        assert!(!result.passed());
        assert!(result.has_compound_pattern);
    }

    #[test]
    fn test_validate_no_compound_pattern_numbered_list_ok() {
        // Numbered list items without "step" prefix should NOT be considered compound
        let desc = "1. First do this.\n2. Then do that.";
        let result = validate_no_compound_pattern(desc);
        assert!(result.passed());
        assert!(!result.has_compound_pattern);
    }

    #[test]
    fn test_validate_single_codebase_part_api_only() {
        let desc = "Implement the user API endpoint.";
        let result = validate_single_codebase_part(desc);
        assert!(result.passed());
        assert!(result.is_single_codebase_part);
        assert_eq!(result.areas_detected, vec!["API"]);
    }

    #[test]
    fn test_validate_single_codebase_part_multiple_areas() {
        let desc = "Update both the CLI commands and the API endpoints and the database.";
        let result = validate_single_codebase_part(desc);
        assert!(!result.passed());
        assert!(!result.is_single_codebase_part);
        assert!(result.areas_detected.contains(&"CLI".to_string()));
        assert!(result.areas_detected.contains(&"API".to_string()));
        assert!(result.areas_detected.contains(&"Database".to_string()));
    }

    #[test]
    fn test_validate_single_codebase_part_api_and_db_allowed() {
        // API + Database are closely related, should be allowed
        let desc = "API handler that queries the database.";
        let result = validate_single_codebase_part(desc);
        assert!(result.passed());
    }

    #[test]
    fn test_validate_bead_standalone_full_pass() {
        let desc = r#"WHAT TO DO
Implement the user profile API endpoint in src/api/users.rs.

WHY
Users need to view and update their profiles via REST API.

HOW TO VERIFY
Run `cargo test` - all tests pass. Curl the endpoint to verify it returns JSON with user data.

EDGE CASES AND PITFALLS
- Handle missing user (404 response)
- Validate input (400 for invalid data)
- Rate limit requests (429 when exceeded)

PROJECT-SPECIFIC TERMINOLOGY
**REST API endpoint**: HTTP resource URL that accepts JSON requests.
"#;
        let result = validate_bead_standalone(desc);
        assert!(result.passed());
    }

    #[test]
    fn test_validate_bead_standalone_missing_sections() {
        let desc = r#"WHAT TO DO
Implement API endpoint.

No WHY section here.
"#;
        let result = validate_bead_standalone(desc);
        assert!(!result.passed());
        assert!(!result.sections.passed());
    }

    #[test]
    fn test_validate_bead_standalone_compound_pattern() {
        let desc = r#"WHAT TO DO
Update CLI, and then add API support, and then fix DB.

WHY
Need all features.

HOW TO VERIFY
Test each part.

EDGE CASES
Handle errors.

TERMINOLOGY
CLI: command-line interface.
"#;
        let result = validate_bead_standalone(desc);
        assert!(!result.passed());
        assert!(!result.compound.passed());
    }

    #[test]
    fn test_validate_bead_standalone_multiple_areas() {
        let desc = r#"WHAT TO DO
CLI and UI and API and Database all need updates.

WHY
Multi-area feature.

HOW TO VERIFY
Test each area.

EDGE CASES
Handle all.

TERMINOLOGY
None.
"#;
        let result = validate_bead_standalone(desc);
        assert!(!result.passed());
        assert!(!result.single_part.passed());
    }

    #[test]
    fn test_full_validation_issue_summary() {
        let desc = r#"Missing sections and has compound patterns."#;
        let result = validate_bead_standalone(desc);
        let summary = result.issue_summary();
        assert!(!summary.is_empty());
    }
}
