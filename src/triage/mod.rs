//! Triage module - Triage loop processing for Rodgers.

pub mod priority;
pub mod router;
pub mod triage_loop;

pub use priority::{assess_priority, llm_assess_priority, Priority, PriorityAssessment};
pub use router::{route_feature, route_feature_batch, FeatureIssue, RouteResult};

pub use triage_loop::{
    IssueState, LABEL_TRIAGED, TriageAction, TriageIssue, TriageResult, has_triaged_label,
    process_issue, process_issues_batch,
};

/// Label constants for triage operations.
pub const LABEL_BUG: &str = "bug";
pub const LABEL_FEATURE: &str = "feature";
pub const LABEL_READY_FOR_REVIEW: &str = "ready-for-review";
pub const LABEL_NEEDS_INFORMATION: &str = "needs-information";
pub const LABEL_WILL_NOT_DO: &str = "will-not-do";
pub const LABEL_READY_FOR_WORK: &str = "ready-for-work";
