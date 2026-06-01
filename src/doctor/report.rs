//! Report generation for doctor command
//!
//! Formats doctor check results into human-readable or JSON output.

use super::{CategoryResult, CategoryStatus, DoctorResult};
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
                output.push_str("\nDrift events:\n");
                for (i, event) in result.drift_events.iter().enumerate() {
                    output.push_str(&format!("  {}. {}\n", i + 1, event.description));
                    if let Some(ref issue_url) = event.github_issue_url {
                        output.push_str(&format!("     Issue: {}\n", issue_url));
                    }
                    if let Some(ref task_id) = event.task_id {
                        output.push_str(&format!("     Task: {}\n", task_id));
                    }
                }
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

/// JSON report structure for API/JSON output
#[derive(Serialize)]
struct JsonReport {
    scan_time: String,
    categories: Vec<CategoryResult>,
    drift_events: Vec<super::DriftEvent>,
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
    use super::super::{
        CATEGORY_AUTH, CATEGORY_BACKLOG, CATEGORY_CONFIG, CATEGORY_DRIFT, CATEGORY_PLANS,
        CATEGORY_REPO, CategoryResult, CategoryStatus, DoctorResult,
    };
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
                CategoryResult::pass(CATEGORY_BACKLOG),
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
}
