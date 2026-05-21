//! Triage module - Triage loop processing for Rodgers.

pub mod router;
pub mod severity;
pub mod triage_loop;

pub use severity::{
    Priority, Severity, SeverityResult, assess_severity, label_is_severity_label,
    severity_needs_backport, severity_to_backport_priority, severity_to_label,
    severity_to_priority,
};

pub use triage_loop::{
    IssueState, TriageAction, TriageIssue, TriageResult, process_issue, process_issues_batch,
};

/// Label constants for triage operations.
pub const LABEL_BUG: &str = "bug";
pub const LABEL_FEATURE: &str = "feature";
pub const LABEL_QUESTION: &str = "question";
pub const LABEL_READY_FOR_REVIEW: &str = "ready-for-review";
pub const LABEL_NEEDS_INFORMATION: &str = "needs-information";
pub const LABEL_WILL_NOT_DO: &str = "will-not-do";
pub const LABEL_READY_FOR_WORK: &str = "ready-for-work";
