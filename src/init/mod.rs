//! Rodgers init — Project Readiness Audit
//!
//! This module performs initial setup audits for repositories managed by Rodgers.
//! It checks for required labels, issue templates, and other prerequisites.

pub mod audit;

pub use audit::{AuditFinding, InitAuditResult, Severity, format_audit_result, run_init_audit};
