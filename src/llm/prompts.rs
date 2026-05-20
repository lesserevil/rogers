//! LLM prompts for Rodgers issue analysis.
//!
//! This module contains prompts used with the LLM to analyze freeform issues
//! and extract specific field information. It complements the heuristic
//! pattern matching in feature_bug/completeness.rs:
//!
//! - Template-based issues: Pattern matching (existing code)
//! - Freeform issues: LLM-based extraction (this module)
//!
//! ## Design
//!
//! The LLM is used to identify which required fields are present or missing
//! from the issue content. For each missing field, a specific request is
//! generated rather than a generic "please provide more information".
//!
//! ## Prompt Architecture
//!
//! 1. **Extract prompt**: Given issue content, identify present fields
//! 2. **Missing fields prompt**: Given result, determine what's missing
//! 3. **Request generation prompt**: Given missing fields, format specific request

use serde::{Deserialize, Serialize};

/// Prompt for extracting fields from a bug report freeform description.
///
/// Returns structured information about which required fields are present.
pub const BUG_FIELD_EXTRACTION_PROMPT: &str = r#"You are analyzing a bug report. Identify which of the following required fields are present in the issue content.

Required fields for a complete bug report:
1. **Behavior observed** - A description of what happened that is wrong
2. **Behavior expected** - A description of what the reporter expected
3. **Reproduction steps** - Steps to reproduce the issue (or N/A with justification)
4. **Environment** - OS, version, hardware, browser, etc.

Analyze the issue content and respond with a JSON object indicating which fields are present.
Return ONLY the JSON object, no additional text.

Example response:
{"behavior_observed": true, "behavior_expected": true, "reproduction_steps": false, "environment": false}

Issue content:
{issue_content}
"#;

/// Prompt for extracting fields from a feature request freeform description.
///
/// Returns structured information about which required fields are present.
pub const FEATURE_FIELD_EXTRACTION_PROMPT: &str = r#"You are analyzing a feature request. Identify which of the following required fields are present in the issue content.

Required fields for a complete feature request:
1. **Use case** - Why the requester needs this feature (the problem they are solving)
2. **Proposed behavior** - How the feature should work once implemented
3. **Acceptance criteria** - How the feature would be verified (testable, enumerated list)

Analyze the issue content and respond with a JSON object indicating which fields are present.
Return ONLY the JSON object, no additional text.

Example response:
{"use_case": true, "proposed_behavior": true, "acceptance_criteria": false}

Issue content:
{issue_content}
"#;

/// Prompt for generating specific requests for missing bug report fields.
pub const BUG_MISSING_FIELDS_REQUEST_PROMPT: &str = r#"The following required fields are missing from this bug report:
{missing_fields}

Generate a friendly, specific request comment asking the user to provide ONLY the missing fields.
Do NOT request fields that are already present. Do NOT use generic phrases like "please provide more details".
Ask specifically for each missing field with a brief explanation of why it's needed.

Bug missing fields:
{bug_missing}

Example for missing environment and reproduction_steps:
"Thanks for the report! To help us reproduce this issue, could you provide:
- **Reproduction steps**: How can we reproduce what you're seeing?
- **Environment**: What OS, version, and relevant context are you using?"

Respond with ONLY the comment text.
"#;

/// Prompt for generating specific requests for missing feature request fields.
pub const FEATURE_MISSING_FIELDS_REQUEST_PROMPT: &str = r#"The following required fields are missing from this feature request:
{missing_fields}

Generate a friendly, specific request comment asking the user to provide ONLY the missing fields.
Do NOT request fields that are already present. Do NOT use generic phrases like "please provide more details".
Ask specifically for each missing field with a brief explanation of why it's needed.

Feature missing fields:
{feature_missing}

Example for missing use_case and acceptance_criteria:
"Thanks for the feature suggestion! To help us evaluate this, could you provide:
- **Use case**: Why do you need this feature? What problem are you solving?
- **Acceptance criteria**: How would you verify this feature works correctly? (Please provide a testable list)"

Respond with ONLY the comment text.
"#;

/// Prompt for generating a warm closure comment when an issue is declined (will-not-do).
///
/// The comment should:
/// - Express gratitude for the report/request
/// - Politely explain the decision not to pursue
/// - Be warm and respectful, NOT curt or dismissive
/// - Never just say "no" or "we won't do this"
pub const WARM_CLOSURE_PROMPT: &str = r#"You are writing a closure comment for a GitHub issue that will not be pursued.

Generate a warm, empathetic comment that:
1. Thanks the requestor for taking the time to report/submit this issue
2. Explains that after consideration, this will not be worked on at this time
3. Expresses regret that we cannot address this right now
4. Leaves the door open for future consideration

TONE: Warm, grateful, respectful. This person took time to file an issue - acknowledge that.
DO NOT USE: Curt phrases like "not a priority", "we won't implement this", or just "no"

Example good response:
"Thanks @username for the detailed feature request! I appreciate you taking the time to outline this use case.

After careful consideration, we're unable to prioritize this at the moment. The team has weighed this against other planned work and has decided not to move forward with this specific request.

We apologize for not being able to address this for you. If circumstances change in the future or you have other ideas, please don't hesitate to open a new issue.

Thanks again for contributing to the project!"

Issue details:
- Title: {issue_title}
- Author: @{issue_author}
- Type: {issue_type}

Respond with ONLY the comment text (no preamble or explanation).
"#;

/// Prompt for analyzing whether an issue requires epic-scale breakdown.
///
/// Epic-scale issues span multiple codebase areas or have sequential dependencies,
/// requiring breakdown into an epic bead + child beads. Standard work can be
/// handled as a single epic bead.
///
/// Epic-scale indicators:
/// - Multiple distinct codebase areas (CLI, API, DB, UI, config)
/// - Sequential dependencies ("and then...", step 1, step 2, etc.)
/// - Multiple logically distinct acceptance criteria groups
pub const EPIC_SCALE_ANALYSIS_PROMPT: &str = r#"You are analyzing a GitHub issue to determine whether it requires epic-scale breakdown.

An issue is epic-scale when it involves:
1. **Multiple codebase areas** - CLI, UI, API, database, configuration, auth, etc.
2. **Sequential dependencies** - work that must be done in phases, "and then..." patterns
3. **Multiple distinct units** - different logical concerns that could be worked on separately

Standard (single epic) issues:
- Describe work in one codebase area
- Can be described without "and then"
- One logical unit of acceptance criteria

Analyze the issue and respond with a JSON object:
{"is_epic_scale": true/false, "reasons": [...], "child_beads": [{"title": "...", "description": "..."}]}

If is_epic_scale is true, provide one child_beads entry per distinct unit of work.
Each child_beads title should indicate the codebase area it touches.
Do NOT provide more than 5 child beads - group if needed.

Issue content:
{issue_content}
"#;

/// Prompt for breaking down an epic-scale issue into child bead specifications.
///
/// Given an issue determined to be epic-scale, generate specific child bead
/// titles and descriptions following the two rules:
/// 1. **Single codebase part** - One entry per area (CLI, API, DB, UI, config)
/// 2. **No "...and then..." scope** - Each bead fits in one non-compound sentence
pub const EPIC_BREAKDOWN_PROMPT: &str = r#"You are breaking down an epic-scale GitHub issue into child bead specifications.

Each child bead must follow two rules:
1. **Single codebase part.** Touches at most one distinct area: CLI, UI, API, database, config, auth, etc.
2. **No "...and then..." scope.** Description fits in one non-compound sentence. If it naturally continues with "and then...", split into separate beads.

Generate child bead specifications as a JSON array:
[
  {"title": "Area: Short description of this unit", "description": "Concrete scope: what this bead does specifically", "priority": 2}
]

Maximum 5 child beads. Priority: 0=critical, 1=high, 2=medium, 3=low.
Group related work into a single bead rather than splitting finely.

Issue title: {issue_title}
Issue body: {issue_body}

Respond with ONLY the JSON array, no preamble.
"#;

/// Prompt for generating a standalone child bead description with all required sections.
///
/// A standalone bead is one that a naive but competent junior developer can implement
/// without consulting other beads or the epic description. Each bead MUST include:
/// 1. **WHAT TO DO** - Concrete files, packages, functions, or commands to create/modify
/// 2. **WHY** - User-visible behavior, constraint, or design rule this serves
/// 3. **HOW TO VERIFY** - Test, command, or observable result that proves work is done
/// 4. **EDGE CASES AND PITFALLS** - Non-obvious constraints a careful reader could miss
/// 5. **PROJECT-SPECIFIC TERMINOLOGY** - Project terms explained inline
pub const STANDALONE_BEAD_PROMPT: &str = r#"Generate a standalone child bead description for implementation.

A standalone bead provides ALL context needed for a naive but competent junior developer
to implement it WITHOUT consulting other beads or the parent epic.

REQUIRED SECTIONS (write all 5):
1. **WHAT TO DO**: Name concrete files, packages, functions, and commands to create or modify.
2. **WHY**: Explain the user-visible behavior, constraint, or design rule this serves.
3. **HOW TO VERIFY**: Specify the test, command, or observable result that proves work is done.
4. **EDGE CASES AND PITFALLS**: Non-obvious constraints a careful reader could miss.
5. **PROJECT-SPECIFIC TERMINOLOGY**: Define project-specific terms inline.

RULES:
- Single codebase part only (CLI OR API OR DB OR UI OR Config OR Auth)
- No "and then..." patterns - each bead scope should fit in one non-compound sentence
- Write for a naive junior dev who can write code and run tools but hasn't read the plan

FORMAT your response as a JSON object:
{
  "title": "Area: Brief description (e.g., 'API: User profile endpoint')",
  "description": "Full standalone description with all 5 sections formatted as markdown"
}

Bead scope: {bead_scope}
Codebase area: {codebase_area}
Acceptance criteria context: {ac_context}

Respond with ONLY the JSON object, no preamble or explanation.
"#;

/// Prompt for validating that a child bead description is standalone-ready.
///
/// This prompt helps an LLM validate that generated beads meet standalone criteria:
/// - All 5 required sections present
/// - Single codebase part (no CLI+API+DB+UI in one bead)
/// - No compound "and then..." patterns
pub const STANDALONE_VALIDATION_PROMPT: &str = r#"Validate whether a child bead description is standalone-ready.

A standalone-ready bead can be implemented by a naive but competent junior developer
WITHOUT consulting other beads, the parent epic, or out-of-band knowledge.

Check for these issues:

1. **MISSING SECTIONS**: Verify all 5 sections exist:
   - WHAT TO DO
   - WHY
   - HOW TO VERIFY
   - EDGE CASES AND PITFALLS (or EDGE CASES)
   - PROJECT-SPECIFIC TERMINOLOGY (or TERMINOLOGY)

2. **MULTIPLE CODEBASE AREAS**: Flag if bead touches multiple distinct areas:
   - CLI alone
   - API alone
   - Database alone
   - UI alone
   - Config alone
   - Auth alone
   (Exception: API + Database may be combined as they're closely related)

3. **COMPOUND PATTERNS**: Flag if bead has sequential work patterns:
   - "and then" patterns
   - "first... second..." patterns
   - "Step 1... Step 2..." numbered patterns
   - "after that" or "afterwards"
   - Sequential work that should be separate beads

Bead description to validate:
{bead_description}

Respond with a JSON object:
{
  "is_standalone_ready": true/false,
  "issues": ["list of issues found"],
  "suggestions": ["list of suggestions to fix issues"]
}
"#;

/// Prompt for splitting a compound bead into separate standalone beads.
pub const BEAD_SPLIT_PROMPT: &str = r#"Split a compound bead into separate standalone beads.

The following bead has compound scope (touches multiple areas or has sequential patterns).
Split it into 2-5 separate beads, each touching ONE distinct codebase area.

Original bead:
{original_bead}

RULES FOR SPLIT BEADS:
1. Each bead touches only ONE codebase area: CLI, API, DB, UI, Config, or Auth
2. No "and then..." patterns in any single bead
3. Each bead is standalone: includes all 5 sections
4. Maximum 5 beads - group closely related work
5. Preserve ordering if beads have dependencies

FORMAT as JSON array:
[
  {
    "title": "Area: Brief description",
    "description": "Standalone description (5 sections) for this unit",
    "has_dependency_on": null or "Area: Previous bead title"
  }
]

Respond with ONLY the JSON array.
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugFieldExtraction {
    /// Whether behavior observed is present
    pub behavior_observed: bool,
    /// Whether behavior expected is present
    pub behavior_expected: bool,
    /// Whether reproduction steps are present
    pub reproduction_steps: bool,
    /// Whether environment is present
    pub environment: bool,
}

/// Result from LLM field extraction for feature requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFieldExtraction {
    /// Whether use case is present
    pub use_case: bool,
    /// Whether proposed behavior is present
    pub proposed_behavior: bool,
    /// Whether acceptance criteria is present
    pub acceptance_criteria: bool,
}

/// Field name constants for bug fields.
pub mod bug_fields {
    /// Behavior observed field identifier
    pub const BEHAVIOR_OBSERVED: &str = "behavior_observed";
    /// Behavior expected field identifier
    pub const BEHAVIOR_EXPECTED: &str = "behavior_expected";
    /// Reproduction steps field identifier
    pub const REPRODUCTION_STEPS: &str = "reproduction_steps";
    /// Environment field identifier
    pub const ENVIRONMENT: &str = "environment";

    /// All bug field identifiers
    pub const ALL: &[&str] = &[
        BEHAVIOR_OBSERVED,
        BEHAVIOR_EXPECTED,
        REPRODUCTION_STEPS,
        ENVIRONMENT,
    ];
}

/// Field name constants for feature fields.
pub mod feature_fields {
    /// Use case field identifier
    pub const USE_CASE: &str = "use_case";
    /// Proposed behavior field identifier
    pub const PROPOSED_BEHAVIOR: &str = "proposed_behavior";
    /// Acceptance criteria field identifier
    pub const ACCEPTANCE_CRITERIA: &str = "acceptance_criteria";

    /// All feature field identifiers
    pub const ALL: &[&str] = &[USE_CASE, PROPOSED_BEHAVIOR, ACCEPTANCE_CRITERIA];
}

impl BugFieldExtraction {
    /// Get list of missing fields (fields that are false).
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.behavior_observed {
            missing.push(bug_fields::BEHAVIOR_OBSERVED);
        }
        if !self.behavior_expected {
            missing.push(bug_fields::BEHAVIOR_EXPECTED);
        }
        if !self.reproduction_steps {
            missing.push(bug_fields::REPRODUCTION_STEPS);
        }
        if !self.environment {
            missing.push(bug_fields::ENVIRONMENT);
        }
        missing
    }

    /// Check if all required fields are present.
    pub fn is_complete(&self) -> bool {
        self.behavior_observed
            && self.behavior_expected
            && self.reproduction_steps
            && self.environment
    }
}

impl FeatureFieldExtraction {
    /// Get list of missing fields (fields that are false).
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.use_case {
            missing.push(feature_fields::USE_CASE);
        }
        if !self.proposed_behavior {
            missing.push(feature_fields::PROPOSED_BEHAVIOR);
        }
        if !self.acceptance_criteria {
            missing.push(feature_fields::ACCEPTANCE_CRITERIA);
        }
        missing
    }

    /// Check if all required fields are present.
    pub fn is_complete(&self) -> bool {
        self.use_case && self.proposed_behavior && self.acceptance_criteria
    }
}

/// Format specific request message for missing bug fields.
///
/// This generates a user-friendly comment that requests ONLY the missing fields,
/// with specific explanations for each.
pub fn format_bug_field_request(missing: &[&str]) -> String {
    use std::fmt::Write;

    if missing.is_empty() {
        return String::new();
    }

    let mut msg = String::from(
        "To help us understand and reproduce this issue, could you provide the following?\n\n",
    );

    for field in missing {
        match *field {
            bug_fields::BEHAVIOR_OBSERVED => {
                msg.push_str("**Behavior observed**: What happened that seems wrong to you?\n");
            }
            bug_fields::BEHAVIOR_EXPECTED => {
                msg.push_str("**Behavior expected**: What did you expect to happen instead?\n");
            }
            bug_fields::REPRODUCTION_STEPS => {
                msg.push_str("- **Reproduction steps**: How can we reproduce this issue? (Or N/A if the bug cannot be reliably reproduced, with an explanation)\n");
            }
            bug_fields::ENVIRONMENT => {
                msg.push_str(
                    "- **Environment**: What OS, version, and relevant context are you using?\n",
                );
            }
            _ => {
                let _ = writeln!(
                    &mut msg,
                    "- **{field}**: Please provide this information.",
                    field = field
                );
            }
        }
    }

    msg
}

/// Format specific request message for missing feature fields.
///
/// This generates a user-friendly comment that requests ONLY the missing fields,
/// with specific explanations for each.
pub fn format_feature_field_request(missing: &[&str]) -> String {
    use std::fmt::Write;

    if missing.is_empty() {
        return String::new();
    }

    let mut msg = String::from(
        "To help us evaluate and implement this feature, could you provide the following?\n\n",
    );

    for field in missing {
        match *field {
            feature_fields::USE_CASE => {
                msg.push_str(
                    "**Use case**: Why do you need this feature? What problem are you solving?\n",
                );
            }
            feature_fields::PROPOSED_BEHAVIOR => {
                msg.push_str(
                    "**Proposed behavior**: How should this feature work once implemented?\n",
                );
            }
            feature_fields::ACCEPTANCE_CRITERIA => {
                msg.push_str("**Acceptance criteria**: How would you verify this feature works correctly? (Please provide a testable, enumerated list)\n");
            }
            _ => {
                let _ = writeln!(
                    &mut msg,
                    "- **{field}**: Please provide this information.",
                    field = field
                );
            }
        }
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bug_field_extraction_missing_fields() {
        let extraction = BugFieldExtraction {
            behavior_observed: true,
            behavior_expected: true,
            reproduction_steps: false,
            environment: false,
        };

        let missing = extraction.missing_fields();
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&bug_fields::REPRODUCTION_STEPS));
        assert!(missing.contains(&bug_fields::ENVIRONMENT));
    }

    #[test]
    fn test_bug_field_extraction_is_complete() {
        let complete = BugFieldExtraction {
            behavior_observed: true,
            behavior_expected: true,
            reproduction_steps: true,
            environment: true,
        };
        assert!(complete.is_complete());

        let incomplete = BugFieldExtraction {
            behavior_observed: true,
            behavior_expected: false,
            reproduction_steps: true,
            environment: true,
        };
        assert!(!incomplete.is_complete());
    }

    #[test]
    fn test_feature_field_extraction_missing_fields() {
        let extraction = FeatureFieldExtraction {
            use_case: true,
            proposed_behavior: true,
            acceptance_criteria: false,
        };

        let missing = extraction.missing_fields();
        assert_eq!(missing.len(), 1);
        assert!(missing.contains(&feature_fields::ACCEPTANCE_CRITERIA));
    }

    #[test]
    fn test_feature_field_extraction_is_complete() {
        let complete = FeatureFieldExtraction {
            use_case: true,
            proposed_behavior: true,
            acceptance_criteria: true,
        };
        assert!(complete.is_complete());
    }

    #[test]
    fn test_format_bug_field_request_environment_only() {
        let missing = vec![bug_fields::ENVIRONMENT];
        let request = format_bug_field_request(&missing);

        assert!(request.contains("Environment"));
        assert!(!request.contains("Reproduction steps"));
        assert!(!request.contains("Behavior observed"));
    }

    #[test]
    fn test_format_bug_field_request_steps_and_expected() {
        let missing = vec![
            bug_fields::REPRODUCTION_STEPS,
            bug_fields::BEHAVIOR_EXPECTED,
        ];
        let request = format_bug_field_request(&missing);

        assert!(request.contains("Reproduction steps"));
        assert!(request.contains("Behavior expected"));
        assert!(!request.contains("Environment"));
    }

    #[test]
    fn test_format_feature_field_request_acceptance_only() {
        let missing = vec![feature_fields::ACCEPTANCE_CRITERIA];
        let request = format_feature_field_request(&missing);

        assert!(request.contains("Acceptance criteria"));
        assert!(!request.contains("Use case"));
        assert!(!request.contains("Proposed behavior"));
    }

    #[test]
    fn test_format_bug_request_empty_missing() {
        let missing: Vec<&str> = vec![];
        let request = format_bug_field_request(&missing);
        assert!(request.is_empty());
    }

    #[test]
    fn test_no_generic_phrases() {
        // Verify that the format functions don't include generic phrases
        let bug_missing = vec![bug_fields::ENVIRONMENT];
        let bug_request = format_bug_field_request(&bug_missing);

        assert!(!bug_request.contains("more details"));
        assert!(!bug_request.contains("need more info"));
        assert!(!bug_request.contains("additional information"));

        let feature_missing = vec![feature_fields::USE_CASE];
        let feature_request = format_feature_field_request(&feature_missing);

        assert!(!feature_request.contains("more details"));
        assert!(!feature_request.contains("need more info"));
    }

    #[test]
    fn test_bug_prompt_includes_issue_content_placeholder() {
        assert!(BUG_FIELD_EXTRACTION_PROMPT.contains("{issue_content}"));
    }

    #[test]
    fn test_feature_prompt_includes_issue_content_placeholder() {
        assert!(FEATURE_FIELD_EXTRACTION_PROMPT.contains("{issue_content}"));
    }

    #[test]
    fn test_bug_missing_prompt_includes_missing_fields_placeholder() {
        assert!(BUG_MISSING_FIELDS_REQUEST_PROMPT.contains("{missing_fields}"));
        assert!(BUG_MISSING_FIELDS_REQUEST_PROMPT.contains("{bug_missing}"));
    }

    #[test]
    fn test_feature_missing_prompt_includes_missing_fields_placeholder() {
        assert!(FEATURE_MISSING_FIELDS_REQUEST_PROMPT.contains("{missing_fields}"));
        assert!(FEATURE_MISSING_FIELDS_REQUEST_PROMPT.contains("{feature_missing}"));
    }

    #[test]
    fn test_warm_closure_prompt_includes_issue_details() {
        assert!(WARM_CLOSURE_PROMPT.contains("{issue_title}"));
        assert!(WARM_CLOSURE_PROMPT.contains("{issue_author}"));
        assert!(WARM_CLOSURE_PROMPT.contains("{issue_type}"));
    }

    #[test]
    fn test_warm_closure_prompt_tone_guidance() {
        // The prompt should instruct for warm, grateful tone
        assert!(WARM_CLOSURE_PROMPT.contains("Thanks"));
        assert!(WARM_CLOSURE_PROMPT.contains("grateful") || WARM_CLOSURE_PROMPT.contains("Warm"));
        assert!(WARM_CLOSURE_PROMPT.contains("regret"));
        // Should instruct to NOT use curt phrases
        assert!(WARM_CLOSURE_PROMPT.contains("DO NOT USE"));
        assert!(WARM_CLOSURE_PROMPT.to_lowercase().contains("curt"));
    }

    #[test]
    fn test_warm_closure_prompt_includes_example() {
        // The prompt should include an example of a good response
        assert!(WARM_CLOSURE_PROMPT.contains("Example good response"));
        assert!(WARM_CLOSURE_PROMPT.contains("@username"));
    }

    // =============================================================================
    // Standalone Bead Prompt Tests (CRIT-5)
    // =============================================================================

    #[test]
    fn test_standalone_bead_prompt_has_required_sections() {
        assert!(STANDALONE_BEAD_PROMPT.contains("WHAT TO DO"));
        assert!(STANDALONE_BEAD_PROMPT.contains("WHY"));
        assert!(STANDALONE_BEAD_PROMPT.contains("HOW TO VERIFY"));
        assert!(STANDALONE_BEAD_PROMPT.contains("EDGE CASES"));
        assert!(STANDALONE_BEAD_PROMPT.contains("TERMINOLOGY"));
    }

    #[test]
    fn test_standalone_bead_prompt_includes_rules() {
        assert!(STANDALONE_BEAD_PROMPT.contains("Single codebase part"));
        assert!(STANDALONE_BEAD_PROMPT.contains("and then"));
    }

    #[test]
    fn test_standalone_bead_prompt_includes_format() {
        assert!(STANDALONE_BEAD_PROMPT.contains("JSON"));
        assert!(STANDALONE_BEAD_PROMPT.contains("title"));
        assert!(STANDALONE_BEAD_PROMPT.contains("description"));
    }

    #[test]
    fn test_standalone_bead_prompt_includes_placeholders() {
        assert!(STANDALONE_BEAD_PROMPT.contains("{bead_scope}"));
        assert!(STANDALONE_BEAD_PROMPT.contains("{codebase_area}"));
        assert!(STANDALONE_BEAD_PROMPT.contains("{ac_context}"));
    }

    #[test]
    fn test_standalone_validation_prompt_checks_sections() {
        assert!(STANDALONE_VALIDATION_PROMPT.contains("MISSING SECTIONS"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("WHAT TO DO"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("WHY"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("HOW TO VERIFY"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("EDGE CASES"));
    }

    #[test]
    fn test_standalone_validation_prompt_checks_multiple_areas() {
        assert!(STANDALONE_VALIDATION_PROMPT.contains("MULTIPLE CODEBASE AREAS"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("CLI"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("API"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("Database"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("UI"));
    }

    #[test]
    fn test_standalone_validation_prompt_checks_compound_patterns() {
        assert!(STANDALONE_VALIDATION_PROMPT.contains("COMPOUND PATTERNS"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("and then"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("Step"));
    }

    #[test]
    fn test_standalone_validation_prompt_includes_bead_desc_placeholder() {
        assert!(STANDALONE_VALIDATION_PROMPT.contains("{bead_description}"));
    }

    #[test]
    fn test_standalone_validation_prompt_returns_json() {
        assert!(STANDALONE_VALIDATION_PROMPT.contains("is_standalone_ready"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("issues"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("suggestions"));
    }

    #[test]
    fn test_bead_split_prompt_includes_rules() {
        assert!(BEAD_SPLIT_PROMPT.contains("compound scope"));
        assert!(BEAD_SPLIT_PROMPT.contains("ONE codebase area"));
        assert!(BEAD_SPLIT_PROMPT.contains("and then"));
        assert!(BEAD_SPLIT_PROMPT.contains("all 5 sections"));
    }

    #[test]
    fn test_bead_split_prompt_includes_original_bead_placeholder() {
        assert!(BEAD_SPLIT_PROMPT.contains("{original_bead}"));
    }

    #[test]
    fn test_bead_split_prompt_format() {
        assert!(BEAD_SPLIT_PROMPT.contains("JSON"));
        assert!(BEAD_SPLIT_PROMPT.contains("title"));
        assert!(BEAD_SPLIT_PROMPT.contains("description"));
        assert!(BEAD_SPLIT_PROMPT.contains("has_dependency_on"));
    }

    #[test]
    fn test_epic_breakdown_prompt_for_standalone_beads() {
        // The epic breakdown prompt should mention standalone rules
        assert!(EPIC_BREAKDOWN_PROMPT.contains("Single codebase part"));
        assert!(EPIC_BREAKDOWN_PROMPT.contains("and then"));
    }

    #[test]
    fn test_standalone_bead_prompt_checks_compound_patterns() {
        // The standalone validation prompt explicitly checks for sequential patterns
        assert!(STANDALONE_VALIDATION_PROMPT.contains("and then"));
        assert!(STANDALONE_VALIDATION_PROMPT.contains("Step"));
    }
}
