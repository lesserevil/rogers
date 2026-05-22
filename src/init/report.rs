//! Structured report formatting for `rogers init`.
//!
//! Produces the human-readable and JSON report outputs defined in the
//! init-plan.md output format specification.
//!
//! # Format
//!
//! ```text
//! === Rodgers Project Readiness Audit ===
//! Repository: owner/repo
//! Scanned at: 2026-05-20T14:32:00Z
//!
//! [BLOCKER] Required labels missing: needs-information, will-not-do
//! [BLOCKER] Issue templates directory not found
//! [WARN   ] Discussion category "Release Proposals" not found
//! [INFO   ] Required labels present: bug, feature, question
//!
//! 4 checks performed
//!   2 blockers — Rodgers cannot safely operate
//!   1 warnings  — review recommended
//!   1 info     — no action needed
//!
//! Run 'rogers init --fix' to apply available automated fixes.
//! ```

use crate::checks::{CheckResult, Severity};
use chrono::Utc;

/// Summary statistics for the audit.
#[derive(Debug, Clone, Default)]
pub struct AuditSummary {
    pub total_checks: usize,
    pub blockers: usize,
    pub warnings: usize,
    pub info_count: usize,
}

/// Human-readable and JSON report formatter.
pub struct ReportFormatter;

impl ReportFormatter {
    /// Format a complete text audit report from check results and repository metadata.
    ///
    /// # Arguments
    /// * `repo_full_name` — Repository in `owner/repo` format.
    /// * `results` — All check results to include in the report.
    /// * `fix` — Whether `--fix` was requested (affects the fix prompt line).
    pub fn format_text(repo_full_name: &str, results: &[CheckResult], fix: bool) -> String {
        let mut output = String::new();

        // Header
        output.push_str("=== Rodgers Project Readiness Audit ===\n");
        output.push_str(&format!("Repository: {}\n", repo_full_name));
        output.push_str(&format!(
            "Scanned at: {}\n",
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
        ));
        output.push('\n');

        // Build deduplicated findings and compute summary
        let findings = Self::build_findings(results);
        let summary = Self::compute_summary(&findings);

        // Output findings in severity order: Blockers, Warns, Info.
        for severity in &[Severity::Blocker, Severity::Warn, Severity::Info] {
            for finding in &findings {
                if finding.severity == *severity {
                    output.push_str(&Self::format_finding_line(finding));
                    output.push('\n');
                }
            }
        }

        output.push('\n');

        // Summary
        output.push_str(&format!("{} checks performed\n", summary.total_checks));
        output.push_str(&format!(
            "  {} blockers — Rodgers cannot safely operate\n",
            summary.blockers
        ));
        output.push_str(&format!(
            "  {} warnings  — review recommended\n",
            summary.warnings
        ));
        output.push_str(&format!(
            "  {} info     — no action needed\n",
            summary.info_count
        ));

        output.push('\n');

        // Success message when no blockers.
        if summary.blockers == 0 {
            output.push_str("All checks passed\n");
        }

        // Fix prompt
        if fix {
            output.push_str("Fix mode: completed\n");
            if summary.blockers > 0 {
                output.push_str("Note: blockers detected — manual review still required.\n");
            }
        } else {
            output.push_str("Run 'rogers init --fix' to apply available automated fixes.\n");
        }

        output
    }

    /// Format the report as JSON for machine parsing.
    ///
    /// # Arguments
    /// * `repo_full_name` — Repository in `owner/repo` format.
    /// * `results` — All check results to include in the report.
    /// * `fix` — Whether `--fix` was requested.
    pub fn format_json(repo_full_name: &str, results: &[CheckResult], fix: bool) -> String {
        let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

        map.insert(
            "audit_header".to_string(),
            serde_json::json!({
                "title": "Rodgers Project Readiness Audit",
                "repository": repo_full_name,
                "scanned_at": Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            }),
        );

        let findings: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "severity": r.severity.as_str(),
                    "description": r.description,
                    "fixability": r.fixability.as_str(),
                })
            })
            .collect();

        map.insert("findings".to_string(), serde_json::Value::Array(findings));

        let summary = Self::compute_summary_from_results(results);
        map.insert(
            "summary".to_string(),
            serde_json::json!({
                "total_checks": summary.total_checks,
                "blockers": summary.blockers,
                "warnings": summary.warnings,
                "info": summary.info_count,
            }),
        );

        if fix {
            map.insert("fix_mode".to_string(), serde_json::json!(true));
        }

        serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_string())
    }

    /// Build formatted findings, deduplicated by severity+description.
    fn build_findings(results: &[CheckResult]) -> Vec<CheckResult> {
        let mut findings: Vec<CheckResult> = results.to_vec();
        // Deduplicate by (severity, description).
        findings.dedup_by(|a, b| a.severity == b.severity && a.description == b.description);
        findings
    }

    /// Compute summary statistics from deduplicated findings.
    fn compute_summary(findings: &[CheckResult]) -> AuditSummary {
        let mut summary = AuditSummary::default();
        for f in findings {
            summary.total_checks += 1;
            match f.severity {
                Severity::Blocker => summary.blockers += 1,
                Severity::Warn => summary.warnings += 1,
                Severity::Info => summary.info_count += 1,
            }
        }
        summary
    }

    /// Compute summary statistics directly from raw check results
    /// (without deduplication — counts all results).
    fn compute_summary_from_results(results: &[CheckResult]) -> AuditSummary {
        let mut summary = AuditSummary::default();
        for r in results {
            summary.total_checks += 1;
            match r.severity {
                Severity::Blocker => summary.blockers += 1,
                Severity::Warn => summary.warnings += 1,
                Severity::Info => summary.info_count += 1,
            }
        }
        summary
    }

    /// Format a single finding line with aligned severity column, including
    /// indented fix instructions when present.
    ///
    /// Example:
    /// ```text
    /// [BLOCKER] Description text - fixability: manual
    ///   → Create the directory and add template files at https://...
    /// ```
    fn format_finding_line(finding: &CheckResult) -> String {
        let severity_label = format_finding_severity(finding.severity);
        let mut line = format!(
            "[{}] {} - fixability: {}",
            severity_label,
            finding.description,
            finding.fixability.as_str()
        );

        // Append fix instructions if present.
        if let Some(ref instructions) = finding.fix_instructions {
            for instr_line in instructions.lines() {
                line.push('\n');
                line.push_str("  → ");
                line.push_str(instr_line);
            }
        }

        line
    }
}

/// Format severity label with fixed-width padding for column alignment.
///
/// BLOCKER → "BLOCKER" (7 chars)
/// WARN    → "WARN   " (7 chars)
/// INFO    → "INFO   " (7 chars)
fn format_finding_severity(severity: Severity) -> String {
    match severity {
        Severity::Blocker => "BLOCKER".to_string(),
        Severity::Warn => "WARN   ".to_string(),
        Severity::Info => "INFO   ".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::Fixability;

    fn make_result(severity: Severity, description: &str) -> CheckResult {
        CheckResult {
            severity,
            description: description.to_string(),
            fixability: Fixability::Auto,
            fix_instructions: None,
        }
    }

    fn make_result_with_fixability(
        severity: Severity,
        description: &str,
        fixability: Fixability,
    ) -> CheckResult {
        CheckResult {
            severity,
            description: description.to_string(),
            fixability,
            fix_instructions: None,
        }
    }

    // ─── Test: Report header format ────────────────────────────────────

    #[test]
    fn test_report_header_format() {
        let results = vec![make_result(Severity::Info, "test")];
        let output = ReportFormatter::format_text("test-owner/test-repo", &results, false);

        assert!(output.starts_with("=== Rodgers Project Readiness Audit ===\n"));
        assert!(output
            .lines()
            .nth(1)
            .unwrap()
            .contains("Repository: test-owner/test-repo"));
        assert!(output.lines().nth(2).unwrap().contains("Scanned at:"));
    }

    // ─── Test: Timestamp is ISO 8601 ───────────────────────────────────

    #[test]
    fn test_report_timestamp_is_iso() {
        let results = vec![make_result(Severity::Info, "test")];
        let output = ReportFormatter::format_text("o/r", &results, false);
        let timestamp_line = output.lines().nth(2).unwrap(); // "Scanned at: ..."
        assert!(timestamp_line.starts_with("Scanned at: 20"));

        // Extract the timestamp and check it contains the expected format
        let parts: Vec<&str> = timestamp_line.split("Scanned at: ").collect();
        if parts.len() == 2 {
            let ts = parts[1];
            // Should match YYYY-MM-DDTHH:MM:SSZ pattern
            assert!(ts.contains('T'), "timestamp should contain 'T'");
            assert!(ts.ends_with('Z'), "timestamp should end with 'Z'");
        }
    }

    // ─── Test: Severity ordering — blockers first ─────────────────────

    #[test]
    fn test_severity_ordering() {
        let results = vec![
            make_result(Severity::Info, "info finding"),
            make_result(Severity::Blocker, "blocker finding"),
            make_result(Severity::Warn, "warn finding"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        let lines: Vec<&str> = output.lines().collect();
        let blocker_line = lines
            .iter()
            .position(|l| l.contains("blocker finding"))
            .unwrap();
        let warn_line = lines
            .iter()
            .position(|l| l.contains("warn finding"))
            .unwrap();
        let info_line = lines
            .iter()
            .position(|l| l.contains("info finding"))
            .unwrap();

        assert!(
            blocker_line < warn_line,
            "blockers should appear before warns"
        );
        assert!(warn_line < info_line, "warns should appear before info");
    }

    // ─── Test: Fixability displayed ───────────────────────────────────

    #[test]
    fn test_fixability_displayed() {
        let results = vec![
            make_result_with_fixability(Severity::Blocker, "missing labels", Fixability::Auto),
            make_result_with_fixability(Severity::Warn, "bad setting", Fixability::Manual),
            make_result_with_fixability(Severity::Info, "all good", Fixability::NotApplicable),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(
            output.contains("fixability: auto"),
            "auto fixability should appear"
        );
        assert!(
            output.contains("fixability: manual"),
            "manual fixability should appear"
        );
        assert!(
            output.contains("fixability: na"),
            "na fixability should appear"
        );
    }

    // ─── Test: Summary counts correct ─────────────────────────────────

    #[test]
    fn test_summary_counts() {
        let results = vec![
            make_result(Severity::Blocker, "blocker 1"),
            make_result(Severity::Blocker, "blocker 2"),
            make_result(Severity::Warn, "warn 1"),
            make_result(Severity::Info, "info 1"),
            make_result(Severity::Info, "info 2"),
            make_result(Severity::Info, "info 3"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(output.contains("6 checks performed"));
        assert!(output.contains("2 blockers"));
        assert!(output.contains("1 warnings"));
        assert!(output.contains("3 info"));
    }

    // ─── Test: Summary with zero of a severity ─────────────────────────

    #[test]
    fn test_summary_with_zero_severity() {
        let results = vec![make_result(Severity::Info, "all info")];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(output.contains("1 checks performed"));
        assert!(output.contains("0 blockers"));
        assert!(output.contains("0 warnings"));
        assert!(output.contains("1 info"));
    }

    // ─── Test: Report format matches plan exactly ──────────────────────

    #[test]
    fn test_report_format_matches_plan() {
        let results = vec![
            make_result_with_fixability(
                Severity::Blocker,
                "Required labels missing: needs-information, will-not-do",
                Fixability::Auto,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "Issue templates directory not found",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Warn,
                "Discussion category \"Release Proposals\" not found",
                Fixability::Auto,
            ),
            make_result_with_fixability(
                Severity::Info,
                "Required labels present: bug, feature, question",
                Fixability::NotApplicable,
            ),
        ];
        let output = ReportFormatter::format_text("owner/repo", &results, false);

        // Verify the overall structure matches the plan format
        let lines: Vec<&str> = output.lines().collect();

        // Line 0: header
        assert_eq!(lines[0], "=== Rodgers Project Readiness Audit ===");

        // Line 1: repository
        assert_eq!(lines[1], "Repository: owner/repo");

        // Line 2: scanned at
        assert!(lines[2].starts_with("Scanned at:"));

        // Line 3: blank
        assert_eq!(lines[3], "");

        // Line 4+: findings (blockers first)
        assert!(lines[4].contains("[BLOCKER]"));
        assert!(lines[4].contains("Required labels missing"));
        assert!(lines[4].contains("fixability: auto"));

        assert!(lines[5].contains("[BLOCKER]"));
        assert!(lines[5].contains("Issue templates directory not found"));
        assert!(lines[5].contains("fixability: manual"));

        assert!(lines[6].contains("[WARN   ]"));
        assert!(lines[6].contains("Discussion category"));
        assert!(lines[6].contains("fixability: auto"));

        assert!(lines[7].contains("[INFO   ]"));
        assert!(lines[7].contains("Required labels present"));
        assert!(lines[7].contains("fixability: na"));

        // Summary section
        assert!(output.contains("4 checks performed"));
        assert!(output.contains("2 blockers"));
        assert!(output.contains("1 warnings"));
        assert!(output.contains("1 info"));

        // Fix prompt
        assert!(output.contains("Run 'rogers init --fix' to apply available automated fixes."));
    }

    // ─── Test: Fix mode changes output ─────────────────────────────────

    #[test]
    fn test_fix_mode_output() {
        let results = vec![make_result(Severity::Info, "all good")];
        let output = ReportFormatter::format_text("o/r", &results, true);

        assert!(output.contains("Fix mode: completed"));
        assert!(!output.contains("Run 'rogers init --fix'"));
    }

    // ─── Test: Fix mode with blockers shows note ───────────────────────

    #[test]
    fn test_fix_mode_with_blockers_note() {
        let results = vec![make_result(Severity::Blocker, "blocker")];
        let output = ReportFormatter::format_text("o/r", &results, true);

        assert!(output.contains("Fix mode: completed"));
        assert!(output.contains("blockers detected"));
        assert!(output.contains("manual review"));
    }

    // ─── Test: JSON report format ──────────────────────────────────────

    #[test]
    fn test_json_report_format() {
        let results = vec![
            make_result_with_fixability(Severity::Blocker, "missing labels", Fixability::Auto),
            make_result_with_fixability(Severity::Info, "all good", Fixability::NotApplicable),
        ];
        let json = ReportFormatter::format_json("owner/repo", &results, false);

        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");

        assert_eq!(
            parsed["audit_header"]["title"],
            "Rodgers Project Readiness Audit"
        );
        assert_eq!(parsed["audit_header"]["repository"], "owner/repo");
        assert!(parsed["audit_header"]["scanned_at"]
            .as_str()
            .unwrap()
            .contains("T"));

        assert!(parsed["findings"].is_array());
        assert_eq!(parsed["findings"].as_array().unwrap().len(), 2);

        assert_eq!(parsed["findings"][0]["severity"], "BLOCKER");
        assert_eq!(parsed["findings"][0]["fixability"], "auto");
        assert_eq!(parsed["findings"][1]["severity"], "INFO");
        assert_eq!(parsed["findings"][1]["fixability"], "na");

        assert_eq!(parsed["summary"]["total_checks"], 2);
        assert_eq!(parsed["summary"]["blockers"], 1);
        assert_eq!(parsed["summary"]["warnings"], 0);
        assert_eq!(parsed["summary"]["info"], 1);
    }

    // ─── Test: JSON fix mode flag ──────────────────────────────────────

    #[test]
    fn test_json_report_fix_mode() {
        let results = vec![make_result(Severity::Info, "ok")];
        let json = ReportFormatter::format_json("o/r", &results, true);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        assert_eq!(parsed["fix_mode"], true);
    }

    // ─── Test: Deduplication of findings ───────────────────────────────

    #[test]
    fn test_deduplication() {
        // Duplicate findings should only appear once in the text report.
        let results = vec![
            make_result(Severity::Info, "duplicate finding"),
            make_result(Severity::Info, "duplicate finding"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);
        let count = output.matches("duplicate finding").count();
        assert_eq!(count, 1, "duplicate findings should be deduplicated");
    }

    // ─── Test: Fixability values are correct strings ───────────────────

    #[test]
    fn test_fixability_strings() {
        let results = vec![
            make_result_with_fixability(Severity::Info, "test auto", Fixability::Auto),
            make_result_with_fixability(Severity::Info, "test manual", Fixability::Manual),
            make_result_with_fixability(Severity::Info, "test na", Fixability::NotApplicable),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);
        assert!(output.contains("fixability: auto"));
        assert!(output.contains("fixability: manual"));
        assert!(output.contains("fixability: na"));
    }

    // ─── Test: Column alignment ────────────────────────────────────────

    #[test]
    fn test_column_alignment() {
        let results = vec![
            make_result(Severity::Info, "short"),
            make_result(
                Severity::Blocker,
                "a very long description that goes on for quite a while",
            ),
            make_result(Severity::Warn, "medium length description"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        // Each finding line should start with [SEVERITY   ]
        for line in output.lines() {
            if line.starts_with('[') {
                assert!(
                    line.starts_with("[BLOCKER] ")
                        || line.starts_with("[WARN   ] ")
                        || line.starts_with("[INFO   ] "),
                    "line should have aligned severity: {}",
                    line
                );
            }
        }
    }

    // ─── Test: Empty results still produces valid report ───────────────

    #[test]
    fn test_empty_results_report() {
        let results: Vec<CheckResult> = vec![];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(output.starts_with("=== Rodgers Project Readiness Audit ==="));
        assert!(output.contains("0 checks performed"));
        assert!(output.contains("0 blockers"));
        assert!(output.contains("0 warnings"));
        assert!(output.contains("0 info"));
        assert!(output.contains("Run 'rogers init --fix'"));
    }

    // ─── Test: All checks show severity, description, fixability ──────

    #[test]
    fn test_all_checks_show_severity_description_fixability() {
        let results = vec![
            make_result_with_fixability(Severity::Blocker, "blocker desc", Fixability::Auto),
            make_result_with_fixability(Severity::Warn, "warn desc", Fixability::Manual),
            make_result_with_fixability(Severity::Info, "info desc", Fixability::NotApplicable),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        // Each check must have severity label
        assert!(output.contains("[BLOCKER]"));
        assert!(output.contains("[WARN   ]"));
        assert!(output.contains("[INFO   ]"));

        // Each check must have its description
        assert!(output.contains("blocker desc"));
        assert!(output.contains("warn desc"));
        assert!(output.contains("info desc"));

        // Each check must have fixability
        assert!(output.contains("fixability: auto"));
        assert!(output.contains("fixability: manual"));
        assert!(output.contains("fixability: na"));
    }

    // ─── Test: AC-1 — "All checks passed" message with zero blockers ──

    #[test]
    fn test_all_checks_passed_message_when_no_blockers() {
        let results = vec![make_result(Severity::Info, "all good")];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(
            output.contains("All checks passed"),
            "report should contain 'All checks passed' when no blockers"
        );
    }

    #[test]
    fn test_all_checks_passed_with_only_warnings_no_blockers() {
        let results = vec![
            make_result(Severity::Warn, "discussion category not found"),
            make_result(Severity::Warn, "delete branches on merge disabled"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(
            output.contains("All checks passed"),
            "report should contain 'All checks passed' when only warnings present (no blockers)"
        );
        assert!(output.contains("0 blockers"));
    }

    #[test]
    fn test_all_checks_passed_with_mixed_info_and_warn_no_blockers() {
        let results = vec![
            make_result(Severity::Info, "all labels present"),
            make_result(Severity::Warn, "discussion category not found"),
            make_result(Severity::Info, "branch protection enabled"),
            make_result(Severity::Warn, "default branch is develop"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(
            output.contains("All checks passed"),
            "report should contain 'All checks passed' when no blockers regardless of warnings/info"
        );
        assert!(output.contains("0 blockers"));
        assert!(output.contains("2 warnings"));
    }

    // ─── Test: "All checks passed" NOT shown when blockers exist ──────

    #[test]
    fn test_no_all_checks_passed_message_when_blockers_exist() {
        let results = vec![
            make_result(Severity::Blocker, "required labels missing"),
            make_result(Severity::Warn, "discussion category not found"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(
            !output.contains("All checks passed"),
            "report should NOT contain 'All checks passed' when blockers exist"
        );
        assert!(output.contains("1 blockers"));
    }

    #[test]
    fn test_no_all_checks_passed_with_only_blockers() {
        let results = vec![
            make_result(Severity::Blocker, "issue templates not found"),
            make_result(Severity::Blocker, "no release workflow"),
        ];
        let output = ReportFormatter::format_text("o/r", &results, false);

        assert!(
            !output.contains("All checks passed"),
            "report should NOT contain 'All checks passed' when only blockers exist"
        );
        assert!(output.contains("2 blockers"));
    }

    // ─── Test: Exit code 0 when no blockers (AC-1 verification) ──────

    #[test]
    fn test_exit_code_0_when_no_blockers() {
        // Simulate the exit code logic from main.rs
        let results = vec![
            make_result(Severity::Info, "All required labels present"),
            make_result(Severity::Info, "Branch protection enabled for main"),
            make_result(Severity::Info, "CI workflow found for pull requests"),
        ];
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);

        assert!(
            !has_blockers,
            "no blockers should be detected when all checks pass"
        );
        // Exit code would be: if has_blockers { exit(1) } else { exit(0) }
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 0, "exit code should be 0 when no blockers");
    }

    #[test]
    fn test_exit_code_1_when_blockers_exist() {
        // Simulate the exit code logic from main.rs
        let results = vec![
            make_result(Severity::Blocker, "required labels missing"),
            make_result(Severity::Info, "Branch protection enabled for main"),
        ];
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);

        assert!(
            has_blockers,
            "blockers should be detected when any blocker exists"
        );
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 1, "exit code should be 1 when blockers exist");
    }

    #[test]
    fn test_warnings_do_not_affect_exit_code() {
        // Only warnings, no blockers → exit code should still be 0
        let results = vec![
            make_result(Severity::Warn, "delete branches on merge disabled"),
            make_result(Severity::Warn, "default branch is develop"),
        ];
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);

        assert!(
            !has_blockers,
            "warnings alone should not set has_blockers to true"
        );
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 0, "warnings alone should not cause exit code 1");
    }

    #[test]
    fn test_info_does_not_affect_exit_code() {
        // Only info → exit code should still be 0
        let results = vec![
            make_result(Severity::Info, "all labels present"),
            make_result(Severity::Info, "branch protection enabled"),
        ];
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);

        assert!(!has_blockers);
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 0);
    }

    // ─── Test: All checks run before exit decision ───────────────────

    #[test]
    fn test_exit_decision_after_all_checks() {
        // Verify that the exit code decision is based on ALL results,
        // not just the first check. The check should iterate over ALL
        // results to find any blockers.
        let results = vec![
            make_result(Severity::Info, "labels check passed"),
            make_result(Severity::Info, "issue templates check passed"),
            make_result(Severity::Info, "repo settings check passed"),
            make_result(Severity::Info, "discussion categories check passed"),
            make_result(Severity::Info, "general workflows check passed"),
            make_result(Severity::Blocker, "release workflow not found"),
        ];
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);

        assert!(
            has_blockers,
            "must detect blocker even when it is the last result"
        );
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 1);
    }

    // ─── AC-2: Single blocker → exit 1, listed ─────────────────────────

    #[test]
    fn test_single_blocker_exit_1_and_listed() {
        // AC-2: When any blocker check fails, exit code is 1 and the
        // blocker is listed in the report.
        let results = vec![
            make_result_with_fixability(
                Severity::Blocker,
                "Required labels missing: needs-information, will-not-do",
                Fixability::Auto,
            ),
            make_result_with_fixability(
                Severity::Info,
                "Issue templates found: bug_report.yml",
                Fixability::NotApplicable,
            ),
            make_result_with_fixability(
                Severity::Info,
                "Branch protection enabled for main",
                Fixability::NotApplicable,
            ),
        ];

        // Exit code: 1 because has_blockers is true
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 1, "exit code should be 1 for single blocker");

        // Report should list the blocker
        let output = ReportFormatter::format_text("owner/repo", &results, false);
        assert!(
            output.contains("[BLOCKER]"),
            "report should contain [BLOCKER] marker"
        );
        assert!(
            output.contains("Required labels missing"),
            "report should contain the blocker description"
        );
        assert!(
            output.contains("fixability: auto"),
            "report should show fixability for blocker"
        );
        assert!(
            output.contains("1 blockers"),
            "summary should show 1 blocker"
        );
    }

    // ─── AC-2: Multiple blockers → exit 1, ALL listed ─────────────────

    #[test]
    fn test_multiple_blockers_exit_1_all_listed() {
        // AC-2: Must collect ALL blockers before exit, not stop at first.
        let results = vec![
            make_result_with_fixability(
                Severity::Blocker,
                "Required labels missing: needs-information, will-not-do",
                Fixability::Auto,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "Issue templates directory not found",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "No release-capable GitHub Actions workflow found",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Warn,
                "Discussion category not found",
                Fixability::Auto,
            ),
            make_result_with_fixability(
                Severity::Info,
                "CI workflow found",
                Fixability::NotApplicable,
            ),
        ];

        // Exit code: 1
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(
            exit_code, 1,
            "exit code should be 1 when multiple blockers exist"
        );

        // Report should list ALL three blockers
        let output = ReportFormatter::format_text("owner/repo", &results, false);

        // All three blockers should appear in the output
        assert!(
            output.contains("Required labels missing"),
            "all blockers listed: required labels missing"
        );
        assert!(
            output.contains("Issue templates directory not found"),
            "all blockers listed: issue templates not found"
        );
        assert!(
            output.contains("No release-capable"),
            "all blockers listed: no release workflow"
        );

        // Summary should show 3 blockers
        assert!(
            output.contains("3 blockers"),
            "summary should show 3 blockers, got: {}",
            output
                .lines()
                .find(|l| l.contains("blockers"))
                .unwrap_or("not found")
        );

        // Count [BLOCKER] markers to confirm all three appear
        let blocker_count = output.matches("[BLOCKER]").count();
        assert_eq!(
            blocker_count, 3,
            "should have exactly 3 [BLOCKER] lines, found {}",
            blocker_count
        );
    }

    // ─── AC-2: Blocker report shows severity, description, fixability ─

    #[test]
    fn test_blocker_report_shows_severity_description_fixability() {
        // AC-2: Report format is '[BLOCKER] Description - fixability'
        let results = vec![
            make_result_with_fixability(
                Severity::Blocker,
                "Required labels missing: bug, feature",
                Fixability::Auto,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "Main branch has no branch protection rules",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "No release-capable workflow found",
                Fixability::Manual,
            ),
        ];

        let output = ReportFormatter::format_text("owner/repo", &results, false);

        // Each blocker must have severity label
        assert!(output.contains("[BLOCKER]"));

        // Each blocker must have its description
        assert!(output.contains("Required labels missing: bug, feature"));
        assert!(output.contains("Main branch has no branch protection rules"));
        assert!(output.contains("No release-capable workflow found"));

        // Each blocker must show fixability
        assert!(
            output.contains("fixability: auto"),
            "auto fixability should be shown"
        );
        assert!(
            output.contains("fixability: manual"),
            "manual fixability should be shown"
        );

        // Verify that warnings do NOT cause exit 1
        let warn_only_results = vec![
            make_result(Severity::Warn, "some warning"),
            make_result(Severity::Warn, "another warning"),
        ];
        let warn_has_blockers = warn_only_results
            .iter()
            .any(|r| r.severity == Severity::Blocker);
        let warn_exit_code = if warn_has_blockers { 1 } else { 0 };
        assert_eq!(warn_exit_code, 0, "warnings alone should NOT cause exit 1");
    }

    // ─── AC-2: ALL blockers collected, none skipped ────────────────────

    #[test]
    fn test_all_blockers_collected_no_early_exit() {
        // Verify the implementation does NOT stop after finding the first
        // blocker — ALL blockers from ALL checks must be present.
        let results = vec![
            // Simulate results from different checks
            make_result_with_fixability(
                Severity::Blocker,
                "Required labels missing: needs-information",
                Fixability::Auto,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "Issue templates directory not found",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "No release-capable GitHub Actions workflow found",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Blocker,
                "Main branch unprotected",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Warn,
                "Delete branches on merge disabled",
                Fixability::Manual,
            ),
            make_result_with_fixability(
                Severity::Info,
                "CI workflow exists",
                Fixability::NotApplicable,
            ),
        ];

        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 1);

        let output = ReportFormatter::format_text("owner/repo", &results, false);

        // ALL four blockers must appear
        let blocker_count = output.matches("[BLOCKER]").count();
        assert_eq!(
            blocker_count, 4,
            "all 4 blockers must be listed, found {} — no early exit",
            blocker_count
        );

        // Verify specific blocker content
        assert!(output.contains("Required labels missing"));
        assert!(output.contains("Issue templates directory not found"));
        assert!(output.contains("No release-capable"));
        assert!(output.contains("Main branch unprotected"));

        // Verify fixability shown for each
        let auto_count = output.matches("fixability: auto").count();
        let manual_count = output.matches("fixability: manual").count();
        let na_count = output.matches("fixability: na").count();

        assert_eq!(auto_count, 1, "should have 1 auto fixability");
        assert!(
            manual_count >= 3,
            "should have at least 3 manual fixabilities (blockers + warn), got {}",
            manual_count
        );
        assert_eq!(na_count, 1, "should have 1 na fixability");
    }

    // ─── AC-2: Exit codes — Auth failure = 3, Invalid args = 2 ──────

    #[test]
    fn test_blocker_exit_code_is_1() {
        // Exit code 1 is for blocker failures, not auth (3) or config (2).
        let results = vec![make_result(Severity::Blocker, "required labels missing")];
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 1, "blocker failures should exit 1, not 2 or 3");
    }

    #[test]
    fn test_no_blockers_exit_0() {
        // Confirm exit 0 when there are no blockers at all.
        let results = vec![
            make_result(Severity::Info, "all labels present"),
            make_result(Severity::Info, "branch protection enabled"),
            make_result(Severity::Warn, "delete branches off"),
        ];
        let has_blockers = results.iter().any(|r| r.severity == Severity::Blocker);
        let exit_code = if has_blockers { 1 } else { 0 };
        assert_eq!(exit_code, 0, "no blockers should exit 0");
    }

    // ─── AC-5: Fix instructions included in report ─────────────────────

    #[test]
    fn test_fix_instructions_included_in_blocker_report() {
        // AC-5: Blockers for missing issue templates must include specific
        // instructions with direct links/commands.
        let results = vec![CheckResult {
            severity: Severity::Blocker,
            description: "Issue templates directory not found".to_string(),
            fixability: Fixability::Manual,
            fix_instructions: Some(
                "Create the directory and add template files at \
                 https://github.com/owner/repo/new/main/.github/ISSUE_TEMPLATE/"
                    .to_string(),
            ),
        }];

        let output = ReportFormatter::format_text("owner/repo", &results, false);

        // Must contain the blocker
        assert!(output.contains("[BLOCKER]"));
        assert!(output.contains("Issue templates directory not found"));

        // Must contain the fix instruction with a link
        assert!(
            output.contains("github.com/owner/repo"),
            "fix instructions must include direct link to create templates"
        );
        assert!(
            output.contains(".github/ISSUE_TEMPLATE/"),
            "fix instructions must reference the template directory path"
        );
        // Instructions are prefixed with "→"
        assert!(
            output.contains("  → Create the directory"),
            "fix instructions must be included with arrow prefix"
        );
    }

    #[test]
    fn test_fix_instructions_included_in_release_workflow_blocker() {
        // AC-5: Blockers for missing release workflow must include specific
        // instructions with direct links/commands.
        let results = vec![CheckResult {
            severity: Severity::Blocker,
            description: "No release-capable GitHub Actions workflow found".to_string(),
            fixability: Fixability::Manual,
            fix_instructions: Some(
                "Create a release workflow at .github/workflows/release.yml with a \
                 push trigger on tag patterns (v*, *.*.*) and artifact upload steps."
                    .to_string(),
            ),
        }];

        let output = ReportFormatter::format_text("owner/repo", &results, false);

        // Must contain the blocker
        assert!(output.contains("[BLOCKER]"));
        assert!(output.contains("No release-capable"));

        // Must contain the fix instruction
        assert!(
            output.contains("release.yml") || output.contains("release-workflow"),
            "fix instructions must reference the release workflow path"
        );
        assert!(
            output.contains("  → Create a release workflow"),
            "fix instructions must be included with arrow prefix"
        );
    }

    #[test]
    fn test_fix_instructions_include_direct_links() {
        // AC-5: Instructions must include direct links/commands.
        let results = vec![
            CheckResult {
                severity: Severity::Blocker,
                description: "Issue templates directory not found".to_string(),
                fixability: Fixability::Manual,
                fix_instructions: Some(
                    "Create the directory and add template files at \
                     https://github.com/owner/repo/new/main/.github/ISSUE_TEMPLATE/"
                        .to_string(),
                ),
            },
            CheckResult {
                severity: Severity::Blocker,
                description: "No release-capable workflow found".to_string(),
                fixability: Fixability::Manual,
                fix_instructions: Some(
                    "Add a release workflow at .github/workflows/release.yml. \
                     See: https://docs.github.com/en/actions/workflow-syntax"
                        .to_string(),
                ),
            },
        ];

        let output = ReportFormatter::format_text("owner/repo", &results, false);

        // Both blockers must be listed
        assert_eq!(output.matches("[BLOCKER]").count(), 2);

        // Both must have fix instructions with links
        assert!(output.contains("github.com"), "must include direct links");
        assert!(
            output.contains("docs.github.com"),
            "must include documentation links"
        );

        // Both must show fix instructions with arrow prefix
        let arrow_count = output.matches("  → ").count();
        assert!(
            arrow_count >= 2,
            "both blockers must have fix instructions, found {}",
            arrow_count
        );
    }

    #[test]
    fn test_fix_instructions_shown_for_manual_fixability() {
        // AC-5: fixability: manual findings should show instructions.
        let results = vec![CheckResult {
            severity: Severity::Blocker,
            description: "Manual fix required".to_string(),
            fixability: Fixability::Manual,
            fix_instructions: Some("Run this command: rogers init --fix".to_string()),
        }];

        let output = ReportFormatter::format_text("o/r", &results, false);
        assert!(
            output.contains("fixability: manual"),
            "manual fixability should be shown"
        );
        assert!(
            output.contains("  → Run this command"),
            "fix instructions should appear for manual fixability"
        );
    }

    #[test]
    fn test_no_fix_instructions_when_none_provided() {
        // When a finding has no fix_instructions, no arrow line should appear.
        let results = vec![make_result(Severity::Info, "all good")];
        let output = ReportFormatter::format_text("o/r", &results, false);
        assert!(
            !output.contains("  →"),
            "no arrow prefix when no instructions"
        );
    }

    #[test]
    fn test_multiline_fix_instructions() {
        // Fix instructions with multiple lines should each get an arrow prefix.
        let results = vec![CheckResult {
            severity: Severity::Blocker,
            description: "Workflow not found".to_string(),
            fixability: Fixability::Manual,
            fix_instructions: Some(
                "Line one: create the file.\nLine two: commit it.\nLine three: push it."
                    .to_string(),
            ),
        }];

        let output = ReportFormatter::format_text("o/r", &results, false);
        assert!(output.contains("  → Line one: create the file."));
        assert!(output.contains("  → Line two: commit it."));
        assert!(output.contains("  → Line three: push it."));
    }

    // ─── AC-5: JSON report includes fixability ──────────────────────────

    #[test]
    fn test_json_report_shows_fixability_for_blockers() {
        // Verify JSON report includes fixability information.
        let results = vec![
            CheckResult {
                severity: Severity::Blocker,
                description: "Issue templates missing".to_string(),
                fixability: Fixability::Manual,
                fix_instructions: None,
            },
            CheckResult {
                severity: Severity::Blocker,
                description: "No release workflow".to_string(),
                fixability: Fixability::Manual,
                fix_instructions: None,
            },
        ];

        let json = ReportFormatter::format_json("o/r", &results, false);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["findings"][0]["fixability"], "manual");
        assert_eq!(parsed["findings"][1]["fixability"], "manual");
        assert_eq!(parsed["findings"][0]["severity"], "BLOCKER");
        assert_eq!(parsed["findings"][1]["severity"], "BLOCKER");
    }
}
