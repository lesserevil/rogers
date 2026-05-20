//! Issue template management.
//!
//! This module handles:
//! - Default templates embedded in the binary
//! - Template discovery and validation
//! - Conformance detection for non-conforming issues
//! - Bead filing when templates are missing
//! - Reformatting with explicit approval gate

pub mod conformance;
pub mod defaults;
pub mod discovery;
pub mod reformat;

pub use conformance::{
    ConformanceResult, TemplateType, check_conformance, is_email_reply, is_non_conforming,
};
pub use defaults::{BUG_REPORT_TEMPLATE, FEATURE_REQUEST_TEMPLATE, QUESTION_TEMPLATE};
pub use discovery::{
    REQUIRED_TEMPLATES, TEMPLATE_BEAD_TITLE, TEMPLATE_BEAD_TYPE_LABEL, TemplateDiscovery,
};
pub use reformat::{
    ApprovalResponse, ReformatState, ReviewResponse, detect_approval_response,
    detect_review_response, generate_reformat_offer_comment, generate_reformat_review_comment,
    generate_reformatted_content, looks_like_review_response, needs_clarification,
    review_needs_clarification, should_accept_freeform, should_apply_reformat,
    should_keep_original, should_reformat, wants_reformat_changes,
};
