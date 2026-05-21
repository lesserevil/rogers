//! Release manager module for Rodgers.
//!
//! This module provides the release management system that detects
//! release candidacy, creates proposal discussions, and executes
//! releases with human approval.
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

pub mod detector;
pub mod execution;
pub mod manager;
pub mod proposal;

pub use detector::{
    Blocker, BlockerReason, CandidacyResult, LastRelease, ReleaseCandidate, ReleaseSource,
};
pub use execution::{ReleaseExecutionError, ReleaseExecutor, ReleaseResult};
pub use manager::{PendingApproval, ReleaseManager, ReleaseRunResult, ReleaseState};
pub use proposal::{ApprovalResult, ReleaseApproval, ReleaseProposalManager};
