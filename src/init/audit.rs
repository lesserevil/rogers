//! Rodgers init — Project Readiness Audit
//!
//! This module performs initial setup audits for repositories managed by Rodgers.
//! It checks for required labels, issue templates, and other prerequisites.

use crate::templates::discover_templates;
use std::path::Path;

/// Severity level for audit findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Blocker — must be fixed before Rodgers can operate.
    Blocker,
    /// Warning — review recommended.
    Warn,
    /// Info — no action needed.
    Info,
}

/// A single audit finding.
#[derive(Debug, Clone)]
pub struct AuditFinding {
    /// Severity level of the finding.
    pub severity: Severity,
    /// Human-readable description.
    pub description: String,
    /// Whether this finding can be automatically fixed.
    pub fixable: bool,
    /// Instructions for fixing (if applicable).
    pub fix_instructions: Option<String>,
}

/// Result of running the init audit.
#[derive(Debug, Clone, Default)]
pub struct InitAuditResult {
    /// All audit findings.
    pub findings: Vec<AuditFinding>,
    /// Template discovery result.
    pub template_result: crate::templates::DiscoveryResult,
}

impl InitAuditResult {
    /// Returns true if there are any blocker findings.
    pub fn has_blockers(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.severity == Severity::Blocker)
    }

    /// Returns blocker findings only.
    pub fn blockers(&self) -> Vec<&AuditFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Blocker)
            .collect()
    }

    /// Returns warning findings only.
    pub fn warnings(&self) -> Vec<&AuditFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warn)
            .collect()
    }

    /// Returns info findings only.
    pub fn infos(&self) -> Vec<&AuditFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .collect()
    }
}

/// Runs the init audit for the given repository root.
///
/// This performs all readiness checks including template discovery
/// and produces a structured audit result.
pub fn run_init_audit(repo_root: &Path) -> InitAuditResult {
    let mut result = InitAuditResult::default();

    // Run template discovery
    let template_result = discover_templates(repo_root);
    result.template_result = template_result.clone();

    // Generate audit findings from template discovery
    let template_findings = generate_template_findings(&template_result);
    result.findings.extend(template_findings);

    result
}

/// Generates audit findings from template discovery results.
fn generate_template_findings(
    template_result: &crate::templates::DiscoveryResult,
) -> Vec<AuditFinding> {
    let mut findings = Vec::new();

    if !template_result.directory_exists {
        findings.push(AuditFinding {
            severity: Severity::Blocker,
            description: "Issue templates directory (.github/ISSUE_TEMPLATE/) not found".to_string(),
            fixable: false,
            fix_instructions: Some(
                "Create a .github/ISSUE_TEMPLATE/ directory in your repository with issue templates. \
                 Run 'rogers init --fix' to generate suggested templates.".to_string(),
            ),
        });
        return findings;
    }

    let missing = template_result.missing_templates();

    if missing.is_empty() {
        let found_names: Vec<_> = template_result
            .found_templates()
            .iter()
            .map(|s| s.replace("_", " "))
            .collect();
        findings.push(AuditFinding {
            severity: Severity::Info,
            description: format!("Issue templates found: {}", found_names.join(", ")),
            fixable: false,
            fix_instructions: None,
        });
    } else {
        let missing_display: Vec<_> = missing.iter().map(|s| s.replace("_", " ")).collect();
        findings.push(AuditFinding {
            severity: Severity::Blocker,
            description: format!(
                "Missing issue templates: {}. Rodgers requires these templates for proper triage.",
                missing_display.join(", ")
            ),
            fixable: false,
            fix_instructions: Some(
                "Create the missing template files in .github/ISSUE_TEMPLATE/. \
                 Run 'rogers init --fix' to generate suggested templates."
                    .to_string(),
            ),
        });
    }

    // Add warnings for any discovery issues
    for warning in &template_result.warnings {
        findings.push(AuditFinding {
            severity: Severity::Warn,
            description: format!("Template discovery warning: {}", warning),
            fixable: false,
            fix_instructions: None,
        });
    }

    findings
}

/// Formats the audit result as a human-readable string.
pub fn format_audit_result(result: &InitAuditResult, repo_name: &str) -> String {
    let mut output = "=== Rodgers Project Readiness Audit ===\n".to_string();
    output.push_str(&format!("Repository: {}\n\n", repo_name));

    // Group findings by severity
    for finding in &result.blockers() {
        output.push_str(&format!(
            "[BLOCKER] {} {}\n",
            if finding.fixable { "[FIXABLE]" } else { "" },
            finding.description
        ));
        if let Some(ref instructions) = finding.fix_instructions {
            output.push_str(&format!("         Fix: {}\n", instructions));
        }
    }

    for finding in result.warnings() {
        output.push_str(&format!("[WARN   ] {}\n", finding.description));
    }

    for finding in result.infos() {
        output.push_str(&format!("[INFO   ] {}\n", finding.description));
    }

    let blocker_count = result.blockers().len();
    let warning_count = result.warnings().len();
    let info_count = result.infos().len();

    output.push_str(&format!(
        "\n{} checks performed\n  {} blocker(s) — Rodgers cannot safely operate\n  {} warning(s) — review recommended\n  {} info(s) — no action needed\n",
        result.findings.len(),
        blocker_count,
        warning_count,
        info_count
    ));

    if result.has_blockers() {
        output.push_str("\nRun 'rogers init --fix' to apply available automated fixes.\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_audit_blocks_missing_templates() {
        let temp = TempDir::new().unwrap();
        // Don't create any templates

        let result = run_init_audit(temp.path());

        assert!(result.has_blockers());
        assert!(!result.blockers().is_empty());
    }

    #[test]
    fn test_audit_reports_found_templates() {
        use std::fs;
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".github").join("ISSUE_TEMPLATE");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("bug_report.md"), "# Bug Report\n").unwrap();
        fs::write(dir.join("feature_request.md"), "# Feature Request\n").unwrap();
        fs::write(dir.join("question.md"), "# Question\n").unwrap();

        let result = run_init_audit(temp.path());

        assert!(!result.has_blockers());
        assert!(!result.infos().is_empty());
    }

    #[test]
    fn test_audit_reports_individual_missing() {
        use std::fs;
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".github").join("ISSUE_TEMPLATE");
        fs::create_dir_all(&dir).unwrap();
        // Only create bug_report
        fs::write(dir.join("bug_report.md"), "# Bug Report\n").unwrap();

        let result = run_init_audit(temp.path());

        let blocker_desc = result.blockers()[0].description.clone();
        assert!(blocker_desc.contains("feature_request") || blocker_desc.contains("question"));
    }

    #[test]
    fn test_format_audit_result() {
        let temp = TempDir::new().unwrap();
        let result = run_init_audit(temp.path());

        let formatted = format_audit_result(&result, "owner/repo");
        assert!(formatted.contains("Rodgers Project Readiness Audit"));
        assert!(formatted.contains("owner/repo"));
        assert!(formatted.contains("BLOCKER"));
    }

    #[test]
    fn test_audit_result_counts() {
        use std::fs;
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join(".github").join("ISSUE_TEMPLATE");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("bug_report.md"), "# Bug Report\n").unwrap();
        fs::write(dir.join("feature_request.md"), "# Feature Request\n").unwrap();
        fs::write(dir.join("question.md"), "# Question\n").unwrap();

        let result = run_init_audit(temp.path());

        assert_eq!(result.blockers().len(), 0);
        // Should have 1 info finding for all templates found
        assert_eq!(result.infos().len(), 1);
    }
}
