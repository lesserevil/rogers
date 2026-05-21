//! Backport module — entry point for the backport workflow.
//!
//! This module orchestrates backport detection and bead filing.
//! See `manager` for the main backport manager implementation.

pub mod manager;

pub use manager::{BackportBeadResult, BackportManager, run_backport_triage};
