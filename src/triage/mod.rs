//! Triage module.
//!
//! Provides the triage engine for classifying and processing GitHub issues.
//!
//! Includes:
//! - `classifier` — issue type classification with label heuristics and LLM fallback
//! - `scheduler` — cron interval and webhook event driven triage loop
//! - `triage_loop` — core triage logic (process_issue, process_issues_batch)
//! - `priority` — priority assessment for feature issues
//! - `router` — feature issue routing to feature-bug workflow

pub mod classifier;
pub mod engine;
pub mod priority;
pub mod router;
pub mod scheduler;
pub mod state_machine;
pub mod triage_loop;

pub use classifier::{
    classify_by_labels, classify_issue, classify_issue as classify, default_llm_classifier,
    is_bot_author, issue_type_to_workflow, pre_check_classification, resolve_conflicting_labels,
    validate_classification, ClassificationMethod, ClassifiedIssue, Classifier, Confidence,
    IssueType, LlmClassificationResult, PreCheckResult, TriageClassification,
};
pub use engine::TriageEngine;
pub use priority::{assess_priority, llm_assess_priority, Priority, PriorityAssessment};
pub use router::{route_feature, route_feature_batch, FeatureIssue, RouteResult};

pub use scheduler::{
    run_once, RetryPolicy, RunLock, RunMetadata, RunTrigger, SchedulerConfig, TriageScheduler,
    TriagedState, WebhookEvent, DEFAULT_INTERVAL_MINUTES,
};
pub use state_machine::{TransitionError, TriageState, TriageStateMachine};
pub use triage_loop::{
    check_backport_triggers, has_triaged_label, process_issue, process_issues_batch,
    BackportTriggerInfo, IssueState, TriageAction, TriageIssue, TriageResult, LABEL_BACKPORT_ME,
    LABEL_BUG, LABEL_FEATURE, LABEL_NEEDS_INFORMATION, LABEL_READY_FOR_REVIEW,
    LABEL_READY_FOR_WORK, LABEL_SECURITY, LABEL_TRIAGED, LABEL_WILL_NOT_DO,
};

/// Question label constant (not yet in triage_loop).
pub const LABEL_QUESTION: &str = "question";
