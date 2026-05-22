//! Feature and bug analysis module.
//!
//! This module provides epic detection and breakdown analysis for
//! feature requests and bug reports.

pub mod breakdown;
pub mod completeness;
pub mod transition;
pub mod will_not_do;

pub use breakdown::{
    BreakdownAnalyzer, BreakdownComment, ChildBeadRequest, EpicBreakdown,
};
pub use completeness::{
    check_bug_completeness, check_feature_completeness, CompletenessCheckResult,
};
pub use transition::{
    execute_breakdown, BreakdownResult, FeatureBugIssue, TransitionSummary,
};
