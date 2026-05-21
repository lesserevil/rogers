//! Severity assessment for bug issues.
//!
//! This module implements severity classification as defined in
//! plans/triage-workflow-plan.md §Top-Level Classification → plans/feature-bug-plan.md.
//!
//! Bug issues are classified into severity levels based on keyword detection
//! and LLM analysis:
//!
//! - **critical**: crash, data loss, security (CVE, GHSA, security label)
//! - **high**: broken feature, major functionality impaired
//! - **medium**: minor issue, degraded functionality
//! - **low**: cosmetic issue, minor UI problems
//!
//! Severity maps to priority:
//! - critical → P1, high → P2, medium → P3, low → P4
//!
//! Plan: plans/feature-bug-plan.md

use serde::{Deserialize, Serialize};

/// Bug severity levels.
///
/// Critical bugs (data loss, security, crashes) need immediate attention.
/// Severity affects backport priority (critical/high = priority 1 for backports).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Critical: crash, data loss, security vulnerabilities
    Critical,
    /// High: broken feature, major functionality impaired
    High,
    /// Medium: minor issue, degraded functionality
    Medium,
    /// Low: cosmetic issue, minor UI problems
    Low,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "critical"),
            Severity::High => write!(f, "high"),
            Severity::Medium => write!(f, "medium"),
            Severity::Low => write!(f, "low"),
        }
    }
}

/// Priority levels derived from severity.
///
/// Mapping:
/// - critical → P1, high → P2, medium → P3, low → P4
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    /// P1: Critical priority - immediate attention required
    P1,
    /// P2: High priority - soon
    P2,
    /// P3: Medium priority - when available
    P3,
    /// P4: Low priority - backlog
    P4,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::P1 => write!(f, "P1"),
            Priority::P2 => write!(f, "P2"),
            Priority::P3 => write!(f, "P3"),
            Priority::P4 => write!(f, "P4"),
        }
    }
}

/// Result of a severity assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityResult {
    /// The assessed severity level
    pub severity: Severity,
    /// The mapped priority
    pub priority: Priority,
    /// Keywords that triggered the assessment
    pub keywords_matched: Vec<String>,
    /// Whether severity was explicitly set by a human
    pub human_set: bool,
    /// Reason for the assessment
    pub reason: String,
}

// Label constant for rodgers:bug label
pub const LABEL_RODGERS_BUG: &str = "rodgers:bug";

// Label constant for security label (always critical)
pub const LABEL_SECURITY: &str = "security";

// Keywords that indicate critical severity
const CRITICAL_KEYWORDS: &[&str] = &[
    "crash",
    "crashes",
    "crashed",
    "crashing",
    "data loss",
    "data-loss",
    "data_loss",
    "corrupt",
    "corruption",
    "cve",
    "ghsa",
    "security",
    "vulnerability",
    "vulnerabilities",
    "exploit",
    "exploitable",
    "injection",
    "rce",
    "privilege escalation",
    "denial of service",
    "dos",
    "ddos",
];

// Keywords that indicate high severity
const HIGH_KEYWORDS: &[&str] = &[
    "broken",
    "not working",
    "not work",
    "broken feature",
    "major",
    "major functionality",
    "critical issue",
    "blocking",
    "blocker",
    "regression",
    "regressed",
    "regresses",
    "stops working",
    "stop working",
    "unusable",
];

// Keywords that indicate medium severity
const MEDIUM_KEYWORDS: &[&str] = &[
    "minor",
    "minor issue",
    "degraded",
    "slight",
    "small",
    "sluggish",
    "slow",
    "performance",
    "latency",
    "timeout",
    "timing",
    "warning",
    "warn",
    "inconsistent",
    "incorrect",
];

// Keywords that indicate low severity
const LOW_KEYWORDS: &[&str] = &[
    "cosmetic",
    "cosmetics",
    "styling",
    "style",
    "typo",
    "typographical",
    "whitespace",
    "formatting",
    "format",
    "align",
    "alignment",
    "color",
    "font",
    "visual",
    "aesthetic",
    "nits",
];

/// Assess severity from issue body content using keyword detection.
///
/// This performs a keyword-based severity assessment by scanning the
/// issue title and body for severity-indicating keywords. Keywords are
/// checked in priority order (critical > high > medium > low).
///
/// The first keyword match determines the severity level.
///
/// Human-set severity is detected by checking for an existing
/// `severity: critical/high/medium/low` label.
pub fn assess_severity(title: &str, body: &str, labels: &[String]) -> SeverityResult {
    let combined = format!("{} {}", title, body).to_lowercase();

    // Check if severity is explicitly set by a human via label
    let human_set = labels.iter().any(|l| {
        l == "severity: critical"
            || l == "severity:high"
            || l == "severity: high"
            || l == "severity: medium"
            || l == "severity:medium"
            || l == "severity: low"
            || l == "severity:low"
    });

    // If human has set severity, respect it
    if human_set {
        let matched_severity = if labels
            .iter()
            .any(|l| l == "severity: critical" || l == "severity:high" || l == "severity: high")
        {
            Severity::Critical
        } else if labels
            .iter()
            .any(|l| l == "severity: medium" || l == "severity:medium")
        {
            Severity::Medium
        } else if labels
            .iter()
            .any(|l| l == "severity: low" || l == "severity:low")
        {
            Severity::Low
        } else {
            Severity::High
        };

        let priority = severity_to_priority(&matched_severity);

        return SeverityResult {
            severity: matched_severity,
            priority,
            keywords_matched: Vec::new(),
            human_set: true,
            reason: "Human-set severity label respected".to_string(),
        };
    }

    // Check for security label - always critical regardless of keywords
    if labels.iter().any(|l| l == LABEL_SECURITY) {
        return SeverityResult {
            severity: Severity::Critical,
            priority: severity_to_priority(&Severity::Critical),
            keywords_matched: vec!["security label".to_string()],
            human_set: false,
            reason: "Security label always maps to critical severity".to_string(),
        };
    }

    // Check for CVE or GHSA references - always critical
    let is_cve_or_ghsa =
        combined.contains("cve-") || combined.contains("ghsa-") || combined.contains("cve.org");
    if is_cve_or_ghsa {
        return SeverityResult {
            severity: Severity::Critical,
            priority: severity_to_priority(&Severity::Critical),
            keywords_matched: vec!["CVE/GHSA reference".to_string()],
            human_set: false,
            reason: "CVE/GHSA reference always maps to critical severity".to_string(),
        };
    }

    // Keyword-based assessment (critical first)
    for keyword in CRITICAL_KEYWORDS {
        if combined.contains(keyword) {
            let severity = Severity::Critical;
            return SeverityResult {
                severity: severity.clone(),
                priority: severity_to_priority(&severity),
                keywords_matched: vec![keyword.to_string()],
                human_set: false,
                reason: format!("Critical keyword detected: \"{}\"", keyword),
            };
        }
    }

    // High severity keywords
    for keyword in HIGH_KEYWORDS {
        if combined.contains(keyword) {
            let severity = Severity::High;
            return SeverityResult {
                severity: severity.clone(),
                priority: severity_to_priority(&severity),
                keywords_matched: vec![keyword.to_string()],
                human_set: false,
                reason: format!("High severity keyword detected: \"{}\"", keyword),
            };
        }
    }

    // Medium severity keywords
    for keyword in MEDIUM_KEYWORDS {
        if combined.contains(keyword) {
            let severity = Severity::Medium;
            return SeverityResult {
                severity: severity.clone(),
                priority: severity_to_priority(&severity),
                keywords_matched: vec![keyword.to_string()],
                human_set: false,
                reason: format!("Medium severity keyword detected: \"{}\"", keyword),
            };
        }
    }

    // Low severity keywords
    for keyword in LOW_KEYWORDS {
        if combined.contains(keyword) {
            let severity = Severity::Low;
            return SeverityResult {
                severity: severity.clone(),
                priority: severity_to_priority(&severity),
                keywords_matched: vec![keyword.to_string()],
                human_set: false,
                reason: format!("Low severity keyword detected: \"{}\"", keyword),
            };
        }
    }

    // No keywords matched - default to medium (conservative assumption)
    SeverityResult {
        severity: Severity::Medium,
        priority: severity_to_priority(&Severity::Medium),
        keywords_matched: Vec::new(),
        human_set: false,
        reason: "No severity keywords detected; defaulting to medium (conservative)".to_string(),
    }
}

/// Map severity to priority.
///
/// Mapping:
/// - critical → P1, high → P2, medium → P3, low → P4
pub fn severity_to_priority(severity: &Severity) -> Priority {
    match severity {
        Severity::Critical => Priority::P1,
        Severity::High => Priority::P2,
        Severity::Medium => Priority::P3,
        Severity::Low => Priority::P4,
    }
}

/// Map severity to backport priority.
///
/// Critical and high severity bugs get priority 1 for backports.
/// Medium and low severity get priority 2 for backports.
pub fn severity_to_backport_priority(severity: &Severity) -> u8 {
    match severity {
        Severity::Critical | Severity::High => 1,
        Severity::Medium | Severity::Low => 2,
    }
}

/// Check if a severity requires backport.
///
/// Critical and high severity bugs should be backported.
pub fn severity_needs_backport(severity: &Severity) -> bool {
    matches!(severity, Severity::Critical | Severity::High)
}

/// Format severity for GitHub label format.
pub fn severity_to_label(severity: &Severity) -> String {
    format!("severity: {}", severity)
}

/// Check if the given label indicates a specific severity.
pub fn label_is_severity_label(label: &str) -> bool {
    label.starts_with("severity: ") || label.starts_with("severity:")
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // CRITICAL KEYWORD TESTS
    // =============================================================================

    #[test]
    fn test_crash_keyword_maps_to_critical() {
        let result = assess_severity(
            "App crashes on startup",
            "The application crashes immediately when launched",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
        assert_eq!(result.priority, Priority::P1);
        assert!(result.keywords_matched.contains(&"crash".to_string()));
    }

    #[test]
    fn test_data_loss_keyword_maps_to_critical() {
        let result = assess_severity(
            "Data loss when saving",
            "User data is lost when the application saves",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
        assert_eq!(result.priority, Priority::P1);
    }

    #[test]
    fn test_security_keyword_maps_to_critical() {
        let result = assess_severity(
            "Security vulnerability in auth",
            "Found a security vulnerability in the authentication module",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
        assert_eq!(result.priority, Priority::P1);
    }

    #[test]
    fn test_security_label_always_critical() {
        // Security issues (CVE, GHSA, security label) always critical
        let result = assess_severity(
            "Minor UI change needed",
            "This is a small UI change",
            &[LABEL_SECURITY.to_string()],
        );
        assert_eq!(result.severity, Severity::Critical);
        assert_eq!(result.priority, Priority::P1);
        assert_eq!(result.keywords_matched, vec!["security label".to_string()]);
    }

    #[test]
    fn test_cve_reference_always_critical() {
        let result = assess_severity(
            "Fix CVE-2024-1234",
            "This addresses CVE-2024-1234 vulnerability",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
        assert_eq!(result.priority, Priority::P1);
        assert!(
            result
                .keywords_matched
                .contains(&"CVE/GHSA reference".to_string())
        );
    }

    #[test]
    fn test_ghsa_reference_always_critical() {
        let result = assess_severity(
            "Address GHSA-xxxx-xxxx",
            "This addresses GHSA-xxxx-xxxx",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
        assert_eq!(result.priority, Priority::P1);
    }

    // =============================================================================
    // HIGH KEYWORD TESTS
    // =============================================================================

    #[test]
    fn test_broken_keyword_maps_to_high() {
        let result = assess_severity(
            "Broken feature: login fails",
            "The login feature is broken after the last update",
            &[],
        );
        assert_eq!(result.severity, Severity::High);
        assert_eq!(result.priority, Priority::P2);
    }

    #[test]
    fn test_regression_keyword_maps_to_high() {
        let result = assess_severity(
            "Regression: export stopped working",
            "Export functionality regressed in version 2.0",
            &[],
        );
        assert_eq!(result.severity, Severity::High);
        assert_eq!(result.priority, Priority::P2);
    }

    #[test]
    fn test_blocking_keyword_maps_to_high() {
        let result = assess_severity(
            "Blocking issue in CI",
            "CI pipeline is blocking due to test failures",
            &[],
        );
        assert_eq!(result.severity, Severity::High);
        assert_eq!(result.priority, Priority::P2);
    }

    // =============================================================================
    // MEDIUM KEYWORD TESTS
    // =============================================================================

    #[test]
    fn test_minor_keyword_maps_to_medium() {
        let result = assess_severity(
            "Minor issue with form",
            "There is a minor issue with form validation",
            &[],
        );
        assert_eq!(result.severity, Severity::Medium);
        assert_eq!(result.priority, Priority::P3);
    }

    #[test]
    fn test_performance_keyword_maps_to_medium() {
        let result = assess_severity(
            "Performance issue",
            "Loading time is slightly degraded",
            &[],
        );
        assert_eq!(result.severity, Severity::Medium);
        assert_eq!(result.priority, Priority::P3);
    }

    #[test]
    fn test_timeout_keyword_maps_to_medium() {
        let result = assess_severity(
            "Timeout on large requests",
            "Large requests timeout after 30 seconds",
            &[],
        );
        assert_eq!(result.severity, Severity::Medium);
        assert_eq!(result.priority, Priority::P3);
    }

    // =============================================================================
    // LOW KEYWORD TESTS
    // =============================================================================

    #[test]
    fn test_cosmetic_keyword_maps_to_low() {
        let result = assess_severity(
            "Cosmetic fix needed",
            "Button alignment is off by a few pixels",
            &[],
        );
        assert_eq!(result.severity, Severity::Low);
        assert_eq!(result.priority, Priority::P4);
    }

    #[test]
    fn test_typo_keyword_maps_to_low() {
        let result = assess_severity(
            "Typo in readme",
            "There is a typographical error in the readme file",
            &[],
        );
        assert_eq!(result.severity, Severity::Low);
        assert_eq!(result.priority, Priority::P4);
    }

    #[test]
    fn test_font_keyword_maps_to_low() {
        let result = assess_severity(
            "Font looks wrong",
            "The font size is different in dark mode",
            &[],
        );
        assert_eq!(result.severity, Severity::Low);
        assert_eq!(result.priority, Priority::P4);
    }

    // =============================================================================
    // PRIORITY MAPPING TESTS
    // =============================================================================

    #[test]
    fn test_severity_to_priority_mapping() {
        assert_eq!(severity_to_priority(&Severity::Critical), Priority::P1);
        assert_eq!(severity_to_priority(&Severity::High), Priority::P2);
        assert_eq!(severity_to_priority(&Severity::Medium), Priority::P3);
        assert_eq!(severity_to_priority(&Severity::Low), Priority::P4);
    }

    // =============================================================================
    // HUMAN-SET SEVERITY TESTS
    // =============================================================================

    #[test]
    fn test_human_set_severity_respected() {
        // Human has set severity: low, must not be overridden by keywords
        let result = assess_severity(
            "Crash in non-critical path",
            "The app crashes but only when clicking a rarely-used button",
            &["severity: low".to_string()],
        );
        assert_eq!(result.severity, Severity::Low);
        assert_eq!(result.priority, Priority::P4);
        assert!(result.human_set);
        assert_eq!(result.keywords_matched.len(), 0);
    }

    #[test]
    fn test_human_set_critical_severity_respected() {
        let result = assess_severity(
            "Minor visual issue",
            "The text is slightly misaligned",
            &["severity: critical".to_string()],
        );
        assert_eq!(result.severity, Severity::Critical);
        assert_eq!(result.priority, Priority::P1);
        assert!(result.human_set);
    }

    #[test]
    fn test_human_set_medium_severity_respected() {
        let result = assess_severity(
            "Minor UI change",
            "Some text could be clearer",
            &["severity: medium".to_string()],
        );
        assert_eq!(result.severity, Severity::Medium);
        assert_eq!(result.priority, Priority::P3);
        assert!(result.human_set);
    }

    // =============================================================================
    // DEFAULT SEVERITY TESTS
    // =============================================================================

    #[test]
    fn test_no_keywords_defaults_to_medium() {
        // No keywords matched → default to medium (conservative)
        let result = assess_severity(
            "Feature suggestion",
            "It would be nice if the app could support more themes",
            &[],
        );
        assert_eq!(result.severity, Severity::Medium);
        assert_eq!(result.priority, Priority::P3);
        assert_eq!(result.keywords_matched.len(), 0);
        assert!(result.reason.contains("defaulting to medium"));
    }

    // =============================================================================
    // CRITICAL KEYWORD VARIANTS
    // =============================================================================

    #[test]
    fn test_crashes_variant_maps_to_critical() {
        let result = assess_severity("App crashes", "It crashes all the time", &[]);
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_crashed_variant_maps_to_critical() {
        let result = assess_severity("App crashed", "The app crashed on launch", &[]);
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_corruption_keyword_maps_to_critical() {
        let result = assess_severity(
            "Database corruption",
            "Data corruption detected in user database",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_vulnerability_keyword_maps_to_critical() {
        let result = assess_severity(
            "SQL injection vulnerability",
            "Found SQL injection in search endpoint",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_exploit_keyword_maps_to_critical() {
        let result = assess_severity(
            "Remote exploit",
            "RCE vulnerability allows arbitrary code execution",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
    }

    // =============================================================================
    // HIGH KEYWORD VARIANTS
    // =============================================================================

    #[test]
    fn test_not_working_keyword_maps_to_high() {
        let result = assess_severity(
            "API not working",
            "The API endpoint returns 500 errors",
            &[],
        );
        assert_eq!(result.severity, Severity::High);
    }

    #[test]
    fn test_broken_feature_keyword_maps_to_high() {
        let result = assess_severity(
            "Broken feature: export",
            "Export feature is completely broken",
            &[],
        );
        assert_eq!(result.severity, Severity::High);
    }

    #[test]
    fn test_unusable_keyword_maps_to_high() {
        let result = assess_severity(
            "Search unusable",
            "Search returns no results for valid queries",
            &[],
        );
        assert_eq!(result.severity, Severity::High);
    }

    // =============================================================================
    // MEDIUM KEYWORD VARIANTS
    // =============================================================================

    #[test]
    fn test_inconsistent_keyword_maps_to_medium() {
        let result = assess_severity(
            "Inconsistent behavior",
            "The UI behaves inconsistently across pages",
            &[],
        );
        assert_eq!(result.severity, Severity::Medium);
    }

    #[test]
    fn test_warning_keyword_maps_to_medium() {
        let result = assess_severity(
            "Warning messages",
            "App shows spurious warning messages on startup",
            &[],
        );
        assert_eq!(result.severity, Severity::Medium);
    }

    // =============================================================================
    // LOW KEYWORD VARIANTS
    // =============================================================================

    #[test]
    fn test_whitespace_keyword_maps_to_low() {
        let result = assess_severity(
            "Whitespace issue",
            "Extra whitespace between paragraphs",
            &[],
        );
        assert_eq!(result.severity, Severity::Low);
    }

    #[test]
    fn test_alignment_keyword_maps_to_low() {
        let result = assess_severity(
            "Alignment issue",
            "Button not properly aligned in sidebar",
            &[],
        );
        assert_eq!(result.severity, Severity::Low);
    }

    #[test]
    fn test_color_keyword_maps_to_low() {
        let result = assess_severity("Wrong color theme", "Primary button has wrong color", &[]);
        assert_eq!(result.severity, Severity::Low);
    }

    // =============================================================================
    // BACKPORT TESTS
    // =============================================================================

    #[test]
    fn test_critical_needs_backport() {
        assert!(severity_needs_backport(&Severity::Critical));
    }

    #[test]
    fn test_high_needs_backport() {
        assert!(severity_needs_backport(&Severity::High));
    }

    #[test]
    fn test_medium_does_not_need_backport() {
        assert!(!severity_needs_backport(&Severity::Medium));
    }

    #[test]
    fn test_low_does_not_need_backport() {
        assert!(!severity_needs_backport(&Severity::Low));
    }

    #[test]
    fn test_backport_priority_critical() {
        assert_eq!(severity_to_backport_priority(&Severity::Critical), 1);
    }

    #[test]
    fn test_backport_priority_high() {
        assert_eq!(severity_to_backport_priority(&Severity::High), 1);
    }

    #[test]
    fn test_backport_priority_medium() {
        assert_eq!(severity_to_backport_priority(&Severity::Medium), 2);
    }

    #[test]
    fn test_backport_priority_low() {
        assert_eq!(severity_to_backport_priority(&Severity::Low), 2);
    }

    // =============================================================================
    // DISPLAY TESTS
    // =============================================================================

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", Severity::Critical), "critical");
        assert_eq!(format!("{}", Severity::High), "high");
        assert_eq!(format!("{}", Severity::Medium), "medium");
        assert_eq!(format!("{}", Severity::Low), "low");
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(format!("{}", Priority::P1), "P1");
        assert_eq!(format!("{}", Priority::P2), "P2");
        assert_eq!(format!("{}", Priority::P3), "P3");
        assert_eq!(format!("{}", Priority::P4), "P4");
    }

    #[test]
    fn test_severity_to_label() {
        assert_eq!(severity_to_label(&Severity::Critical), "severity: critical");
        assert_eq!(severity_to_label(&Severity::High), "severity: high");
        assert_eq!(severity_to_label(&Severity::Medium), "severity: medium");
        assert_eq!(severity_to_label(&Severity::Low), "severity: low");
    }

    #[test]
    fn test_label_is_severity_label() {
        assert!(label_is_severity_label("severity: critical"));
        assert!(label_is_severity_label("severity:high"));
        assert!(!label_is_severity_label("bug"));
        assert!(!label_is_severity_label("rodgers:bug"));
    }

    // =============================================================================
    // SERIALIZATION TESTS
    // =============================================================================

    #[test]
    fn test_severity_serialization() {
        let json = serde_json::to_string(&Severity::Critical).unwrap();
        assert!(json.contains("Critical"));

        let parsed: Severity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Severity::Critical);
    }

    #[test]
    fn test_priority_serialization() {
        let json = serde_json::to_string(&Priority::P2).unwrap();
        assert!(json.contains("P2"));

        let parsed: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Priority::P2);
    }

    #[test]
    fn test_severity_result_serialization() {
        let result = SeverityResult {
            severity: Severity::Critical,
            priority: Priority::P1,
            keywords_matched: vec!["crash".to_string()],
            human_set: false,
            reason: "Test".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SeverityResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.severity, Severity::Critical);
        assert_eq!(parsed.priority, Priority::P1);
        assert_eq!(parsed.keywords_matched, vec!["crash".to_string()]);
        assert!(!parsed.human_set);
    }

    // =============================================================================
    // EDGE CASE TESTS
    // =============================================================================

    #[test]
    fn test_multiple_keywords_uses_highest_severity() {
        // If both crash and typo are present, critical should win
        let result = assess_severity(
            "Cosmetic fix for typo",
            "There is a minor typo in the button, but also a crash on submit",
            &[],
        );
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_empty_body_defaults_to_medium() {
        let result = assess_severity("Issue title", "", &[]);
        assert_eq!(result.severity, Severity::Medium);
        assert_eq!(result.priority, Priority::P3);
    }

    #[test]
    fn test_case_insensitive_keywords() {
        let result = assess_severity("CRASH in production", "APP CRASHES when loading", &[]);
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_security_label_overrides_medium_keywords() {
        // Even with "minor" keyword, security label forces critical
        let result = assess_severity(
            "Minor security issue",
            "A slight minor vulnerability exists",
            &[LABEL_SECURITY.to_string()],
        );
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn test_human_set_overrides_all_keywords() {
        // Even with crash keyword, human-set severity is respected
        let result = assess_severity(
            "App crashes sometimes",
            "The app crashes on startup in rare conditions",
            &["severity: low".to_string(), LABEL_RODGERS_BUG.to_string()],
        );
        assert_eq!(result.severity, Severity::Low);
        assert_eq!(result.priority, Priority::P4);
        assert!(result.human_set);
    }
}
