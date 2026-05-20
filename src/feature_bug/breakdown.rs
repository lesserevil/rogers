//! Epic and child bead breakdown logic.
//!
//! This module implements the breakdown analysis defined in plans/feature-bug-plan.md §Bead Breakdown.
//!
//! When `ready-for-work` is detected, Rodgers analyzes the issue to determine
//! whether it requires epic-scale breakdown (multiple logical units of work)
//! or can be handled as a single epic.
//!
//! ## Epic-Scale Indicators
//!
//! An issue is epic-scale when it involves:
//! 1. **Multiple codebase areas** - CLI, UI, API, DB, config, etc.
//! 2. **Sequential dependencies** - work that must be done "and then..." before progressing
//! 3. **Multiple acceptance criteria groups** - logically distinct units of work
//!
//! ## Child Bead Scope
//!
//! Each child bead follows the two rules:
//! 1. **Single codebase part.** A bead should touch at most one distinct area.
//! 2. **No "...and then..." scope.** The description fits in one non-compound sentence.
//!
//! ## Child Bead Types
//!
//! Child beads inherit their type from the parent: if the issue is `bug`,
//! children are type=bug; if `feature`, children are type=feature.
//!
//! ## Standalone Bead Specification
//!
//! A standalone bead (AGENTS.md §Beads must stand alone) is one that a naive
//! but competent junior developer can implement without consulting other beads
//! or the epic description. Every standalone bead includes:
//!
//! - **WHAT TO DO**: Concrete files, packages, functions, or commands
//! - **WHY**: User-visible behavior, constraint, or design rule
//! - **HOW TO VERIFY**: Test, command, or observable result
//! - **EDGE CASES**: Non-obvious constraints a careful reader could miss
//! - **TERMINOLOGY**: Project-specific terms explained inline

use crate::beads::client::{BeadClient, BeadType, ChildBeadSpec, EpicScaleResult};

/// Analyze an issue to determine if it requires epic-scale breakdown.
///
/// Returns an `EpicScaleResult` indicating whether the work should be
/// broken into an epic + child beads or handled as a single epic.
pub fn analyze_epic_scale(
    issue_body: &str,
    github_issue_number: u64,
    is_bug: bool,
) -> EpicScaleResult {
    let indicators = detect_epic_scale_indicators(issue_body);

    if indicators.is_epic_scale {
        let child_beads = generate_child_beads(issue_body, github_issue_number, is_bug);
        EpicScaleResult {
            is_epic_scale: true,
            reasons: indicators.reasons,
            child_beads,
            recommendation: "Break into epic + child beads".to_string(),
        }
    } else {
        EpicScaleResult {
            is_epic_scale: false,
            reasons: indicators.reasons,
            child_beads: Vec::new(),
            recommendation: "Single epic is sufficient".to_string(),
        }
    }
}

/// Detected indicators from epic-scale analysis.
struct EpicScaleIndicators {
    /// Whether epic-scale breakdown is recommended
    is_epic_scale: bool,
    /// Detailed reasons for the decision
    reasons: Vec<String>,
}

/// Detect epic-scale indicators in issue content.
///
/// An issue is epic-scale if it has multiple signals across different
/// codebase areas or sequential work patterns.
fn detect_epic_scale_indicators(body: &str) -> EpicScaleIndicators {
    let body_lower = body.to_lowercase();

    let mut reasons = Vec::new();
    let mut score = 0usize;

    // Indicator 1: Multiple codebase area signals
    let area_signals = count_codebase_areas(&body_lower);
    if area_signals >= 2 {
        reasons.push(format!(
            "Spans multiple codebase areas ({area_signals} signs detected)"
        ));
        score += area_signals;
    } else if area_signals == 1 {
        reasons.push("Single codebase area detected".to_string());
    }

    // Indicator 2: Sequential/compound work signals
    let sequential_score = detect_sequential_work(&body_lower);
    if sequential_score >= 2 {
        reasons.push("Sequential/compound work pattern detected".to_string());
        score += sequential_score;
    }

    // Indicator 3: Multiple unrelated acceptance criteria
    let ac_groups = count_acceptance_criteria_groups(&body_lower);
    if ac_groups >= 2 {
        reasons.push(format!("{ac_groups} distinct acceptance criteria groups"));
        score += ac_groups;
    }

    // Indicator 4: Feature/enhancement covering multiple user interactions
    if body_lower.contains("and")
        && body_lower.contains("also")
        && (body_lower.contains("should") || body_lower.contains("must"))
    {
        reasons.push("Multiple user interactions described".to_string());
        score += 1;
    }

    // Indicator 5: Clear multi-step implementation hints
    let step_count = count_implementation_steps(&body_lower);
    if step_count >= 3 {
        reasons.push(format!("{step_count}+ implementation steps detected"));
        score += 2;
    }

    let is_epic_scale = score >= 3;

    EpicScaleIndicators {
        is_epic_scale,
        reasons,
    }
}

/// Count distinct codebase areas mentioned in the body.
fn count_codebase_areas(body: &str) -> usize {
    let mut count = 0usize;

    // CLI / command line interface
    if body.contains("cli")
        || body.contains("command-line")
        || body.contains("command line")
        || body.contains("commandline")
    {
        count += 1;
    }
    // API / REST / endpoints
    if body.contains("api") || body.contains("rest") || body.contains("endpoint") {
        count += 1;
    }
    // Database / storage / persistence
    if body.contains("database")
        || body.contains("db ")
        || body.contains("db,")
        || body.contains("db.")
        || body.contains("storage")
        || body.contains("persist")
    {
        count += 1;
    }
    // UI / dashboard / frontend / web interface
    if body.contains("ui")
        || body.contains("dashboard")
        || body.contains("frontend")
        || body.contains("interface")
        || body.contains("user interface")
        || body.contains("web ")
    {
        count += 1;
    }
    // Configuration / settings / config
    if body.contains("config") || body.contains("settings") {
        count += 1;
    }
    // Authentication / authorization
    if body.contains("auth")
        || body.contains("permission")
        || body.contains("role")
        || body.contains("login")
        || body.contains("credential")
        || body.contains("identity")
    {
        count += 1;
    }
    // Plugin / extension / integration
    if body.contains("plugin") || body.contains("extension") || body.contains("integration") {
        count += 1;
    }

    count
}

/// Detect sequential work patterns ("and then..." signals).
fn detect_sequential_work(body: &str) -> usize {
    let mut score = 0usize;

    // "and then" pattern
    if body.contains("and then") {
        score += 2;
    }
    // sequential terms
    if body.contains("first") && body.contains("second") {
        score += 1;
    }
    // "step N" patterns (step 1, step 2, etc.) - more explicit than just "1. 2."
    let has_explicit_step_1 =
        body.contains("step 1") || body.contains("step one:") || body.contains("step one -");
    let has_explicit_step_2 = body.contains("step 2")
        || body.contains("step two:")
        || body.contains("step two -")
        || body.contains("step 3")
        || body.contains("step three:");
    if has_explicit_step_1 && has_explicit_step_2 {
        score += 1;
    }
    // Numbered patterns like "1." followed by "2." but WITH context (not just list items)
    // This is more strict - requires the numbers to be part of a description, not bullet points
    if body.contains("1.") && body.contains("2.") {
        // Check if this looks like a step description rather than bullet points
        let lines: Vec<&str> = body.lines().collect();
        let has_step_context = lines.iter().any(|l| {
            let lower = l.to_lowercase();
            (lower.contains("step") || lower.contains("first"))
                && (lower.contains("1.") || lower.contains("step 1"))
        });
        if has_step_context {
            score += 1;
        }
    }
    // "then" keyword (sequential signal without explicit numbering)
    if body.contains("then") {
        score += 1;
    }
    // before/after dependencies
    if body.contains("before") && body.contains("after") {
        score += 1;
    }
    // depends on
    if body.contains("depends on") || body.contains("depends upon") {
        score += 1;
    }
    // multi-phase
    if body.contains("phase") && body.contains("then") {
        score += 1;
    }

    score
}

/// Count distinct acceptance criteria groups.
fn count_acceptance_criteria_groups(body: &str) -> usize {
    // This is a heuristic: acceptance criteria often appear as checkbox lists
    // or numbered AC-* statements. We look for logical grouping signals.
    let mut groups = 0usize;

    // Each ## section that contains acceptance criteria is a group
    let sections = [
        "authentication",
        "authorization",
        "api",
        "database",
        "ui",
        "cli",
        "configuration",
        "error handling",
        "performance",
        "security",
    ];

    for section in sections {
        if body.contains(section) {
            groups += 1;
        }
    }

    groups
}

/// Count implementation steps.
fn count_implementation_steps(body: &str) -> usize {
    let mut steps = 0usize;

    // Numbered steps: step 1, step 2, etc.
    for n in 1..=10 {
        if body.contains(&format!("step {n}"))
            || body.contains(&format!("{n}."))
            || body.contains(&format!("{n})"))
        {
            steps += 1;
        }
    }

    steps
}

/// Generate child beads from issue content.
///
/// Each child bead represents one logical unit of work following the
/// two rules: single codebase part, no "...and then..." descriptions.
fn generate_child_beads(body: &str, github_issue_number: u64, _is_bug: bool) -> Vec<ChildBeadSpec> {
    let body_lower = body.to_lowercase();

    // First, detect dedicated codebase areas
    let areas = detect_child_bead_areas(&body_lower);

    if !areas.is_empty() {
        // Map areas to child bead specs
        areas
            .iter()
            .map(
                |ChildArea(_area, title_prefix, desc_prefix)| ChildBeadSpec {
                    title: format!("{}: {}", title_prefix, github_issue_number),
                    description: format!(
                        "{} for Issue #{issue_number}. Detailed scope: TBD by implementer.",
                        desc_prefix,
                        issue_number = github_issue_number
                    ),
                    priority: 2,
                },
            )
            .collect()
    } else {
        // Fallback: analyze acceptance criteria for logical units
        let ac_units = extract_ac_logical_units(body);
        if ac_units.len() >= 2 {
            ac_units
        } else {
            // Default single child if nothing detected
            vec![ChildBeadSpec {
                title: format!("Work unit 1: Issue #{}", github_issue_number),
                description: format!(
                    "Implementation scope for Issue #{issue_number}. First logical unit of work.",
                    issue_number = github_issue_number
                ),
                priority: 2,
            }]
        }
    }
}

/// Detected child bead areas.
struct ChildArea(&'static str, &'static str, &'static str);

/// Detect child bead areas from issue content.
fn detect_child_bead_areas(body: &str) -> Vec<ChildArea> {
    let mut areas = Vec::new();

    // API / Backend
    if body.contains("api") || body.contains("endpoint") || body.contains("rest") {
        areas.push(ChildArea(
            "api",
            "API / backend implementation",
            "Implement API endpoints and backend logic",
        ));
    }
    // Database / storage
    if body.contains("database") || body.contains("storage") || body.contains("db ") {
        areas.push(ChildArea(
            "database",
            "Database / storage layer",
            "Implement database schema and queries",
        ));
    }
    // Frontend / UI
    if body.contains("ui")
        || body.contains("frontend")
        || body.contains("interface")
        || body.contains("dashboard")
    {
        areas.push(ChildArea(
            "ui",
            "UI / frontend implementation",
            "Implement user interface components",
        ));
    }
    // CLI
    if body.contains("cli") || body.contains("command-line") {
        areas.push(ChildArea(
            "cli",
            "CLI implementation",
            "Implement command-line interface",
        ));
    }
    // Configuration
    if body.contains("config") || body.contains("settings") {
        areas.push(ChildArea(
            "config",
            "Configuration setup",
            "Implement configuration management",
        ));
    }
    // Authentication
    if body.contains("auth") || body.contains("login") || body.contains("permission") {
        areas.push(ChildArea(
            "auth",
            "Authentication / authorization",
            "Implement authentication and authorization",
        ));
    }

    // Limit to 5 children max
    if areas.len() > 5 {
        areas.truncate(5);
    }

    areas
}

/// Extract logical units from acceptance criteria.
fn extract_ac_logical_units(body: &str) -> Vec<ChildBeadSpec> {
    // Look for checkbox items or AC-* items
    let mut units = Vec::new();
    let mut unit_num = 1usize;

    // Parse checkbox-style criteria
    for line in body.lines() {
        let trimmed = line.trim();
        // Check for [ ], [x], or - [ ] patterns
        if trimmed.contains("[ ]")
            || trimmed.starts_with("- [")
            || trimmed.starts_with("AC-")
            || trimmed.starts_with("ac-")
        {
            let description = trimmed
                .trim_start_matches(|c: char| c == '-' || c == '[' || c == ']')
                .trim();

            if !description.is_empty()
                && description.len() > 3
                && !description.to_lowercase().contains("test")
            {
                units.push(ChildBeadSpec {
                    title: format!("Work unit {}: {}", unit_num, description),
                    description: description.to_string(),
                    priority: 2,
                });
                unit_num += 1;
            }
        }

        if unit_num > 5 {
            break;
        }
    }

    units
}

/// Execute the full breakdown pipeline for a ready-for-work issue.
///
/// This function:
/// 1. Analyzes the issue for epic-scale indicators
/// 2. Files the epic bead (deferred)
/// 3. Files child beads (if epic-scale, all deferred)
/// 4. Returns the breakdown result with comment to post
pub fn execute_breakdown(
    issue_body: &str,
    issue_title: &str,
    github_issue_number: u64,
    github_issue_url: &str,
    is_bug: bool,
) -> BreakdownResult {
    // Step 1: Analyze for epic scale
    let scale_result = analyze_epic_scale(issue_body, github_issue_number, is_bug);

    // Step 2: Build the epic bead request
    let client = BeadClient::new();
    let bead_type = if is_bug {
        BeadType::Bug
    } else {
        BeadType::Feature
    };

    // Extract acceptance criteria from body for epic description
    let acceptance_criteria = extract_acceptance_criteria_text(issue_body);

    let epic_request = client.build_epic_request(
        github_issue_number,
        issue_title,
        issue_body,
        github_issue_url,
        &acceptance_criteria,
        scale_result.is_epic_scale,
        bead_type,
        2, // priority: medium
    );

    // Step 3: Generate child bead requests if epic-scale
    let mut child_requests = Vec::new();
    if scale_result.is_epic_scale {
        for spec in &scale_result.child_beads {
            let child_request = client.build_child_request(
                spec,
                "", // parent_id filled in after epic is filed
                github_issue_number,
                bead_type,
            );
            child_requests.push(child_request);
        }
    }

    // Step 4: Build breakdown comment
    let breakdown_comment = if scale_result.is_epic_scale {
        // Comment with child bead placeholders (IDs filled in after filing)
        client.build_breakdown_comment("TBD-epic", &[][..], true)
    } else {
        client.build_breakdown_comment("TBD-epic", &[][..], false)
    };

    BreakdownResult {
        epic_request,
        child_requests,
        breakdown_comment,
        is_epic_scale: scale_result.is_epic_scale,
        reasons: scale_result.reasons,
    }
}

/// Extract acceptance criteria text from issue body.
fn extract_acceptance_criteria_text(body: &str) -> String {
    let mut in_ac_section = false;
    let mut ac_lines = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim().to_lowercase();

        if trimmed.starts_with("## acceptance")
            || trimmed.starts_with("## criteria")
            || trimmed.starts_with("## verification")
        {
            in_ac_section = true;
            continue;
        }

        if in_ac_section {
            // Stop at the next ## header
            if trimmed.starts_with("## ") {
                break;
            }

            let line_trimmed = line.trim();
            if !line_trimmed.is_empty() {
                ac_lines.push(line.to_string());
            }
        }
    }

    if ac_lines.is_empty() {
        // Fallback: look for checkbox patterns anywhere
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.contains("[ ]") || trimmed.starts_with("- [") {
                ac_lines.push(line.to_string());
            }
        }
    }

    if ac_lines.is_empty() {
        String::from("- [ ] Work is complete and verified")
    } else {
        ac_lines.join("\n")
    }
}

/// Result of a breakdown operation.
#[derive(Debug, Clone)]
pub struct BreakdownResult {
    /// Request to file the epic bead
    pub epic_request: crate::beads::client::FileBeadRequest,
    /// Requests to file child beads (if epic-scale)
    pub child_requests: Vec<crate::beads::client::FileBeadRequest>,
    /// Breakdown comment to post on GitHub issue
    pub breakdown_comment: String,
    /// Whether this was epic-scale
    pub is_epic_scale: bool,
    /// Reasons for the epic-scale decision
    pub reasons: Vec<String>,
}

// =============================================================================
// Standalone Bead Specification
// =============================================================================

/// A standalone bead description following AGENTS.md §Beads must stand alone.
///
/// A standalone bead provides all context needed for a naive but competent
/// junior developer to implement it without consulting other beads or the
/// parent epic.
#[derive(Debug, Clone, Default)]
pub struct StandaloneBead {
    /// WHAT TO DO: Concrete files, packages, functions, or commands to create/modify
    pub what_to_do: String,
    /// WHY: User-visible behavior, constraint, or design rule this serves
    pub why: String,
    /// HOW TO VERIFY: Test, command, or observable result that proves the work is done
    pub how_to_verify: String,
    /// EDGE CASES AND PITFALLS: Non-obvious constraints a careful reader could miss
    pub edge_cases: String,
    /// PROJECT-SPECIFIC TERMINOLOGY: Terms that only make sense in context
    pub terminology: String,
}

impl StandaloneBead {
    /// Create a new standalone bead with all sections empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a standalone bead with all required sections.
    pub fn with_sections(
        what_to_do: &str,
        why: &str,
        how_to_verify: &str,
        edge_cases: &str,
        terminology: &str,
    ) -> Self {
        Self {
            what_to_do: what_to_do.to_string(),
            why: why.to_string(),
            how_to_verify: how_to_verify.to_string(),
            edge_cases: edge_cases.to_string(),
            terminology: terminology.to_string(),
        }
    }

    /// Check if all 5 required sections are present and non-empty.
    pub fn has_all_sections(&self) -> bool {
        !self.what_to_do.trim().is_empty()
            && !self.why.trim().is_empty()
            && !self.how_to_verify.trim().is_empty()
            && !self.edge_cases.trim().is_empty()
            && !self.terminology.trim().is_empty()
    }

    /// Check if the bead describes work in a single codebase part.
    ///
    /// A compound bead would touch multiple areas (CLI + API + DB) which
    /// should be separate beads. API with integrated database access is considered single.
    /// Uses word boundary detection to avoid false positives (e.g., "clients" contains "cli").
    pub fn is_single_codebase_part(&self) -> bool {
        let content_lower = format!(
            "{} {} {} {} {}",
            self.what_to_do, self.why, self.how_to_verify, self.edge_cases, self.terminology
        )
        .to_lowercase();

        // Count the distinct areas mentioned using word boundaries
        let mut areas = 0usize;

        // CLI area - require action word boundary (not "clients", "click", "cache")
        let has_cli = content_lower.contains(" cli ")
            || content_lower.contains(" cli,")
            || content_lower.contains(" cli.")
            || content_lower.contains(" cli\n")
            || content_lower.contains(" cli/")
            || content_lower.contains(" cli-")
            || content_lower.contains("-cli ")
            || content_lower.contains("\ncli ");
        if has_cli {
            areas += 1;
        }

        // API area - matches "api", "rest", or "endpoint"
        let has_api = content_lower.contains("api")
            || content_lower.contains(" rest ")
            || content_lower.contains("rest,")
            || content_lower.contains("rest ")
            || content_lower.contains(" endpoint")
            || content_lower.contains("endpoint:")
            || content_lower.contains("endpoint ");
        if has_api {
            areas += 1;
        }

        // Database - only count if NO API is present (API includes DB access normally)
        let has_db = content_lower.contains("database")
            || content_lower.contains(" db ")
            || content_lower.contains("db,")
            || content_lower.contains("storage")
            || content_lower.contains("persist");
        if has_db && !has_api {
            areas += 1;
        }

        // UI area
        let has_ui = content_lower.contains("ui ")
            || content_lower.contains("ui,")
            || content_lower.contains(" dashboard")
            || content_lower.contains("dashboard ")
            || content_lower.contains("frontend")
            || content_lower.contains("interface ");
        if has_ui {
            areas += 1;
        }

        // Config area
        let has_config = content_lower.contains("config") || content_lower.contains("settings");
        if has_config {
            areas += 1;
        }

        // Auth area
        let has_auth = content_lower.contains("auth")
            || content_lower.contains("permission")
            || content_lower.contains("login");
        if has_auth {
            areas += 1;
        }

        // Allow at most 1 area (API with integrated DB counts as 1)
        areas <= 1
    }

    /// Check if there's a compound "...and then..." pattern.
    ///
    /// Compound beads should be split into separate beads.
    pub fn has_compound_pattern(&self) -> bool {
        let content_lower = format!(
            "{} {} {} {} {}",
            self.what_to_do, self.why, self.how_to_verify, self.edge_cases, self.terminology
        )
        .to_lowercase();

        // Direct "and then" pattern
        if content_lower.contains("and then") {
            return true;
        }

        // Sequential indicators without explicit "and then"
        let has_first = content_lower.contains("first ");
        let has_second = content_lower.contains("second ");
        let has_step_1 = content_lower.contains("step 1:")
            || content_lower.contains("step one:")
            || content_lower.contains("step one -")
            || content_lower.contains("\nstep 1 ");
        let has_step_2 = content_lower.contains("step 2:")
            || content_lower.contains("step two:")
            || content_lower.contains("step two -")
            || content_lower.contains("\nstep 2 ");

        if (has_first && has_second) || (has_step_1 && has_step_2) {
            return true;
        }

        // Multiple "and then" patterns like "also" then "then" pattern
        if content_lower.contains("also")
            && content_lower.contains("then")
            && (content_lower.contains("and") || content_lower.contains("after"))
        {
            return true;
        }

        // "after that" pattern
        if content_lower.contains("after that") || content_lower.contains("afterwards") {
            return true;
        }

        false
    }

    /// Check if the bead is standalone-ready.
    ///
    /// A bead is standalone-ready if:
    /// - All 5 sections are present
    /// - It deals with a single codebase part
    /// - It has no compound patterns
    pub fn is_standalone_ready(&self) -> StandaloneValidation {
        let sections_present = self.has_all_sections();
        let single_part = self.is_single_codebase_part();
        let no_compound = !self.has_compound_pattern();

        let issues = {
            let mut v = Vec::new();
            if !sections_present {
                v.push(StandaloneIssue::MissingSections);
            }
            if !single_part {
                v.push(StandaloneIssue::MultipleCodebaseParts);
            }
            if !no_compound {
                v.push(StandaloneIssue::CompoundPattern);
            }
            v
        };

        StandaloneValidation {
            is_valid: issues.is_empty(),
            issues,
        }
    }

    /// Format the bead as a markdown string for use in bead descriptions.
    pub fn to_markdown(&self) -> String {
        format!(
            r#"WHAT TO DO
{}

WHY
{}

HOW TO VERIFY
{}

EDGE CASES AND PITFALLS
{}

PROJECT-SPECIFIC TERMINOLOGY
{}"#,
            self.what_to_do.trim(),
            self.why.trim(),
            self.how_to_verify.trim(),
            self.edge_cases.trim(),
            self.terminology.trim()
        )
    }
}

/// Validation result for standalone bead checks.
#[derive(Debug, Clone, Default)]
pub struct StandaloneValidation {
    /// Whether the bead meets all standalone criteria
    pub is_valid: bool,
    /// Issues found during validation
    pub issues: Vec<StandaloneIssue>,
}

impl StandaloneValidation {
    /// Create a validation result from issues.
    pub fn from_issues(issues: Vec<StandaloneIssue>) -> Self {
        Self {
            is_valid: issues.is_empty(),
            issues,
        }
    }

    /// Check if validation passed.
    pub fn passed(&self) -> bool {
        self.is_valid
    }

    /// Get human-readable descriptions of all issues.
    pub fn descriptions(&self) -> Vec<String> {
        self.issues.iter().map(|i| i.description()).collect()
    }
}

/// Issues that prevent a bead from being standalone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneIssue {
    /// One or more of the 5 required sections is missing or empty
    MissingSections,
    /// Bead touches multiple distinct codebase areas
    MultipleCodebaseParts,
    /// Bead has compound "...and then..." pattern
    CompoundPattern,
}

impl StandaloneIssue {
    /// Get a human-readable description of the issue.
    pub fn description(&self) -> String {
        match self {
            Self::MissingSections => {
                "Missing one or more required sections (WHAT TO DO, WHY, HOW TO VERIFY, EDGE CASES, TERMINOLOGY)".to_string()
            }
            Self::MultipleCodebaseParts => {
                "Bead touches multiple distinct codebase parts (CLI, API, DB, etc.). Split into separate beads.".to_string()
            }
            Self::CompoundPattern => {
                "Bead has compound 'and then...' pattern. Split into separate sequential beads.".to_string()
            }
        }
    }
}

/// Generate a standalone bead from work scope information.
///
/// This function creates a standalone bead description with all required
/// sections based on the codebase area and the acceptance criteria context.
pub fn generate_standalone_bead(
    codebase_area: &str,
    scope_description: &str,
    ac_context: &[&str],
) -> StandaloneBead {
    let area_lower = codebase_area.to_lowercase();
    let is_api = area_lower.contains("api") || area_lower.contains("endpoint");
    let is_db = area_lower.contains("database")
        || area_lower.contains("storage")
        || area_lower.contains("db");
    let is_ui = area_lower.contains("ui")
        || area_lower.contains("frontend")
        || area_lower.contains("dashboard");
    let is_cli = area_lower.contains("cli") || area_lower.contains("command");
    let is_auth = area_lower.contains("auth")
        || area_lower.contains("permission")
        || area_lower.contains("login");
    let is_config = area_lower.contains("config") || area_lower.contains("settings");

    // Generate WHAT TO DO based on area
    let what_to_do = match () {
        _ if is_api => format!(
            "Implement API endpoints for: {}\n\nFiles to modify:\n- src/api/ (new handlers)\n- src/models/ (request/response types)\n- src/routes.rs (add routes)\n\nFunctions to create/modify: handler functions, request validators",
            scope_description
        ),
        _ if is_db => format!(
            "Implement database layer for: {}\n\nFiles to modify:\n- src/db/ (schema, migrations)\n- src/models/ (entity definitions)\n- src/queries.rs (composite queries)\n\nCreate schema, migrations, and repository functions",
            scope_description
        ),
        _ if is_ui => format!(
            "Implement UI components for: {}\n\nFiles to modify:\n- src/ui/ (new components)\n- src/components/ (shared components)\n- src/styles/ (CSS/styling)\n\nBuild React/HTML components, wire to state management",
            scope_description
        ),
        _ if is_cli => format!(
            "Implement CLI commands for: {}\n\nFiles to modify:\n- src/cli/ (command modules)\n- src/commands.rs (command registration)\n\nCreate CLI argument parser, command handlers",
            scope_description
        ),
        _ if is_auth => format!(
            "Implement authentication/authorization for: {}\n\nFiles to modify:\n- src/auth/ (auth logic)\n- src/middleware/ (auth middleware)\n- src/models/ (user/role types)\n\nImplement auth checks, middleware, user management",
            scope_description
        ),
        _ if is_config => format!(
            "Implement configuration management for: {}\n\nFiles to modify:\n- src/config/ (config types)\n- config.example.yaml (schema)\n- src/validation.rs (config validation)\n\nDefine config schema, env var handling",
            scope_description
        ),
        _ => format!(
            "Implement: {}\n\nScope: {}",
            codebase_area, scope_description
        ),
    };

    // Generate WHY from acceptance criteria context
    let why = if !ac_context.is_empty() {
        format!("Required by acceptance criteria: {}", ac_context.join("; "))
    } else {
        format!(
            "Required for complete implementation of: {}",
            scope_description
        )
    };

    // Generate HOW TO VERIFY
    let how_to_verify = match () {
        _ if is_api => String::from(
            "1. Run existing tests: `cargo test`\n2. Manual test: curl the endpoint with valid/invalid inputs\n3. Verify response matches schema: `cargo test -- --test-threads=1 api_*`\n4. Check logs for errors",
        ),
        _ if is_db => String::from(
            "1. Run migrations: `cargo run migrate`\n2. Run tests: `cargo test`\n3. Verify data persists across restarts\n4. Check migration logs",
        ),
        _ if is_ui => String::from(
            "1. Start dev server: `cargo run`\n2. Navigate to affected UI area\n3. Verify component renders correctly\n4. Test user interactions",
        ),
        _ if is_cli => String::from(
            "1. Build: `cargo build --release`\n2. Run help: `./target/release/rogers --help`\n3. Test command: `./target/release/rogers <command> --help`\n4. Verify output format",
        ),
        _ if is_auth => String::from(
            "1. Test unauthenticated access is rejected\n2. Test authenticated access succeeds\n3. Test unauthorized actions fail\n4. Verify session/token handling",
        ),
        _ if is_config => String::from(
            "1. Run with new config: `./rogers --config config.yaml`\n2. Verify config loads without errors\n3. Test invalid config is rejected with clear error\n4. Verify env var overrides work",
        ),
        _ => String::from("Run tests and verify behavior matches acceptance criteria"),
    };

    // Generate EDGE CASES based on area
    let edge_cases = match () {
        _ if is_api || is_db => String::from(
            "- Handle concurrent requests properly (mutex/locking where needed)\n- Return appropriate HTTP codes for error cases\n- Validate all inputs before processing\n- Handle None/empty values gracefully\n- Connection pooling for database",
        ),
        _ if is_ui => String::from(
            "- Handle loading states briefly (avoid flash)\n- Handle error states with clear messages\n- Responsive layout on different screen sizes\n- Keyboard navigation accessibility\n- Focus management for modals/dialogs",
        ),
        _ if is_cli => String::from(
            "- Handle invalid argument combinations\n- Show helpful error messages\n- Handle piped input and large inputs\n- Progress indicators for long operations\n- Proper exit codes (0 success, non-zero failure)",
        ),
        _ if is_auth => String::from(
            "- Session timeout handling\n- Cross-site request forgery (CSRF) prevention\n- Rate limiting on auth endpoints\n- Token refresh logic\n- Secure credential storage (no plaintext)",
        ),
        _ if is_config => String::from(
            "- Unknown config keys should warn, not fail\n- Validate types before parsing\n- Handle missing optional fields\n- Config file path resolution\n- Environment variable precedence",
        ),
        _ => String::from(
            "- Handle error cases gracefully\n- Include appropriate logging\n- Clean up resources on failure",
        ),
    };

    // Generate TERMINOLOGY
    let terminology = String::from(
        "**Bead**: A unit of work tracked as an issue. Child beads are sub-tasks of an epic.\n\
        **Standalone bead**: A bead with complete context so a junior dev can implement it alone.\n\
        **Acceptance criteria**: Testable conditions that verify the work is complete.",
    );

    StandaloneBead::with_sections(&what_to_do, &why, &how_to_verify, &edge_cases, &terminology)
}

/// Validate that a list of child beads would be standalone.
///
/// Returns validation results for each bead plus a summary indicating
/// whether all beads meet standalone criteria.
pub fn validate_beads_standalone(beads: &[StandaloneBead]) -> BeadValidationResult {
    let mut individual_results = Vec::new();

    for (idx, bead) in beads.iter().enumerate() {
        let validation = bead.is_standalone_ready();
        individual_results.push((idx, validation));
    }

    let all_valid = individual_results.iter().all(|(_, v)| v.passed());

    BeadValidationResult {
        all_standalone: all_valid,
        individual_results,
    }
}

/// Result of validating multiple beads for standalone criteria.
#[derive(Debug, Clone, Default)]
pub struct BeadValidationResult {
    /// Whether all beads are standalone-ready
    pub all_standalone: bool,
    /// Individual validation results per bead
    pub individual_results: Vec<(usize, StandaloneValidation)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epic_scale_single_area_detected() {
        // Single area - not epic-scale (score 1 < 3)
        let body = r#"
## Use Case
I want to export my data to CSV.

## Proposed Behavior
A button that exports data.

## Acceptance Criteria
- [ ] Export button appears
"#;
        let result = analyze_epic_scale(body, 1, false);
        assert!(!result.is_epic_scale);
    }

    #[test]
    fn test_epic_scale_with_four_distinct_areas() {
        // Four distinct areas: auth, api, ui, database
        // Should trigger is_epic_scale and add "Multiple" reason
        let body = "auth login credential identity api rest endpoint ui dashboard frontend interface config settings database storage plugin extension integration";
        let result = analyze_epic_scale(body, 1, false);
        assert!(
            result.is_epic_scale,
            "Body with 4+ areas should be epic scale"
        );
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r.to_lowercase().contains("multiple")),
            "Reasons should mention multiple: {:?}",
            result.reasons
        );
    }

    #[test]
    fn test_epic_scale_sequential_pattern() {
        // Sequential pattern only - borderline
        let body = r#"
## Use Case
Migration work.

Step 1: Create new schema
Step 2: Migrate data
Step 3: Update API
Step 4: Update frontend

Acceptance Criteria
- [ ] Migration completes
- [ ] App works
"#;
        let result = analyze_epic_scale(body, 1, false);
        assert!(result.is_epic_scale);
    }

    #[test]
    fn test_epic_scale_complex_multi_area() {
        // Complex work spanning multiple areas
        let body = r#"
## Use Case
Full-stack feature with multiple moving parts.

## Areas
- CLI for admin commands
- API for client access
- Database for storage
- UI dashboard

## Acceptance Criteria
- [ ] Admin can manage via CLI
- [ ] Client access via REST
- [ ] Data persisted
- [ ] Dashboard shows status
"#;
        let result = analyze_epic_scale(body, 1, false);
        assert!(result.is_epic_scale);
        assert!(result.child_beads.len() >= 2);
    }

    #[test]
    fn test_execute_breakdown_single_epic() {
        let body = r#"
## Use Case
Simple CSV export.

## Proposed Behavior
Export button downloads CSV.

## Acceptance Criteria
- [ ] Button present
"#;
        let result = execute_breakdown(
            body,
            "Export to CSV",
            42,
            "https://github.com/org/repo/issues/42",
            false,
        );

        assert!(!result.is_epic_scale);
        assert!(result.child_requests.is_empty());
        assert!(
            result.breakdown_comment.contains("Work Tracking")
                || result.breakdown_comment.contains("epic")
        );
    }

    #[test]
    fn test_execute_breakdown_epic_scale() {
        let body = r#"
## Use Case
Multi-area feature.

## Acceptance Criteria
- [ ] API works
- [ ] UI works
- [ ] Data persisted
"#;
        let result = execute_breakdown(
            body,
            "Full-stack feature",
            42,
            "https://github.com/org/repo/issues/42",
            false,
        );

        assert!(result.is_epic_scale);
        assert!(!result.child_requests.is_empty());
        assert!(result.breakdown_comment.contains("deferred"));
    }

    #[test]
    fn test_extract_acceptance_criteria() {
        let body = r#"
## Use Case
Test feature.

## Acceptance Criteria
- [ ] Feature works
- [ ] Tests pass

## Notes
More info here.
"#;
        let ac = extract_acceptance_criteria_text(body);
        assert!(ac.contains("Feature works") || ac.contains("Feature"));
    }

    #[test]
    fn test_detect_child_bead_areas_multiple() {
        let body = "api endpoint, database storage, ui interface";
        let areas = detect_child_bead_areas(body);
        assert!(areas.len() >= 2);
        assert!(areas.iter().any(|a| a.0 == "api"));
        assert!(areas.iter().any(|a| a.0 == "database"));
        assert!(areas.iter().any(|a| a.0 == "ui"));
    }

    #[test]
    fn test_detect_child_bead_areas_auth() {
        let body = "login authentication permission";
        let areas = detect_child_bead_areas(body);
        assert!(areas.iter().any(|a| a.0 == "auth"));
    }

    #[test]
    fn test_detect_child_bead_areas_empty() {
        let body = "simple feature request";
        let areas = detect_child_bead_areas(body);
        assert!(areas.is_empty());
    }

    #[test]
    fn test_count_codebase_areas() {
        assert_eq!(count_codebase_areas("api endpoint rest"), 1);
        assert_eq!(count_codebase_areas("api and database and ui"), 3);
        assert_eq!(count_codebase_areas("simple text"), 0);
    }

    #[test]
    fn test_count_codebase_areas_auth_api_ui_database() {
        // Verify that auth, api, ui, and database are each detected as separate areas
        let body = "authentication api login ui interface database";
        let count = count_codebase_areas(body);
        assert_eq!(count, 4, "auth+api+ui+database should be 4 areas");
    }

    #[test]
    fn test_detect_sequential_work() {
        assert!(detect_sequential_work("first do X and then do Y") >= 2);
        assert!(detect_sequential_work("step 1 then step 2") >= 2);
        assert!(detect_sequential_work("depends on the auth") >= 1);
    }

    #[test]
    fn test_count_implementation_steps() {
        assert!(count_implementation_steps("step 1. step 2. step 3.") >= 3);
        assert_eq!(count_implementation_steps("just one step"), 0);
    }

    #[test]
    fn test_generate_child_beads_falls_back_to_ac_units() {
        let body = r#"
## Acceptance Criteria
- [ ] API works
- [ ] UI displays
- [ ] Data saves
"#;
        let beads = generate_child_beads(body, 1, false);
        assert!(!beads.is_empty());
    }

    #[test]
    fn test_extract_ac_logical_units() {
        let body = r#"
- [ ] API works
- [ ] UI displays  
- [ ] Data saves
- [ ] Tests pass
"#;
        let units = extract_ac_logical_units(body);
        assert!(units.len() >= 2);
    }

    #[test]
    fn test_bug_type_sets_bead_type() {
        let body = r#"
## What Happened
Bug.

## Acceptance Criteria
- [ ] Fixed
"#;
        let result = execute_breakdown(
            body,
            "Bug fix",
            1,
            "https://github.com/org/repo/issues/1",
            true, // is_bug = true
        );

        assert_eq!(
            result.epic_request.bead_type,
            crate::beads::client::BeadType::Bug
        );
    }

    #[test]
    fn test_feature_type_sets_bead_type() {
        let body = "Simple feature request.";
        let result = execute_breakdown(
            body,
            "Feature",
            1,
            "https://github.com/org/repo/issues/1",
            false, // is_bug = false
        );

        assert_eq!(
            result.epic_request.bead_type,
            crate::beads::client::BeadType::Feature
        );
    }

    #[test]
    fn test_epic_status_is_deferred() {
        let result = execute_breakdown(
            "Feature body.",
            "Title",
            1,
            "https://github.com/org/repo/issues/1",
            false,
        );

        assert_eq!(
            result.epic_request.status,
            crate::beads::client::BeadStatus::Deferred
        );
    }

    #[test]
    fn test_child_beads_status_is_deferred() {
        let body = "api endpoint, database storage";
        let result = execute_breakdown(
            body,
            "Title",
            1,
            "https://github.com/org/repo/issues/1",
            false,
        );

        for child in &result.child_requests {
            assert_eq!(child.status, crate::beads::client::BeadStatus::Deferred);
        }
    }

    #[test]
    fn test_epic_scale_result_includes_reasons() {
        let body = "api endpoint, database storage, user interface";
        let result = analyze_epic_scale(body, 1, false);
        assert!(!result.reasons.is_empty());
    }

    // =============================================================================
    // Standalone Bead Tests (CRIT-5)
    // =============================================================================

    #[test]
    fn test_standalone_bead_has_all_sections() {
        let bead = StandaloneBead::with_sections(
            "Create file.txt",
            "User needs this file",
            "cat file.txt shows content",
            "Handle empty file",
            "None",
        );

        assert!(bead.has_all_sections());
    }

    #[test]
    fn test_standalone_bead_missing_sections() {
        let bead = StandaloneBead::with_sections(
            "Create file.txt",
            "",
            "cat file.txt shows content",
            "Handle empty file",
            "None",
        );

        assert!(!bead.has_all_sections());
    }

    #[test]
    fn test_standalone_bead_single_codebase_part() {
        // API only - should pass
        let api_bead = StandaloneBead::with_sections(
            "Implement API handler in src/api/",
            "User-visible API endpoint",
            "curl test passes",
            "Handle None gracefully",
            "Handler: API processing function",
        );

        assert!(api_bead.is_single_codebase_part());

        // Multiple areas - should fail
        let multi_bead = StandaloneBead::with_sections(
            "Implement in CLI and API",
            "Both areas need this",
            "Test both",
            "Handle both",
            "None",
        );

        assert!(!multi_bead.is_single_codebase_part());
    }

    #[test]
    fn test_standalone_bead_compound_pattern_detection() {
        // Compound bead with "and then"
        let compound_bead = StandaloneBead::with_sections(
            "First do X to the database, and then do Y to the API",
            "Sequential work",
            "Test both steps",
            "Handle ordering",
            "None",
        );

        assert!(compound_bead.has_compound_pattern());
        assert!(!compound_bead.is_standalone_ready().passed());

        // Sequential steps pattern
        let steps_bead = StandaloneBead::with_sections(
            "Step 1: Create schema. Step 2: Add data.",
            "Migration work",
            "Run migration",
            "Handle rollback",
            "None",
        );

        assert!(steps_bead.has_compound_pattern());

        // Non-compound - clean single unit
        let clean_bead = StandaloneBead::with_sections(
            "Implement database schema for user table",
            "Store user data persistently",
            "Query user by ID succeeds",
            "Handle missing fields",
            "Entity: database model object",
        );

        assert!(!clean_bead.has_compound_pattern());
        assert!(clean_bead.is_standalone_ready().passed());
    }

    #[test]
    fn test_standalone_bead_is_standalone_ready_full() {
        let bead = StandaloneBead::with_sections(
            "Implement the weather API endpoint",
            "Expose weather data to clients via REST",
            "curl /api/weather returns JSON with data",
            "- Rate limit requests\n- Handle API key rotation\n- Cache responses for 5 minutes",
            "**Weather API**: single GET endpoint returning JSON forecast data",
        );

        let validation = bead.is_standalone_ready();
        assert!(validation.passed());
        assert!(validation.issues.is_empty());
    }

    #[test]
    fn test_standalone_bead_validation_multiple_issues() {
        let bead = StandaloneBead::with_sections(
            "", // Missing WHAT
            "", // Missing WHY
            "", // Missing HOW
            "", // Missing EDGE
            "", // Missing TERMS
        );

        let validation = bead.is_standalone_ready();
        assert!(!validation.passed());
        assert!(
            validation
                .issues
                .contains(&StandaloneIssue::MissingSections)
        );
    }

    #[test]
    fn test_standalone_validation_descriptions() {
        let validation = StandaloneValidation::from_issues(vec![
            StandaloneIssue::MissingSections,
            StandaloneIssue::MultipleCodebaseParts,
        ]);

        let descriptions = validation.descriptions();
        assert!(descriptions.len() == 2);
        assert!(descriptions[0].contains("Missing"));
        assert!(descriptions[1].contains("multiple"));
    }

    #[test]
    fn test_standalone_bead_to_markdown() {
        let bead = StandaloneBead::with_sections(
            "Create feature X",
            "Users need this",
            "Run tests",
            "Handle errors",
            "Feature X: new capability",
        );

        let md = bead.to_markdown();
        assert!(md.contains("WHAT TO DO"));
        assert!(md.contains("Create feature X"));
        assert!(md.contains("WHY"));
        assert!(md.contains("HOW TO VERIFY"));
        assert!(md.contains("EDGE CASES"));
        assert!(md.contains("TERMINOLOGY"));
    }

    #[test]
    fn test_generate_standalone_bead_api() {
        let bead = generate_standalone_bead(
            "API",
            "user profile endpoints",
            &[
                "AC-1: Profile displays correctly",
                "AC-2: Profile updates persist",
            ],
        );

        assert!(bead.has_all_sections());
        assert!(bead.is_standalone_ready().passed());
        assert!(bead.what_to_do.contains("API"));
        assert!(bead.why.contains("AC-1"));
    }

    #[test]
    fn test_generate_standalone_bead_database() {
        let bead = generate_standalone_bead(
            "Database",
            "user table schema",
            &["AC-1: Data persists", "AC-2: Queries are fast"],
        );

        assert!(bead.has_all_sections());
        assert!(bead.what_to_do.contains("database"));
        assert!(bead.what_to_do.contains("schema"));
    }

    #[test]
    fn test_generate_standalone_bead_ui() {
        let bead = generate_standalone_bead(
            "UI",
            "dashboard components",
            &["AC-1: Dashboard loads", "AC-2: Charts render"],
        );

        assert!(bead.has_all_sections());
        assert!(bead.what_to_do.contains("UI"));
        assert!(bead.how_to_verify.contains("dev server"));
    }

    #[test]
    fn test_generate_standalone_bead_cli() {
        let bead = generate_standalone_bead("CLI", "export command", &["AC-1: CSV export works"]);

        assert!(bead.has_all_sections());
        assert!(bead.what_to_do.contains("CLI"));
        assert!(bead.how_to_verify.contains("--help"));
    }

    #[test]
    fn test_validate_beads_standalone_all_pass() {
        let beads = vec![
            generate_standalone_bead("API", "endpoint 1", &[]),
            generate_standalone_bead("UI", "component 1", &[]),
            generate_standalone_bead("DB", "schema 1", &[]),
        ];

        let result = validate_beads_standalone(&beads);
        assert!(result.all_standalone);
        assert_eq!(result.individual_results.len(), 3);
        for (_, validation) in result.individual_results {
            assert!(validation.passed());
        }
    }

    #[test]
    fn test_validate_beads_standalone_one_fails() {
        let mut beads = vec![
            generate_standalone_bead("API", "endpoint 1", &[]),
            generate_standalone_bead("UI", "component 1", &[]),
        ];

        // Corrupt one bead to have multiple issues
        if let Some(bad) = beads.get_mut(0) {
            bad.what_to_do = "CLI and API combined".to_string();
            bad.why = "Sequential work: first do API, and then do CLI".to_string();
        }

        let result = validate_beads_standalone(&beads);
        assert!(!result.all_standalone);
        // First bead should fail (compound + multiple areas)
        assert!(!result.individual_results[0].1.passed());
        // Second should pass
        assert!(result.individual_results[1].1.passed());
    }

    #[test]
    fn test_standalone_issue_description() {
        assert!(
            StandaloneIssue::MissingSections
                .description()
                .contains("Missing")
        );
        assert!(
            StandaloneIssue::MultipleCodebaseParts
                .description()
                .to_lowercase()
                .contains("multiple")
        );
        assert!(
            StandaloneIssue::CompoundPattern
                .description()
                .contains("compound")
        );
    }

    #[test]
    fn test_closely_related_areas_allows_api_db() {
        // API + Database alone should be allowed as closely related
        let bead = StandaloneBead::with_sections(
            "API handler with database access",
            "Fetch user data",
            "Test endpoint",
            "Handle DB errors",
            "None",
        );

        // This bead mentions API and DB, which are closely related
        assert!(bead.is_single_codebase_part());
    }
}
