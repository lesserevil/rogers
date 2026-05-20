//! Issue template management.
//!
//! This module handles:
//! - Default templates embedded in the binary
//! - Template discovery and validation
//! - Conformance detection for non-conforming issues
//! - Bead filing when templates are missing

pub mod conformance;
pub mod defaults;
pub mod discovery;

pub use conformance::{
    ConformanceResult, TemplateType, check_conformance, is_email_reply, is_non_conforming,
};
pub use defaults::{BUG_REPORT_TEMPLATE, FEATURE_REQUEST_TEMPLATE, QUESTION_TEMPLATE};
pub use discovery::{
    REQUIRED_TEMPLATES, TEMPLATE_BEAD_TITLE, TEMPLATE_BEAD_TYPE_LABEL, TemplateDiscovery,
};
