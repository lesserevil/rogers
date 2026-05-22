//! Release manager module for Rodgers.
//!
//! This module provides the release management system that detects
//! release candidacy, creates proposal discussions, and executes
//! releases with human approval. It also includes changelog generation
//! from PR data using conventional commits and backport trigger detection.
//!
//! ## Overview
//!
//! The release manager runs on each scheduler cycle and:
//! 1. Detects candidate releases from merged PRs since last tag
//! 2. Surfaces potential blockers (blocker label, priority, human-flagged)
//! 3. Creates a GitHub Discussion for release approval
//! 4. Waits for 👍 reaction before executing
//! 5. Creates the release branch, git tag, and GitHub Release
//! 6. Posts a notification and closes the discussion
//! 7. Handles stale proposals (reminder → close + revisit bead)

pub mod backport_trigger;
pub mod bead;
pub mod branch;
pub mod changelog;
pub mod config;
pub mod detector;
pub mod execution;
pub mod manager;
pub mod proposal;
pub mod tag;

pub use backport_trigger::{
    BackportConfig, BackportDetectionResult, BackportTriggerEvent, build_approval_discussion_body,
    build_backport_pending_comment, create_trigger_from_merge, create_trigger_from_triage,
    detect_backport_candidate, identify_target_branches,
};
pub use bead::{
    ReleaseBeadMetadata, build_release_bead_request, build_release_bead_start,
    build_release_bead_with_url, update_release_bead_for_github_release,
};
pub use branch::{
    ReleaseBranchConfig, ReleaseBranchResult, create_branch, determine_source_branch,
};
pub use changelog::{
    ChangelogConfig, ConventionalCommitType, GroupedPRs, ParsedCommit, PullRequest,
    generate_markdown, generate_release_notes, group_prs_by_type, parse_conventional_commit,
};
pub use detector::{
    Blocker, BlockerReason, CandidacyResult, LastRelease, ReleaseCandidate, ReleaseSource,
};
pub use execution::{ReleaseExecutionError, ReleaseExecutor, ReleaseResult};
pub use manager::{PendingApproval, ReleaseManager, ReleaseRunResult, ReleaseState};
pub use proposal::{ApprovalResult, ReleaseApproval, ReleaseProposalManager};
pub use tag::{
    TagConfig, TagResult, build_release_message, build_release_message_with_changelog, create_tag,
    create_tag_local,
};
