//! Rodgers — GitHub-native community relations agent.

mod cli;
mod error;
mod init;
mod labels;
mod templates;

pub use error::{Result, RogersError};

fn main() {
    println!("Hello, world!");
}

// Re-export public types for use by other modules
pub use crate::init::{
    AuditFinding, InitAuditResult, Severity, format_audit_result, run_init_audit,
};
pub use crate::templates::{DiscoveryResult, TemplateStatus, discover_templates};
