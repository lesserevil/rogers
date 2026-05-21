//! Release management module.
//!
//! This module handles release proposal, approval, and execution.
//! It also includes backport trigger detection for merging fixes to release branches.
//!
//! ## Modules
//!
//! - `changelog` - Changelog generation from PR data
//! - `backport_trigger` - Backport label detection and trigger creation

pub mod backport_trigger;
pub mod changelog;

pub use backport_trigger::{
    BackportConfig, BackportDetectionResult, BackportTriggerEvent, build_approval_discussion_body,
    build_backport_pending_comment, create_trigger_from_merge, create_trigger_from_triage,
    detect_backport_candidate, identify_target_branches,
};
pub use changelog::{
    ChangelogConfig, ConventionalCommitType, GroupedPRs, ParsedCommit, PullRequest,
    generate_markdown, generate_release_notes, group_prs_by_type, parse_conventional_commit,
};
