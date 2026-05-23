//! Backport manager module for Rodgers.
//!
//! This module provides the backport management system that detects
//! backport-worthy commits, files approval beads, and executes backports
//! with human approval.
//!
//! ## Overview
//!
//! The backport manager runs on each scheduler cycle and:
//! 1. Detects candidate commits merged since the last run
//! 2. Files a `backport` bead for each active release branch
//! 3. Creates a GitHub Discussion for human approval
//! 4. Waits for 👍 reaction before executing
//! 5. Creates the backport branch and PR
//! 6. Handles conflicts by filing a conflict-resolution bead

pub mod approval;
pub mod detector;
pub mod execution;
pub mod manager;

pub use approval::{ApprovalResult, BackportApproval};
pub use detector::{BackportCandidate, CandidateReason, DetectionResult};
pub use execution::{BackportConflictError, BackportExecutionError};
pub use manager::{BackportManager, BackportState};
