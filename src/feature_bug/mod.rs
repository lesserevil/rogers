//! Feature bug module.
//!
//! This module handles bug report and feature request completeness checking
//! and triage:
//! - Completeness verification for bug reports and feature requests
//! - Triage loop integration (ready-for-review when complete)

pub mod breakdown;
pub mod completeness;
pub mod triage_loop;

pub use completeness::{
    CompletenessResult, check_bug_completeness, check_bug_completeness_semantic,
    check_feature_completeness,
};
pub use triage_loop::{TriageAction, is_bug_ready_for_review, triage_bug_completeness};
