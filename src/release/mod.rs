//! Release management — coordinates release suggestions from backport completeness.
//!
//! This module handles CRIT-6 from plans/backport-plan.md: when all critical
//! backports for a release branch are merged, this module files a release
//! suggestion bead that triggers the longer release process.
//!
//! The actual release cut (branch creation, tagging, GitHub Release) is
//! handled per plans/release-management-plan.md by the release process,
//! triggered by the bead filed here. This module is responsible only for:
//! 1. Detecting completeness conditions
//! 2. Filing the release suggestion bead
//! 3. Coordinating with the triage loop
//!
//! ## Release suggestion bead shape
//!
//! - type: chore
//! - tag: rodgers:type=release
//! - title: "Release Suggestion: {branch_name}"
//! - description: summarizes what critical backports are included

pub mod manager;
