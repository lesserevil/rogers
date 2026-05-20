//! Report generation for doctor command
//!
//! Formats doctor check results into human-readable or JSON output.

use super::{
    CATEGORY_AUTH, CATEGORY_BEADS, CATEGORY_CONFIG, CATEGORY_DRIFT, CATEGORY_PLANS, CATEGORY_REPO,
    CategoryResult, CategoryStatus, DoctorResult, DriftEvent, DriftSeverity,
};
use chrono::Utc;
use serde::Serialize;

/// Report output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output
    Json,
}

/// Report generator for doctor results
pub struct ReportGenerator {
    format: OutputFormat,
    verbose: bool,
    scan_time: String,
}

impl ReportGenerator {
    /// Create a new report generator
    pub fn new(format: OutputFormat, verbose: bool) -> Self {
        Self {
            format,
            verbose,
            scan_time: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        }
    }

    /// Generate a text report from doctor results
    pub fn generate_text(&self, result: &DoctorResult) -> String {
        let mut output = String::new();

        output.push_str("=== Rodgers Health Check ===\n");
        output.push_str(&format!("Scanned at: {}\n\n", self.scan_time));

        // Output each category result
        for category in &result.categories {
            let line = format_category_line(&category.name, &category.status);
            output.push_str(&line);
            output.push('\n');

            // Show messages if verbose
            if self.verbose && !category.messages.is_empty() {
                for msg in &category.messages {
                    output.push_str(&format!("  {}\n", msg));
                }
                output.push('\n');
            }
        }

        // Output drift summary
        if !result.drift_events.is_empty() {
            let drift_count = result.drift_events.len();
            output.push_str(&format!(
                "[drift   ] ⚠ DRIFT DETECTED — {} drift events found\n",
                drift_count
            ));

            if self.verbose {
                output.push_str("\n");
                self.format_verbose_drift_events(&mut output, &result.drift_events);
            }
        } else {
            output.push_str("[drift   ] ✓ No drift detected\n");
        }

        // Output overall summary
        output.push('\n');
        let passed = result.passed_count();
        let warned = result.warned_count();
        let failed = result.failed_count();
        let has_drift = result.has_drift();

        if failed > 0 {
            output.push_str(&format!(
                "Overall: {} categories OK, {} warnings, {} failures",
                passed, warned, failed
            ));
        } else if has_drift {
            output.push_str(&format!(
                "Overall: {} categories OK, {} warnings, drift detected",
                passed, warned
            ));
        } else {
            output.push_str(&format!("Overall: {} categories OK", passed));
            if warned > 0 {
                output.push_str(&format!(", {} warnings", warned));
            }
            output.push_str(" — Rodgers is healthy!");
        }

        output
    }

    /// Format verbose drift events with full details
    fn format_verbose_drift_events(&self, output: &mut String, events: &[DriftEvent]) {
        output.push_str("Drift events:\n");

        for (i, event) in events.iter().enumerate() {
            // Event number and type
            output.push_str(&format!(
                "  {}. [{:?}] {}: {}\n",
                i + 1,
                event.severity,
                event.event_type,
                event.description
            ));

            // GitHub issue URL if available
            if let Some(ref issue_url) = event.github_issue_url {
                output.push_str(&format!("     Issue URL: {}\n", issue_url));
            }

            // Bead ID if available
            if let Some(ref bead_id) = event.bead_id {
                output.push_str(&format!("     Bead ID: {}\n", bead_id));
            }

            // Linking info based on event type
            let linking_info = get_linking_info(event);
            if !linking_info.is_empty() {
                output.push_str(&format!("     Link: {}\n", linking_info));
            }

            output.push('\n');
        }

        output.push_str("Run 'rogers doctor --fix' to address drift (prompts for confirmation)\n");
    }

    /// Generate a JSON report from doctor results
    pub fn generate_json(&self, result: &DoctorResult) -> String {
        let report = JsonReport {
            scan_time: self.scan_time.clone(),
            categories: result.categories.clone(),
            drift_events: result.drift_events.clone(),
            summary: Summary {
                passed: result.passed_count(),
                warnings: result.warned_count(),
                failed: result.failed_count(),
                drift_count: result.drift_events.len(),
                is_healthy: result.is_healthy,
                exit_code: result.exit_code(),
            },
        };

        serde_json::to_string_pretty(&report).unwrap_or_default()
    }

    /// Generate the report based on configured format
    pub fn generate(&self, result: &DoctorResult) -> String {
        match self.format {
            OutputFormat::Text => self.generate_text(result),
            OutputFormat::Json => self.generate_json(result),
        }
    }
}

/// Format a single category line for text output
fn format_category_line(name: &str, status: &CategoryStatus) -> String {
    let padded_name = format!("[{:8}]", name);
    match status {
        CategoryStatus::Pass => format!("{} ✓ OK", padded_name),
        CategoryStatus::Warn(msgs) => {
            if msgs.is_empty() {
                format!("{} ⚠ warnings", padded_name)
            } else {
                format!("{} ⚠ {}", padded_name, msgs[0])
            }
        }
        CategoryStatus::Fail(msg) => format!("{} ✗ FAIL: {}", padded_name, msg),
        CategoryStatus::Skipped => format!("{} — skipped", padded_name),
    }
}

/// Get linking information based on drift event type
fn get_linking_info(event: &DriftEvent) -> String {
    match event.event_type.as_str() {
        "closed_bead_open_issue" => {
            if event.bead_id.is_some() && event.github_issue_url.is_some() {
                "Close the bead to match the GitHub issue state".to_string()
            } else {
                String::new()
            }
        }
        "in_progress_bead_closed_issue" => {
            if event.bead_id.is_some() && event.github_issue_url.is_some() {
                "Reopen the GitHub issue or close the bead".to_string()
            } else {
                String::new()
            }
        }
        "orphan_bead" => {
            if event.bead_id.is_some() {
                "Link the bead to a GitHub issue or mark as internal tracking".to_string()
            } else {
                String::new()
            }
        }
        "ready_for_work_no_bead" => {
            if event.github_issue_url.is_some() {
                "File a new bead to track this work".to_string()
            } else {
                String::new()
            }
        }
        "release_proposed_no_milestone" => {
            if event.github_issue_url.is_some() {
                "Assign the issue to the appropriate release milestone".to_string()
            } else {
                String::new()
            }
        }
        "convention_violation" => {
            if event.bead_id.is_some() {
                "Update the bead description to follow AGENTS.md conventions".to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

/// Human-readable name for drift event types
pub fn format_drift_type(event_type: &str) -> String {
    match event_type {
        "closed_bead_open_issue" => "Closed Bead / Open Issue".to_string(),
        "in_progress_bead_closed_issue" => "In-Progress Bead / Closed Issue".to_string(),
        "orphan_bead" => "Orphan Bead".to_string(),
        "ready_for_work_no_bead" => "Ready-for-Work Issue / No Bead".to_string(),
        "release_proposed_no_milestone" => "Release-Proposed / No Milestone".to_string(),
        "convention_violation" => "Convention Violation".to_string(),
        _ => event_type.to_string(),
    }
}

/// Format severity level for display
pub fn format_severity(severity: DriftSeverity) -> String {
    match severity {
        DriftSeverity::Warning => "⚠ Warning".to_string(),
        DriftSeverity::Error => "✗ Error".to_string(),
    }
}

/// JSON report structure for API/JSON output
#[derive(Serialize)]
struct JsonReport {
    scan_time: String,
    categories: Vec<CategoryResult>,
    drift_events: Vec<DriftEvent>,
    summary: Summary,
}

#[derive(Serialize)]
struct Summary {
    passed: usize,
    warnings: usize,
    failed: usize,
    drift_count: usize,
    is_healthy: bool,
    exit_code: i32,
}

#[cfg(test)]
mod tests {
    use super::super::{CategoryResult, CategoryStatus, DoctorResult};
    use super::*;

    #[test]
    fn test_format_category_pass() {
        let status = CategoryStatus::Pass;
        let line = format_category_line(CATEGORY_CONFIG, &status);
        assert!(line.contains("✓"));
    }

    #[test]
    fn test_format_category_fail() {
        let status = CategoryStatus::Fail("Missing key".into());
        let line = format_category_line(CATEGORY_CONFIG, &status);
        assert!(line.contains("✗"));
        assert!(line.contains("Missing key"));
    }

    #[test]
    fn test_format_category_warn() {
        let status = CategoryStatus::Warn(vec!["Low rate limit".into()]);
        let line = format_category_line(CATEGORY_CONFIG, &status);
        assert!(line.contains("⚠"));
    }

    #[test]
    fn test_text_report_all_pass() {
        let result = DoctorResult {
            categories: vec![
                CategoryResult::pass(CATEGORY_CONFIG),
                CategoryResult::pass(CATEGORY_AUTH),
                CategoryResult::pass(CATEGORY_BEADS),
                CategoryResult::pass(CATEGORY_PLANS),
                CategoryResult::pass(CATEGORY_REPO),
                CategoryResult::pass(CATEGORY_DRIFT),
            ],
            drift_events: Vec::new(),
            is_healthy: true,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, false);
        let report = generator.generate(&result);

        assert!(report.contains("Rodgers is healthy"));
        assert!(report.contains("✓ OK"));
    }

    #[test]
    fn test_text_report_with_failures() {
        let result = DoctorResult {
            categories: vec![
                CategoryResult::pass(CATEGORY_CONFIG),
                CategoryResult::fail(CATEGORY_AUTH, "Token expired"),
            ],
            drift_events: Vec::new(),
            is_healthy: false,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, false);
        let report = generator.generate(&result);

        assert!(report.contains("✗ FAIL"));
        assert!(report.contains("Token expired"));
    }

    #[test]
    fn test_json_report() {
        let result = DoctorResult {
            categories: vec![CategoryResult::pass(CATEGORY_CONFIG)],
            drift_events: Vec::new(),
            is_healthy: true,
        };

        let generator = ReportGenerator::new(OutputFormat::Json, false);
        let report = generator.generate(&result);

        assert!(report.contains("scan_time"));
        assert!(report.contains("\"is_healthy\": true"));
    }

    #[test]
    fn test_verbose_drift_report_shows_issue_url() {
        let result = DoctorResult {
            categories: vec![CategoryResult::warn(
                CATEGORY_DRIFT,
                vec!["1 drift event found".to_string()],
            )],
            drift_events: vec![DriftEvent {
                event_type: "closed_bead_open_issue".into(),
                description: "Bead b-001 is closed but linked GitHub issue #123 is open".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/123".into()),
                bead_id: Some("b-001".into()),
                severity: DriftSeverity::Error,
            }],
            is_healthy: false,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, true);
        let report = generator.generate(&result);

        // In verbose mode, should show issue URL
        assert!(report.contains("Issue URL: https://github.com/owner/repo/issues/123"));
        // Should show bead ID
        assert!(report.contains("Bead ID: b-001"));
        // Should show the description
        assert!(report.contains("Bead b-001 is closed but linked GitHub issue #123 is open"));
    }

    #[test]
    fn test_non_verbose_drift_report_shows_summary_only() {
        let result = DoctorResult {
            categories: vec![CategoryResult::warn(
                CATEGORY_DRIFT,
                vec!["1 drift event found".to_string()],
            )],
            drift_events: vec![DriftEvent {
                event_type: "closed_bead_open_issue".into(),
                description: "Bead b-001 is closed but linked GitHub issue #123 is open".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/123".into()),
                bead_id: Some("b-001".into()),
                severity: DriftSeverity::Error,
            }],
            is_healthy: false,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, false);
        let report = generator.generate(&result);

        // In non-verbose mode, should NOT show issue URL
        assert!(!report.contains("Issue URL:"));
        // Should NOT show individual drift events
        assert!(!report.contains("Bead ID:"));
        // But should mention drift was detected
        assert!(report.contains("DRIFT DETECTED"));
    }

    #[test]
    fn test_all_drift_types_detailed_in_verbose_mode() {
        let drift_events = vec![
            DriftEvent {
                event_type: "closed_bead_open_issue".into(),
                description: "Bead b-001 is closed but linked GitHub issue #123 is open".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/123".into()),
                bead_id: Some("b-001".into()),
                severity: DriftSeverity::Error,
            },
            DriftEvent {
                event_type: "in_progress_bead_closed_issue".into(),
                description: "Bead b-002 is in-progress but linked GitHub issue #456 is closed"
                    .into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/456".into()),
                bead_id: Some("b-002".into()),
                severity: DriftSeverity::Warning,
            },
            DriftEvent {
                event_type: "orphan_bead".into(),
                description: "Bead b-003 has no linked GitHub issue".into(),
                github_issue_url: None,
                bead_id: Some("b-003".into()),
                severity: DriftSeverity::Warning,
            },
            DriftEvent {
                event_type: "ready_for_work_no_bead".into(),
                description: "Issue #789 has 'ready-for-work' label but no linked bead".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/789".into()),
                bead_id: None,
                severity: DriftSeverity::Warning,
            },
            DriftEvent {
                event_type: "release_proposed_no_milestone".into(),
                description: "Issue #999 is release-proposed but not in a milestone".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/999".into()),
                bead_id: Some("b-999".into()),
                severity: DriftSeverity::Warning,
            },
            DriftEvent {
                event_type: "convention_violation".into(),
                description: "Bead b-100 missing 'Plan: plans/...' reference".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/100".into()),
                bead_id: Some("b-100".into()),
                severity: DriftSeverity::Warning,
            },
        ];

        let result = DoctorResult {
            categories: vec![CategoryResult::warn(
                CATEGORY_DRIFT,
                vec!["6 drift events found".to_string()],
            )],
            drift_events,
            is_healthy: false,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, true);
        let report = generator.generate(&result);

        // Check all drift types are present with full details
        assert!(report.contains("Issue URL: https://github.com/owner/repo/issues/123"));
        assert!(report.contains("Issue URL: https://github.com/owner/repo/issues/456"));
        assert!(report.contains("Bead ID: b-003"));
        assert!(report.contains("Issue URL: https://github.com/owner/repo/issues/789"));
        assert!(report.contains("Issue URL: https://github.com/owner/repo/issues/999"));
        assert!(report.contains("Bead ID: b-100"));

        // Verify linking info is shown
        assert!(report.contains("Link:"));
    }

    #[test]
    fn test_verbose_report_includes_linking_remediation() {
        let result = DoctorResult {
            categories: vec![CategoryResult::warn(
                CATEGORY_DRIFT,
                vec!["5 drift events found".to_string()],
            )],
            drift_events: vec![
                DriftEvent {
                    event_type: "closed_bead_open_issue".into(),
                    description: "Bead b-001 is closed but linked GitHub issue is open".into(),
                    github_issue_url: Some("https://github.com/owner/repo/issues/123".into()),
                    bead_id: Some("b-001".into()),
                    severity: DriftSeverity::Error,
                },
                DriftEvent {
                    event_type: "orphan_bead".into(),
                    description: "Bead b-002 has no GitHub issue link".into(),
                    github_issue_url: None,
                    bead_id: Some("b-002".into()),
                    severity: DriftSeverity::Warning,
                },
            ],
            is_healthy: false,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, true);
        let report = generator.generate(&result);

        // Should show linking remediation info
        assert!(report.contains("Link: Close the bead to match"));
        assert!(report.contains("Link: Link the bead to a GitHub issue"));
        // Should mention --fix option
        assert!(report.contains("doctor --fix"));
    }

    #[test]
    fn test_get_linking_info() {
        use super::DriftEvent;

        // Closed bead / open issue
        let event = DriftEvent {
            event_type: "closed_bead_open_issue".into(),
            description: "Test".into(),
            github_issue_url: Some("https://github.com/owner/repo/issues/1".into()),
            bead_id: Some("b-1".into()),
            severity: DriftSeverity::Error,
        };
        assert!(get_linking_info(&event).contains("Close the bead"));

        // Orphan bead
        let event = DriftEvent {
            event_type: "orphan_bead".into(),
            description: "Test".into(),
            github_issue_url: None,
            bead_id: Some("b-2".into()),
            severity: DriftSeverity::Warning,
        };
        assert!(get_linking_info(&event).contains("Link the bead"));

        // Without proper links
        let event = DriftEvent {
            event_type: "closed_bead_open_issue".into(),
            description: "Test".into(),
            github_issue_url: None,
            bead_id: None,
            severity: DriftSeverity::Error,
        };
        assert!(get_linking_info(&event).is_empty());
    }

    #[test]
    fn test_format_drift_type() {
        assert_eq!(
            format_drift_type("closed_bead_open_issue"),
            "Closed Bead / Open Issue"
        );
        assert_eq!(
            format_drift_type("in_progress_bead_closed_issue"),
            "In-Progress Bead / Closed Issue"
        );
        assert_eq!(format_drift_type("orphan_bead"), "Orphan Bead");
        assert_eq!(
            format_drift_type("ready_for_work_no_bead"),
            "Ready-for-Work Issue / No Bead"
        );
        assert_eq!(
            format_drift_type("release_proposed_no_milestone"),
            "Release-Proposed / No Milestone"
        );
        assert_eq!(
            format_drift_type("convention_violation"),
            "Convention Violation"
        );
    }

    #[test]
    fn test_format_severity() {
        use super::DriftSeverity;
        assert_eq!(format_severity(DriftSeverity::Warning), "⚠ Warning");
        assert_eq!(format_severity(DriftSeverity::Error), "✗ Error");
    }

    #[test]
    fn test_verbose_orphan_bead_shows_no_issue_url() {
        let result = DoctorResult {
            categories: vec![CategoryResult::warn(
                CATEGORY_DRIFT,
                vec!["1 drift event found".to_string()],
            )],
            drift_events: vec![DriftEvent {
                event_type: "orphan_bead".into(),
                description: "Bead b-001 has no GitHub issue link".into(),
                github_issue_url: None,
                bead_id: Some("b-001".into()),
                severity: DriftSeverity::Warning,
            }],
            is_healthy: false,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, true);
        let report = generator.generate(&result);

        // Should show bead ID
        assert!(report.contains("Bead ID: b-001"));
        // Should NOT show Issue URL line (since there's none)
        assert!(!report.contains("Issue URL:"));
    }

    #[test]
    fn test_verbose_ready_for_work_no_bead_shows_no_bead_id() {
        let result = DoctorResult {
            categories: vec![CategoryResult::warn(
                CATEGORY_DRIFT,
                vec!["1 drift event found".to_string()],
            )],
            drift_events: vec![DriftEvent {
                event_type: "ready_for_work_no_bead".into(),
                description: "Issue #123 has no linked bead".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/123".into()),
                bead_id: None,
                severity: DriftSeverity::Warning,
            }],
            is_healthy: false,
        };

        let generator = ReportGenerator::new(OutputFormat::Text, true);
        let report = generator.generate(&result);

        // Should show issue URL
        assert!(report.contains("Issue URL: https://github.com/owner/repo/issues/123"));
        // Should NOT show Bead ID line (since there's no bead)
        assert!(!report.contains("Bead ID:"));
    }
}
