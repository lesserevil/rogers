//! Triage module - Triage loop processing for Rodgers.

pub mod router;
pub mod triage_loop;

pub use triage_loop::{
    IssueState, TriageAction, TriageIssue, TriageResult, process_issue, process_issues_batch,
};

/// Label constants for triage operations.
pub const LABEL_BUG: &str = "bug";
pub const LABEL_FEATURE: &str = "feature";
pub const LABEL_DOCS: &str = "docs";
pub const LABEL_READY_FOR_REVIEW: &str = "ready-for-review";
pub const LABEL_NEEDS_INFORMATION: &str = "needs-information";
pub const LABEL_WILL_NOT_DO: &str = "will-not-do";
pub const LABEL_READY_FOR_WORK: &str = "ready-for-work";

/// Label marking an issue as docs/template work routed to issue-templates workflow.
pub const LABEL_RODGERS_DOCS: &str = "rodgers:docs";
