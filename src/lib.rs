//! Rodgers - GitHub-native community relations agent.
//!
//! Rodgers helps manage GitHub issues by providing templated workflows,
//! automated triage, and structured issue processing.

pub mod error;
pub mod init;
pub mod labels;
pub mod templates;
pub mod triage;

// Re-export commonly used items
pub use error::{Result, RogersError};
