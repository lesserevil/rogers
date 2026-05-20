//! Triage module.
//!
//! This module handles the issue triage workflow after reformat approval.

#![allow(dead_code)]

pub struct TriageConfig {
    // Configuration for triage workflow
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TriageConfig {
    /// Create a new triage config with default values.
    pub fn new() -> Self {
        Self {}
    }
}
