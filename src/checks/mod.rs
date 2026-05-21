//! Init audit checks — reusable check trait and common result types.

mod issue_templates;
mod labels;

pub use issue_templates::IssueTemplatesCheck;
pub use labels::LabelsCheck;

use serde::{Deserialize, Serialize};

/// How severe a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Blocker,
    Warn,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Blocker => "BLOCKER",
            Severity::Warn => "WARN",
            Severity::Info => "INFO",
        }
    }
}

/// Whether this finding can be auto-fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fixability {
    /// Rodgers can fix automatically via the GitHub API.
    Auto,
    /// Requires manual action (or a PR opened for human review).
    Manual,
    /// Not applicable — this is purely informational.
    NotApplicable,
}

impl Fixability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Fixability::Auto => "auto",
            Fixability::Manual => "manual",
            Fixability::NotApplicable => "info",
        }
    }
}

/// A single check result returned by an audit check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub severity: Severity,
    pub description: String,
    pub fixability: Fixability,
    pub fix_instructions: Option<String>,
}

impl CheckResult {
    pub fn blocker(description: impl Into<String>) -> Self {
        Self {
            severity: Severity::Blocker,
            description: description.into(),
            fixability: Fixability::Manual,
            fix_instructions: None,
        }
    }

    pub fn warn(description: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warn,
            description: description.into(),
            fixability: Fixability::Manual,
            fix_instructions: None,
        }
    }

    pub fn info(description: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            description: description.into(),
            fixability: Fixability::NotApplicable,
            fix_instructions: None,
        }
    }
}

/// Trait that all init audit checks must implement.
#[allow(async_fn_in_trait)]
pub trait InitCheck {
    /// Run the check and return the result.
    async fn check(
        &self,
        github: &crate::github::GitHubClient,
        owner: &str,
        repo: &str,
    ) -> crate::error::Result<CheckResult>;

    /// Human-readable name of this check.
    fn name(&self) -> &'static str;
}
