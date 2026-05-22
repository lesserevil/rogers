//! Rodgers - GitHub-native community relations agent
//!
//! This library implements the triage-to-release lifecycle for GitHub issues.
//! It processes new and updated issues, verifies completeness, manages labels,
//! and coordinates with GitHub and the beads database.
//!
//! ## Modules
//!
//! - `feature_bug` - Bug and feature issue handling
//!   - `completeness` - Completeness verification for bug/feature issues
//! - `triage` - Triage loop processing

pub mod beads;
pub mod checks;
pub mod cli;
pub mod config;
pub mod error;
pub mod feature_bug;
pub mod github;
pub mod init;
pub mod labels;
pub mod llm;
pub mod triage;
