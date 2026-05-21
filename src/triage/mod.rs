//! Triage module - Triage loop processing for Rodgers.
//!
//! Includes:
//! - `classifier` — issue type classification with label heuristics and LLM fallback
//! - `scheduler` — cron interval and webhook event driven triage loop
//! - `triage_loop` — core triage logic (process_issue, process_issues_batch)
//! - `priority` — priority assessment for feature issues
//! - `router` — feature issue routing to feature-bug workflow

pub mod classifier;
pub mod priority;
pub mod router;
pub mod scheduler;
pub mod triage_loop;

pub use classifier::{
    classify_issue, classify_issue as classify, classify_by_labels, pre_check_classification,
    ClassifiedIssue, ClassificationMethod, Confidence, IssueType, PreCheckResult,
    TriageClassification, default_llm_classifier, is_bot_author, validate_classification,
    issue_type_to_workflow, resolve_conflicting_labels,
};

pub use priority::{Priority, PriorityAssessment, assess_priority, llm_assess_priority};
pub use router::{FeatureIssue, RouteResult, route_feature, route_feature_batch};

pub use scheduler::{
    DEFAULT_INTERVAL_MINUTES, RetryPolicy, RunLock, RunMetadata, RunTrigger, SchedulerConfig,
    TriageScheduler, TriagedState, WebhookEvent, run_once,
};

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
