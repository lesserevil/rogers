//! Release management module.
//!
//! This module handles release proposal, approval, and execution.
//! It also includes backport trigger detection for merging fixes to release branches.
//!
//! ## Modules
//!
//! - `task` - Release task creation with full metadata for audit trail
//! - `changelog` - Changelog generation from PR data
//! - `detector` - Release candidacy detection from merged PRs since last tag
//! - `backport_trigger` - Backport label detection and trigger creation
//! - `config` - Release configuration loading and validation
//! - `branch` - Release branch creation and version computation
//! - `tag` - Git tag creation with semantic version

pub mod backport_trigger;
pub mod branch;
pub mod changelog;
pub mod config;
pub mod detector;
pub mod tag;
pub mod task;

pub use backport_trigger::{
    build_approval_discussion_body, build_backport_pending_comment, create_trigger_from_merge,
    create_trigger_from_triage, detect_backport_candidate, identify_target_branches,
    BackportConfig, BackportDetectionResult, BackportTriggerEvent,
};
pub use branch::{
    create_branch, determine_source_branch, ReleaseBranchConfig, ReleaseBranchResult,
};
pub use changelog::{
    generate_markdown, generate_release_notes, group_prs_by_type, parse_conventional_commit,
    ChangelogConfig, ConventionalCommitType, GroupedPRs, ParsedCommit, PullRequest,
};
pub use detector::{
    determine_version_bump, DetectionResult, DetectorConfig, NoReleaseReason, ReleaseDetector,
    SemVer, VersionBump,
};
pub use tag::{
    build_release_message, build_release_message_with_changelog, create_tag, create_tag_local,
    TagConfig, TagResult,
};
pub use task::{
    build_release_task_request, build_release_task_start, build_release_task_with_url,
    update_release_task_for_github_release, ReleaseTaskMetadata,
};
