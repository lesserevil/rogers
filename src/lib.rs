//! Rodgers - GitHub-native community relations agent
//!
//! This library implements the triage-to-release lifecycle for GitHub issues.
//! It processes new and updated issues, verifies completeness, manages labels,
//! and coordinates with GitHub and the beads database.
//!
//! ## Modules
//!
//! - `backport` - Backport detection and workflow management
//!   - `manager` - Backport manager entry point
//! - `beads` - Bead database client for filing work items
//! - `doctor` - Health checks and diagnostics
//! - `feature_bug` - Bug and feature issue handling
//!   - `completeness` - Completeness verification for bug/feature issues
//! - `github` - GitHub API client
//! - `labels` - Canonical label definitions
//! - `llm` - Language model integration
//! - `release` - Release management
//!   - `changelog` - Changelog generation from PR data
//!   - `backport_trigger` - Backport trigger detection
//! - `triage` - Triage loop processing

pub mod backport;
pub mod beads;
pub mod config;
pub mod error;
pub mod feature_bug;
pub mod github;
pub mod labels;
pub mod llm;
pub mod release;
pub mod triage;
