//! Triage module.
//!
//! Provides the triage engine for classifying and processing GitHub issues.

pub mod classifier;
pub mod config;
pub mod engine;
pub mod state_machine;
pub mod triage_loop;

pub use classifier::{ClassificationResult, Classifier};
pub use engine::TriageEngine;
pub use state_machine::{TransitionError, TriageState, TriageStateMachine};
