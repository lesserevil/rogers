//! Rodgers - GitHub-native community relations agent
//!
//! This library implements the triage-to-release lifecycle for GitHub issues.
//! It processes new and updated issues, verifies completeness, manages labels,
//! and coordinates with GitHub and the beads database.
//!
//! ## Modules
//!
//! - `feature_bug` - Bug and feature issue handling
//! - `triage` - Triage loop processing

// Re-export workspace crate modules under their original names
pub use rogers_beads as beads;
pub use rogers_github as github;
pub use rogers_llm as llm;
pub use rogers_triage as triage;
pub use rogers_feature_bug as feature_bug;

// Re-export core types for convenience
pub use rogers_core::error::{Result, RogersError};

// Root crate modules
pub mod cli;
pub mod config;
pub mod labels;
