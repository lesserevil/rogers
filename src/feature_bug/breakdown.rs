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
    // "step N" patterns (step 1, step 2, etc.)
    let has_step_1 = body.contains("step 1") || body.contains("step one");
    let has_step_2 = body.contains("step 2")
        || body.contains("step two")
        || body.contains("step 3")
        || body.contains("step three");
    if has_step_1 && has_step_2 {
        score += 1;
    }
    // Numbered patterns like "1." followed by "2."
    if body.contains("1.") && body.contains("2.") {
        score += 1;
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
}
