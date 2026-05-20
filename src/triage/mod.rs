//! Triage loop for processing GitHub issues.
//!
//! This module handles the automated triage workflow:
//! - Checking issue conformance (template usage)
//! - Posting reformat offers for non-conforming issues
//! - Ensuring one-offer-only policy

pub mod reformat_offer;
pub mod triage_loop;

// Re-export commonly used items from submodules
pub use reformat_offer::{
    OFFER_SENT_LABEL, REFOMAT_OFFER_COMMENT, build_reformat_offer_comment, needs_reformat_offer,
    suggest_template_type,
};
pub use triage_loop::{
    GitHubIssue, IssueState, TriageConfig, TriageResult, count_needs_offer, process_issue,
    process_issues,
};
