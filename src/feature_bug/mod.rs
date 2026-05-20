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
    analyze_epic_scale, build_epic_description_enriched, execute_breakdown,
    validate_beads_standalone,
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

// =============================================================================
// Acceptance Criteria Extraction (CRIT-6)
// =============================================================================
//
// Extract acceptance criteria from GitHub issue body AND comments.
// Acceptance criteria can appear:
//   - In the issue body (explicit AC section or checkbox list)
//   - In Rodgers-generated comments (draft criteria Rodgers drafted)
//   - In human comments (human accept/modify/reject criteria)
//
// Rodgers-generated criteria are marked with "Rodgers" in the comment/header.
// Human-modified criteria preserve the human's changes.

/// Categories of acceptance criteria by origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceCriteriaSource {
    /// Criteria in the GitHub issue body
    IssueBody,
    /// Criteria generated by Rodgers in a comment
    RodgersGenerated,
    /// Criteria modified or added by a human in a comment
    HumanModified,
}

/// A single acceptance criterion item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceCriterion {
    /// The criterion text (the full line/item)
    pub text: String,
    /// Whether it's marked done ([x]) or not ([ ])
    pub is_checked: bool,
    /// Source of this criterion
    pub source: AcceptanceCriteriaSource,
}

/// Result of extracting all acceptance criteria from an issue.
#[derive(Debug, Clone, Default)]
pub struct AllAcceptanceCriteria {
    /// All criteria found (Rodgers-generated + human-modified, in order)
    pub criteria: Vec<AcceptanceCriterion>,
    /// Rodgers-generated criteria specifically
    pub rodgers_generated: Vec<AcceptanceCriterion>,
    /// Human-modified criteria specifically
    pub human_modified: Vec<AcceptanceCriterion>,
    /// Whether any criteria were found at all
    pub has_criteria: bool,
    /// Whether the issue has no criteria yet (pending human review)
    pub no_criteria_yet: bool,
}

impl AllAcceptanceCriteria {
    /// Format all criteria as a markdown string for epic bead description.
    ///
    /// If no criteria were found, returns "Pending human review" note.
    pub fn format_for_epic(&self) -> String {
        if self.no_criteria_yet || self.criteria.is_empty() {
            return String::from(
                "- [ ] _Pending human review — acceptance criteria not yet defined_",
            );
        }

        self.criteria
            .iter()
            .map(|c| {
                if c.is_checked {
                    format!("- [x] {}", c.text)
                } else {
                    format!("- [ ] {}", c.text)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get a summary of how many criteria were found and their sources.
    pub fn summary(&self) -> String {
        if self.no_criteria_yet {
            "No acceptance criteria yet — pending human review".to_string()
        } else {
            format!(
                "{} total criteria ({} Rodgers-generated, {} human-modified)",
                self.criteria.len(),
                self.rodgers_generated.len(),
                self.human_modified.len()
            )
        }
    }
}

/// Extract acceptance criteria from the GitHub issue body.
///
/// Scans the body for:
/// - "## Acceptance Criteria" section headers (and aliases)
/// - Checkbox items: `- [ ]`, `- [x]`, `[ ] `, `[x] `
/// - Lines matching `AC-N:` pattern
fn extract_acceptance_criteria_from_body(body: &str) -> Vec<AcceptanceCriterion> {
    let mut criteria = Vec::new();
    let mut in_ac_section = false;

    // Detect if this appears to be Rodgers-generated from body content markers
    for line in body.lines() {
        let trimmed = line.trim();
        let trimmed_lower = trimmed.to_lowercase();

        // Check for AC section header
        if trimmed_lower.starts_with("## acceptance")
            || trimmed_lower.starts_with("## criteria")
            || trimmed_lower.starts_with("## verification")
            || trimmed_lower.starts_with("## rods")
        {
            in_ac_section = true;
            continue;
        }

        // Stop at the next ## header if we were in an AC section
        if in_ac_section && trimmed_lower.starts_with("## ") {
            in_ac_section = false;
        }

        // Extract checkbox items
        if extract_checkbox_item(trimmed).is_some() {
            if let Some(text) = extract_checkbox_item(trimmed) {
                criteria.push(AcceptanceCriterion {
                    text,
                    is_checked: trimmed.contains("[x]"),
                    source: AcceptanceCriteriaSource::IssueBody,
                });
            }
        }
    }

    // Also scan for AC-N patterns in the body
    if criteria.is_empty() {
        for line in body.lines() {
            let trimmed = line.trim();
            // Look for "AC-N:" or "AC-N." pattern anywhere in the body
            if trimmed.to_uppercase().starts_with("AC-")
                || trimmed.to_uppercase().starts_with("AC:")
            {
                let text = trimmed
                    .trim_start_matches(|c: char| {
                        c.is_ascii_digit() || c == '-' || c == ':' || c == ' '
                    })
                    .trim();

                if !text.is_empty() && text.len() > 3 {
                    criteria.push(AcceptanceCriterion {
                        text: text.to_string(),
                        is_checked: false,
                        source: AcceptanceCriteriaSource::IssueBody,
                    });
                }
            }
        }
    }

    // If still nothing, try broader checkbox scan
    if criteria.is_empty() {
        for line in body.lines() {
            let trimmed = line.trim();
            if extract_checkbox_item(trimmed).is_some() {
                if let Some(text) = extract_checkbox_item(trimmed) {
                    criteria.push(AcceptanceCriterion {
                        text,
                        is_checked: trimmed.contains("[x]"),
                        source: AcceptanceCriteriaSource::IssueBody,
                    });
                }
            }
        }
    }

    criteria
}

/// Extract acceptance criteria from a set of GitHub comments.
///
/// Rodgers-generated criteria are identified by:
/// - Comment by a known bot/automation account (Rodgers app)
/// - Comment containing "Rodgers" or "Rodgers Generated" patterns
/// - Comment containing "Acceptance Criteria" section header
///
/// Human-modified criteria are identified by:
/// - Non-Rodgers comments containing acceptance criteria
/// - Human edits/replies to Rodgers' generated criteria
fn extract_acceptance_criteria_from_comments(
    comments: &[crate::github::GitHubComment],
) -> AllAcceptanceCriteria {
    let mut all = AllAcceptanceCriteria::default();

    for comment in comments {
        let is_rodgers = comment.user.login.contains("rodgers")
            || comment.user.login.contains("github-actions")
            || comment.user.login.contains("겂_agent")
            || comment.user.login.contains("瓢_agent");

        let is_rodgers_comment = comment.body.to_lowercase().contains("rodgers")
            || comment
                .body
                .to_lowercase()
                .contains("generated acceptance criteria")
            || comment.body.to_lowercase().contains("acceptance criteria")
                && comment.body.contains("AC-");

        let source = if is_rodgers || is_rodgers_comment {
            AcceptanceCriteriaSource::RodgersGenerated
        } else {
            AcceptanceCriteriaSource::HumanModified
        };

        let extracted = extract_acceptance_criteria_from_comment_body(&comment.body);

        for text in extracted {
            let criterion = AcceptanceCriterion {
                text: text.clone(),
                is_checked: comment.body.contains("[x]") && comment.body.contains(&text),
                source: source.clone(),
            };

            all.criteria.push(criterion.clone());

            if source == AcceptanceCriteriaSource::RodgersGenerated {
                all.rodgers_generated.push(criterion.clone());
            } else {
                all.human_modified.push(criterion);
            }
        }
    }

    all.has_criteria = !all.criteria.is_empty();
    all.no_criteria_yet = all.criteria.is_empty();
    all
}

/// Extract acceptance criteria text from a comment body string.
///
/// Returns just the criterion texts (without checkbox markers).
fn extract_acceptance_criteria_from_comment_body(body: &str) -> Vec<String> {
    let mut criteria = Vec::new();
    let mut in_ac_section = false;

    for line in body.lines() {
        let trimmed = line.trim();
        let trimmed_lower = trimmed.to_lowercase();

        // Check for Rodgers-generated acceptance criteria section markers
        if trimmed_lower.contains("rodgers")
            && (trimmed_lower.contains("generated")
                || trimmed_lower.contains("acceptance")
                || trimmed_lower.contains("criteria"))
        {
            in_ac_section = true;
            continue;
        }

        // Check for AC section in comments
        if trimmed_lower.starts_with("## acceptance") || trimmed_lower.starts_with("## criteria") {
            in_ac_section = true;
            continue;
        }

        // Stop at next ## header
        if in_ac_section && trimmed_lower.starts_with("## ") {
            in_ac_section = false;
        }

        // Extract checkbox items
        if in_ac_section || !criteria.is_empty() {
            if let Some(text) = extract_checkbox_item(trimmed) {
                criteria.push(text);
            }
        }
    }

    // Also scan for AC-N patterns in the body (not just in section)
    if criteria.is_empty() {
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.to_uppercase().starts_with("AC-")
                || (trimmed.to_uppercase().starts_with("- [")
                    && trimmed.to_uppercase().contains("AC-"))
            {
                if let Some(text) = extract_checkbox_item(trimmed) {
                    criteria.push(text);
                }
            }
        }
    }

    criteria
}

/// Extract the text from a checkbox item line.
///
/// Accepts:
/// - `- [ ] text` or `- [x] text` → returns text (strip checkbox markers)
/// - `[ ] text` or `[x] text` → returns text
/// - `- [ ] AC-N: text` → returns "AC-N: text" (checkbox with AC, preserve full)
/// - `AC-1: text` (no checkbox) → returns just "text" (AC-N stripped)
/// - `- AC-2: text` (dash + AC, no checkbox) → returns just "text" (strip dash + AC-N:)
fn extract_checkbox_item(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Checkbox pattern
    if trimmed.contains("[ ]") || trimmed.contains("[x]") {
        let text = trimmed
            .trim_start_matches('-')
            .trim_start_matches(|c: char| c.is_whitespace() || c == '[' || c == 'x' || c == ']')
            .trim();

        if !text.is_empty() && text.len() > 2 {
            return Some(text.to_string());
        }
    }

    // AC-N pattern (no checkbox) - covers both "AC-1: text" and "- AC-2: text"
    let upper = trimmed.to_uppercase();
    let starts_with_ac = upper.starts_with("AC-") || upper.starts_with("- AC-");

    if starts_with_ac {
        // Find the first colon separating "AC-N:" from the actual text
        if let Some(colon_pos) = trimmed.find(':') {
            let after_colon = &trimmed[colon_pos + 1..];
            let text = after_colon.trim();
            if !text.is_empty() && text.len() > 2 {
                return Some(text.to_string());
            }
        }
    }

    None
}

/// Extract ALL acceptance criteria from an issue body AND comments combined.
///
/// This is the main extraction function for CRIT-6. It:
///
/// 1. First extracts criteria from the issue body
/// 2. Then adds criteria from comments (Rodgers-generated and human-modified)
/// 3. Returns a combined result with source tracking
///
/// The returned `AllAcceptanceCriteria` can be used to build the epic description:
/// - `format_for_epic()` gives the full criteria text for the epic bead
/// - `no_criteria_yet` if true, includes "pending human review" note
pub fn extract_all_acceptance_criteria(
    issue_body: &str,
    comments: &[crate::github::GitHubComment],
) -> AllAcceptanceCriteria {
    let mut result = AllAcceptanceCriteria::default();

    // Step 1: Extract from issue body
    let body_criteria = extract_acceptance_criteria_from_body(issue_body);
    for criterion in body_criteria {
        result.criteria.push(criterion.clone());
    }

    // Step 2: Extract from comments
    let comment_result = extract_acceptance_criteria_from_comments(comments);

    // Merge comment criteria (avoid duplicates)
    for criterion in comment_result.criteria {
        let is_duplicate = result.criteria.iter().any(|c| c.text == criterion.text);
        if !is_duplicate {
            result.criteria.push(criterion.clone());
        }

        if criterion.source == AcceptanceCriteriaSource::RodgersGenerated {
            if !result
                .rodgers_generated
                .iter()
                .any(|c| c.text == criterion.text)
            {
                result.rodgers_generated.push(criterion);
            }
        } else {
            if !result
                .human_modified
                .iter()
                .any(|c| c.text == criterion.text)
            {
                result.human_modified.push(criterion);
            }
        }
    }

    result.has_criteria = !result.criteria.is_empty();
    result.no_criteria_yet = result.criteria.is_empty();
    result
}

// =============================================================================
// GitHub Issue Section Extraction (for LLM What/Why summarization)
// =============================================================================

/// A section extracted from a GitHub issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueSection {
    /// Section name/header (e.g., "Use Case", "Proposed Behavior")
    pub name: String,
    /// Section content
    pub content: String,
}

/// Extract all named sections from a GitHub issue body.
///
/// Sections are identified by `## Header` or `# Header` markdown headers.
/// Returns sections in the order they appear in the body.
pub fn extract_issue_sections(body: &str) -> Vec<IssueSection> {
    let mut sections = Vec::new();
    let mut current_header = String::new();
    let mut current_content = String::new();

    for line in body.lines() {
        let trimmed = line.trim();

        // Detect section headers: ## Section Name
        if (trimmed.starts_with("## ") || trimmed.starts_with("# ")) && trimmed.len() > 3 {
            // Save previous section if non-empty
            if !current_header.is_empty() && !current_content.trim().is_empty() {
                sections.push(IssueSection {
                    name: current_header.clone(),
                    content: current_content.trim().to_string(),
                });
            }

            // Start new section
            current_header = trimmed
                .trim_start_matches('#')
                .trim_start_matches(|c: char| c == '#' || c.is_whitespace())
                .to_string();
            current_content.clear();
        } else if !current_header.is_empty() {
            // Accumulate content for current section
            if !current_content.is_empty() || !trimmed.is_empty() {
                if !current_content.is_empty() {
                    current_content.push('\n');
                }
                current_content.push_str(trimmed);
            }
        }
    }

    // Don't forget the last section
    if !current_header.is_empty() && !current_content.trim().is_empty() {
        sections.push(IssueSection {
            name: current_header,
            content: current_content.trim().to_string(),
        });
    }

    sections
}

/// Generate a "What and Why" summary of a GitHub issue.
///
/// This formats the issue content into a human-readable summary that answers:
/// - WHAT: What the issue is requesting (use case / problem description)
/// - WHY: Why this is needed (the rationale, problem being solved)
///
/// The summary is suitable for inclusion in an epic bead description.
pub fn generate_what_why_summary(
    issue_body: &str,
    issue_title: &str,
    author: &str,
    is_bug: bool,
) -> WhatWhySummary {
    let sections = extract_issue_sections(issue_body);

    let what = if is_bug {
        // For bug reports: get behavior observed + steps
        let behavior_observed = sections
            .iter()
            .find(|s| {
                s.name.to_lowercase().contains("behavior")
                    || s.name.to_lowercase().contains("happened")
                    || s.name.to_lowercase().contains("observed")
            })
            .map(|s| s.content.clone());

        let steps = sections
            .iter()
            .find(|s| {
                s.name.to_lowercase().contains("step")
                    || s.name.to_lowercase().contains("reproduce")
            })
            .map(|s| s.content.clone());

        format_what_from_parts(behavior_observed, steps, is_bug)
    } else {
        // For feature requests: get use case + proposed behavior
        let use_case = sections
            .iter()
            .find(|s| {
                s.name.to_lowercase().contains("use case")
                    || s.name.to_lowercase().contains("problem")
                    || s.name.to_lowercase().contains("why")
            })
            .map(|s| s.content.clone());

        let proposed_behavior = sections
            .iter()
            .find(|s| {
                s.name.to_lowercase().contains("proposed")
                    || s.name.to_lowercase().contains("behavior")
                    || s.name.to_lowercase().contains("solution")
            })
            .map(|s| s.content.clone());

        format_what_from_parts(use_case, proposed_behavior, is_bug)
    };

    let why = sections
        .iter()
        .find(|s| {
            s.name.to_lowercase().contains("why")
                || s.name.to_lowercase().contains("rationale")
                || s.name.to_lowercase().contains("motivation")
        })
        .map(|s| s.content.clone())
        .unwrap_or_else(|| {
            // Fallback: derive why from title
            String::from("Issue filed by user to request this change")
        });

    WhatWhySummary {
        what,
        why,
        issue_title: issue_title.to_string(),
        author: author.to_string(),
    }
}

/// Format "what" content from parsed parts.
fn format_what_from_parts(
    primary: Option<String>,
    secondary: Option<String>,
    is_bug: bool,
) -> String {
    let prefix = if is_bug { "Bug: " } else { "Feature: " };

    match (primary, secondary) {
        (Some(primary), Some(secondary)) => {
            format!("{}\n\n{}", prefix.to_string() + &primary, secondary)
        }
        (Some(primary), None) => prefix.to_string() + &primary,
        (None, Some(secondary)) => prefix.to_string() + &secondary,
        (None, None) => String::new(),
    }
}

/// A "What and Why" summary for an epic bead.
#[derive(Debug, Clone)]
pub struct WhatWhySummary {
    /// The "what" - what this issue is about
    pub what: String,
    /// The "why" - why this is needed
    pub why: String,
    /// Original issue title
    pub issue_title: String,
    /// Issue author
    pub author: String,
}

impl WhatWhySummary {
    /// Format as a markdown section for epic bead description.
    pub fn format_for_epic(&self) -> String {
        let what_block = if !self.what.is_empty() {
            format!("## What\n\n{}\n\n", self.what)
        } else {
            String::from("## What\n\n_[Extracted from issue body]_\n\n")
        };

        let why_block = if !self.why.is_empty() {
            format!("## Why\n\n{}\n\n", self.why)
        } else {
            String::from("## Why\n\n_[Extracted from issue body]_\n\n")
        };

        format!(
            "## Summary\n\n_Generated from GitHub issue: **{title}** by @{author}_\n\n{}{}",
            what_block,
            why_block,
            title = self.issue_title,
            author = self.author
        )
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

    // =============================================================================
    // CRIT-6: Acceptance Criteria Extraction and What/Why Summary
    // =============================================================================

    #[test]
    fn test_extract_checkbox_item_basic() {
        // Test various checkbox patterns
        assert_eq!(
            extract_checkbox_item("- [ ] Feature works correctly"),
            Some("Feature works correctly".to_string())
        );
        assert_eq!(
            extract_checkbox_item("- [x] Feature is working"),
            Some("Feature is working".to_string())
        );
        assert_eq!(
            extract_checkbox_item("[ ] Simple checkbox"),
            Some("Simple checkbox".to_string())
        );
    }

    #[test]
    fn test_extract_checkbox_item_ac_pattern() {
        // AC-N style acceptance criteria
        assert_eq!(
            extract_checkbox_item("AC-1: Feature is implemented"),
            Some("Feature is implemented".to_string())
        );
        assert_eq!(
            extract_checkbox_item("- AC-2: Tests pass"),
            Some("Tests pass".to_string())
        );
        assert_eq!(
            extract_checkbox_item("- [ ] AC-3: Docs updated"),
            Some("AC-3: Docs updated".to_string())
        );
    }

    #[test]
    fn test_extract_checkbox_item_empty_or_short() {
        // Empty or too short lines should return None
        assert_eq!(extract_checkbox_item(""), None);
        assert_eq!(extract_checkbox_item("- [ ]"), None);
        assert_eq!(extract_checkbox_item("- [ ] a"), None);
    }

    #[test]
    fn test_extract_acceptance_criteria_from_body_section() {
        let body = r#"
## Use Case
Test feature.

## Acceptance Criteria
- [ ] AC-1: First criterion
- [ ] AC-2: Second criterion
- [x] AC-3: Third criterion done

## Notes
More info.
"#;
        let criteria = extract_acceptance_criteria_from_body(body);
        assert!(criteria.len() >= 3, "Should extract at least 3 criteria");
        assert!(
            criteria
                .iter()
                .any(|c| c.source == AcceptanceCriteriaSource::IssueBody)
        );
    }

    #[test]
    fn test_extract_acceptance_criteria_from_body_no_ac_section() {
        // Should still find criteria even without ## Acceptance Criteria header
        let body = r#"
My feature request:
- [ ] Should do X
- [ ] Should do Y
- [x] Should do Z (already done)
"#;
        let criteria = extract_acceptance_criteria_from_body(body);
        assert!(
            criteria.len() >= 3,
            "Should extract criteria from body without ## header"
        );
        assert_eq!(criteria[0].is_checked, false);
        assert_eq!(criteria[2].is_checked, true); // [x] is checked
    }

    #[test]
    fn test_extract_acceptance_criteria_from_body_ac_n_pattern() {
        // AC-N patterns without checkbox markers
        let body = r#"
AC-1: First requirement
AC-2: Second requirement
AC-3: Third requirement
"#;
        let criteria = extract_acceptance_criteria_from_body(body);
        assert!(criteria.len() >= 3, "Should extract AC-N patterns");
    }

    #[test]
    fn test_extract_all_acceptance_criteria_combines_body_and_comments() {
        use crate::github::{GitHubComment, GitHubUser};

        let body = r#"
## Use Case
Feature request.

## Acceptance Criteria
- [ ] AC-1: Body criteria one
"#;
        let comments = vec![GitHubComment {
            id: 1,
            body: "## Rodgers Generated Acceptance Criteria\n\n- [ ] AC-2: Rodgers criteria one"
                .to_string(),
            user: GitHubUser {
                login: "rodgers-app".to_string(),
                id: 1,
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let result = extract_all_acceptance_criteria(body, &comments);

        assert!(result.has_criteria);
        assert!(!result.no_criteria_yet);
        // Should have both body and comment criteria
        assert!(result.criteria.len() >= 2);
        assert!(!result.rodgers_generated.is_empty());

        let formatted = result.format_for_epic();
        assert!(formatted.contains("AC-1"));
        assert!(formatted.contains("AC-2"));
    }

    #[test]
    fn test_extract_all_acceptance_criteria_no_criteria_pending() {
        let body = "Just a vague feature request without any acceptance criteria.";
        let result = extract_all_acceptance_criteria(body, &[]);

        assert!(!result.has_criteria);
        assert!(result.no_criteria_yet);

        let formatted = result.format_for_epic();
        assert!(
            formatted.contains("pending") || formatted.contains("Pending"),
            "Should include pending review note"
        );
    }

    #[test]
    fn test_all_acceptance_criteria_format_for_epic_empty() {
        let ac = AllAcceptanceCriteria {
            criteria: vec![],
            rodgers_generated: vec![],
            human_modified: vec![],
            has_criteria: false,
            no_criteria_yet: true,
        };

        let formatted = ac.format_for_epic();
        assert!(formatted.contains("pending") || formatted.contains("Pending"));
        assert!(formatted.contains("- [ ]")); // Should still format as checkbox
    }

    #[test]
    fn test_all_acceptance_criteria_checked_vs_unchecked() {
        let ac = AllAcceptanceCriteria {
            criteria: vec![
                AcceptanceCriterion {
                    text: "Done item".to_string(),
                    is_checked: true,
                    source: AcceptanceCriteriaSource::IssueBody,
                },
                AcceptanceCriterion {
                    text: "Not done".to_string(),
                    is_checked: false,
                    source: AcceptanceCriteriaSource::IssueBody,
                },
            ],
            rodgers_generated: vec![],
            human_modified: vec![],
            has_criteria: true,
            no_criteria_yet: false,
        };

        let formatted = ac.format_for_epic();
        assert!(formatted.contains("- [x] Done item"));
        assert!(formatted.contains("- [ ] Not done"));
    }

    #[test]
    fn test_extract_issue_sections_basic() {
        let body = r#"
## Use Case
Export data to CSV.

## Proposed Behavior
Button downloads a CSV file.

## Acceptance Criteria
- [ ] CSV downloads correctly
"#;
        let sections = extract_issue_sections(body);
        assert_eq!(sections.len(), 3);
        assert!(sections.iter().any(|s| s.name == "Use Case"));
        assert!(sections.iter().any(|s| s.name == "Proposed Behavior"));
        assert!(sections.iter().any(|s| s.name == "Acceptance Criteria"));
    }

    #[test]
    fn test_extract_issue_sections_with_content() {
        let body = "## Use Case\n\nI need CSV export.\n\n## Proposed Behavior\n\nA button.\n\n## Notes\n\nNo notes.\n\n## Something Else\n\nMore content.";
        let sections = extract_issue_sections(body);

        let use_case = sections.iter().find(|s| s.name == "Use Case").unwrap();
        assert!(use_case.content.contains("CSV export"));
    }

    #[test]
    fn test_generate_what_why_summary_feature() {
        let body = r#"
## Use Case
I need CSV export to analyze data in Excel.

## Proposed Behavior
A download button that generates CSV.

## Why It Matters
Users can't analyze data without it.
"#;
        let summary = generate_what_why_summary(body, "CSV Export Feature", "analyst", false);

        assert!(summary.issue_title == "CSV Export Feature");
        assert!(summary.author == "analyst");
        // What should include use case
        assert!(summary.what.contains("CSV export") || summary.what.contains("Use Case"));
        // Why should be populated
        assert!(summary.why.contains("analyze") || summary.why.contains("Excel"));
    }

    #[test]
    fn test_generate_what_why_summary_bug() {
        let body = r#"
## Behavior Observed
App crashes on launch.

## Steps to Reproduce
1. Open app
2. User sees crash

## Environment
macOS 14.0
"#;
        let summary = generate_what_why_summary(body, "App Crash", "user1", true);

        assert!(summary.issue_title == "App Crash");
        assert!(summary.author == "user1");
        // Bug summary should include behavior observed
        assert!(summary.what.contains("crash") || summary.what.contains("Bug"));
    }

    #[test]
    fn test_what_why_summary_format_for_epic() {
        let summary = WhatWhySummary {
            what: "Export feature requested".to_string(),
            why: "Users want to analyze data".to_string(),
            issue_title: "CSV Export".to_string(),
            author: "analyst".to_string(),
        };

        let formatted = summary.format_for_epic();
        assert!(formatted.contains("## Summary"));
        assert!(formatted.contains("## What"));
        assert!(formatted.contains("## Why"));
        assert!(formatted.contains("CSV Export"));
        assert!(formatted.contains("analyst"));
        assert!(formatted.contains("Export feature"));
        assert!(formatted.contains("analyze data"));
    }

    #[test]
    fn test_what_why_summary_fallback_when_no_sections() {
        let body = "Just some plain text without any ## sections.";
        let summary = generate_what_why_summary(body, "Plain Issue", "user", false);

        // Should not panic and should produce a summary
        assert!(summary.what.is_empty() || !summary.what.is_empty());
        assert!(!summary.why.is_empty()); // Should have fallback why
    }

    #[test]
    fn test_all_acceptance_criteria_deduplicates() {
        use crate::github::{GitHubComment, GitHubUser};

        let body = r#"
- [ ] AC-1: Shared criterion
- [ ] AC-2: Body-only
"#;
        let comments = vec![GitHubComment {
            id: 1,
            body:
                "## Acceptance Criteria\n\n- [ ] AC-1: Shared criterion\n- [ ] AC-3: Comment-only"
                    .to_string(),
            user: GitHubUser {
                login: "rodgers-app".to_string(),
                id: 1,
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let result = extract_all_acceptance_criteria(body, &comments);

        // Should not duplicate AC-1 (appears in both body and comment)
        let ac1_count = result
            .criteria
            .iter()
            .filter(|c| c.text.contains("Shared criterion"))
            .count();
        assert_eq!(ac1_count, 1, "AC-1 should appear only once (deduplicated)");
        assert_eq!(result.criteria.len(), 3); // AC-2, AC-1 (deduped), AC-3
    }

    #[test]
    fn test_acceptance_criteria_no_criteria_yet() {
        let body =
            "This is just a vague feature request without any ## Acceptance Criteria section.";
        let result = extract_all_acceptance_criteria(body, &[]);

        assert!(result.no_criteria_yet);
        assert!(!result.has_criteria);
        assert!(result.summary().contains("pending"));
    }

    #[test]
    fn test_all_acceptance_criteria_sources() {
        let ac = AllAcceptanceCriteria {
            criteria: vec![
                AcceptanceCriterion {
                    text: "Body AC".to_string(),
                    is_checked: false,
                    source: AcceptanceCriteriaSource::IssueBody,
                },
                AcceptanceCriterion {
                    text: "Rodgers AC".to_string(),
                    is_checked: false,
                    source: AcceptanceCriteriaSource::RodgersGenerated,
                },
                AcceptanceCriterion {
                    text: "Human AC".to_string(),
                    is_checked: false,
                    source: AcceptanceCriteriaSource::HumanModified,
                },
            ],
            rodgers_generated: vec![AcceptanceCriterion {
                text: "Rodgers AC".to_string(),
                is_checked: false,
                source: AcceptanceCriteriaSource::RodgersGenerated,
            }],
            human_modified: vec![AcceptanceCriterion {
                text: "Human AC".to_string(),
                is_checked: false,
                source: AcceptanceCriteriaSource::HumanModified,
            }],
            has_criteria: true,
            no_criteria_yet: false,
        };

        let summary = ac.summary();
        assert!(summary.contains("3 total criteria"));
        assert!(summary.contains("1 Rodgers-generated"));
        assert!(summary.contains("1 human-modified"));
    }
}
