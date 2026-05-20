//! Templates module for GitHub issue template management.
//!
//! This module handles template discovery and validation for GitHub issue
//! templates. It checks for canonical templates (bug_report, feature_request,
//! question) and provides data structures for audit reporting.

pub mod discovery;

pub use discovery::{DiscoveryResult, TemplateStatus, discover_templates};
