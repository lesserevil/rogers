//! Release management module.
//!
//! This module handles release proposal, approval, and execution.
//! It also includes backport trigger detection for merging fixes to release branches.
//!
//! ## Modules
//!
//! - `bead` - Release bead creation with full metadata for audit trail
//! - `changelog` - Changelog generation from PR data
//! - `backport_trigger` - Backport label detection and trigger creation
//! - `config` - Release configuration loading and validation

pub mod backport_trigger;
pub mod bead;
pub mod changelog;
pub mod config;

pub use backport_trigger::{
    BackportConfig, BackportDetectionResult, BackportTriggerEvent, build_approval_discussion_body,
    build_backport_pending_comment, create_trigger_from_merge, create_trigger_from_triage,
    detect_backport_candidate, identify_target_branches,
};
pub use bead::{
    ReleaseBeadMetadata, build_release_bead_request, build_release_bead_start,
    build_release_bead_with_url, update_release_bead_for_github_release,
};
pub use changelog::{
    ChangelogConfig, ConventionalCommitType, GroupedPRs, ParsedCommit, PullRequest,
    generate_markdown, generate_release_notes, group_prs_by_type, parse_conventional_commit,
};
