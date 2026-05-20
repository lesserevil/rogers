//! Rodgers Doctor — Health Check Command
//!
//! `rogers doctor` audits a Rodgers installation for configuration problems
//! and state drift. Exit codes:
//! - 0: All checks passed (healthy)
//! - 1: One or more checks failed or drift detected
//! - 2: Invalid arguments or configuration
//! - 3: Authentication failed

pub mod categories;
pub mod drift;
pub mod report;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Category name identifiers
pub const CATEGORY_CONFIG: &str = "config";
pub const CATEGORY_AUTH: &str = "auth";
pub const CATEGORY_BEADS: &str = "beads";
pub const CATEGORY_PLANS: &str = "plans";
pub const CATEGORY_REPO: &str = "repo";
pub const CATEGORY_DRIFT: &str = "drift";

/// All known categories in execution order
pub const ALL_CATEGORIES: &[&str] = &[
    CATEGORY_CONFIG,
    CATEGORY_AUTH,
    CATEGORY_BEADS,
    CATEGORY_PLANS,
    CATEGORY_REPO,
    CATEGORY_DRIFT,
];

/// Categories that should fail-fast (run first, stop on failure)
pub const FAIL_FAST_CATEGORIES: &[&str] = &[CATEGORY_CONFIG, CATEGORY_AUTH];

/// Result status for a single category check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CategoryStatus {
    /// All checks in this category passed
    Pass,
    /// Category passed but with warnings
    Warn(Vec<String>),
    /// Category check failed
    Fail(String),
    /// Category was not run (e.g., filtered by --only)
    Skipped,
}

impl CategoryStatus {
    /// Returns true if this status represents a passing check
    pub fn is_ok(&self) -> bool {
        match self {
            CategoryStatus::Pass | CategoryStatus::Warn(_) | CategoryStatus::Skipped => true,
            CategoryStatus::Fail(_) => false,
        }
    }

    /// Returns true if this status has warnings but no failures
    pub fn has_warning(&self) -> bool {
        matches!(self, CategoryStatus::Warn(_))
    }
}

/// A single category check result with details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryResult {
    /// Category name (e.g., "config", "auth")
    pub name: String,
    /// Check status
    pub status: CategoryStatus,
    /// Detailed messages for this category
    pub messages: Vec<String>,
}

impl CategoryResult {
    /// Create a passing result
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CategoryStatus::Pass,
            messages: Vec::new(),
        }
    }

    /// Create a passing result with messages
    pub fn pass_with_messages(name: impl Into<String>, messages: Vec<String>) -> Self {
        Self {
            name: name.into(),
            status: CategoryStatus::Pass,
            messages,
        }
    }

    /// Create a warning result
    pub fn warn(name: impl Into<String>, warnings: Vec<String>) -> Self {
        Self {
            name: name.into(),
            status: CategoryStatus::Warn(warnings.clone()),
            messages: warnings,
        }
    }

    /// Create a failing result
    pub fn fail(name: impl Into<String>, error: impl Into<String>) -> Self {
        let error_msg = error.into();
        Self {
            name: name.into(),
            status: CategoryStatus::Fail(error_msg.clone()),
            messages: vec![error_msg],
        }
    }

    /// Create a skipped result
    pub fn skipped(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CategoryStatus::Skipped,
            messages: Vec::new(),
        }
    }
}

/// Drift event representing state divergence between GitHub and beads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    /// Drift event type
    pub event_type: String,
    /// Description of the drift
    pub description: String,
    /// GitHub issue URL if applicable
    pub github_issue_url: Option<String>,
    /// Bead ID if applicable
    pub bead_id: Option<String>,
    /// Severity level
    pub severity: DriftSeverity,
}

/// Severity level for drift events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftSeverity {
    /// Warning - drift detected but not critical
    Warning,
    /// Error - drift detected that requires attention
    Error,
}

/// Summary of all doctor check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorResult {
    /// Individual category results
    pub categories: Vec<CategoryResult>,
    /// Drift events detected
    pub drift_events: Vec<DriftEvent>,
    /// All categories passed (including with warnings) and no drift
    pub is_healthy: bool,
}

impl DoctorResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
            drift_events: Vec::new(),
            is_healthy: false,
        }
    }

    /// Check if all required categories passed
    pub fn all_categories_passed(&self) -> bool {
        self.categories
            .iter()
            .filter(|c| c.status != CategoryStatus::Skipped)
            .all(|c| c.status.is_ok())
    }

    /// Check if any categories failed
    pub fn any_category_failed(&self) -> bool {
        self.categories
            .iter()
            .any(|c| matches!(c.status, CategoryStatus::Fail(_)))
    }

    /// Check if drift is present
    pub fn has_drift(&self) -> bool {
        !self.drift_events.is_empty()
    }

    /// Count of categories that passed
    pub fn passed_count(&self) -> usize {
        self.categories
            .iter()
            .filter(|c| c.status == CategoryStatus::Pass)
            .count()
    }

    /// Count of categories that passed with warnings
    pub fn warned_count(&self) -> usize {
        self.categories
            .iter()
            .filter(|c| c.status.has_warning())
            .count()
    }

    /// Count of categories that failed
    pub fn failed_count(&self) -> usize {
        self.categories
            .iter()
            .filter(|c| matches!(c.status, CategoryStatus::Fail(_)))
            .count()
    }

    /// Determine the exit code for this result
    pub fn exit_code(&self) -> i32 {
        if self.any_category_failed() {
            1
        } else if self.has_drift() {
            1
        } else if self.all_categories_passed() && !self.has_drift() {
            0
        } else {
            // Default to 1 - something unexpected happened
            1
        }
    }
}

impl Default for DoctorResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_pass_no_drift_exits_0() {
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

        assert_eq!(result.exit_code(), 0);
        assert!(result.all_categories_passed());
        assert!(!result.has_drift());
    }

    #[test]
    fn test_category_fail_exits_1() {
        let result = DoctorResult {
            categories: vec![
                CategoryResult::pass(CATEGORY_CONFIG),
                CategoryResult::fail(CATEGORY_AUTH, "Token invalid"),
            ],
            drift_events: Vec::new(),
            is_healthy: false,
        };

        assert_eq!(result.exit_code(), 1);
        assert!(result.any_category_failed());
    }

    #[test]
    fn test_drift_detected_exits_1() {
        let result = DoctorResult {
            categories: vec![
                CategoryResult::pass(CATEGORY_CONFIG),
                CategoryResult::pass(CATEGORY_AUTH),
            ],
            drift_events: vec![DriftEvent {
                event_type: "closed_bead_open_issue".into(),
                description: "Bead #b-001 is closed but linked GitHub issue #123 is open".into(),
                github_issue_url: Some("https://github.com/owner/repo/issues/123".into()),
                bead_id: Some("b-001".into()),
                severity: DriftSeverity::Warning,
            }],
            is_healthy: false,
        };

        assert_eq!(result.exit_code(), 1);
        assert!(result.has_drift());
    }

    #[test]
    fn test_warnings_still_exits_0() {
        let result = DoctorResult {
            categories: vec![CategoryResult::warn(
                CATEGORY_CONFIG,
                vec!["scheduler.interval_minutes is at minimum value".into()],
            )],
            drift_events: Vec::new(),
            is_healthy: true,
        };

        assert_eq!(result.exit_code(), 0);
        assert!(result.all_categories_passed());
    }

    #[test]
    fn test_skipped_categories_ignored() {
        let result = DoctorResult {
            categories: vec![
                CategoryResult::pass(CATEGORY_CONFIG),
                CategoryResult::pass(CATEGORY_AUTH),
                CategoryResult::skipped(CATEGORY_BEADS),
                CategoryResult::skipped(CATEGORY_PLANS),
                CategoryResult::skipped(CATEGORY_REPO),
                CategoryResult::skipped(CATEGORY_DRIFT),
            ],
            drift_events: Vec::new(),
            is_healthy: true,
        };

        assert_eq!(result.exit_code(), 0);
        assert!(result.all_categories_passed());
    }
}
