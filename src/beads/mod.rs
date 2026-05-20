//! Beads client for creating tracking beads.
//!
//! This module provides the interface for filing epic and child beads
//! against GitHub issues.

pub mod client;

pub use client::{
    BeadClient, BeadStatus, BeadType, ChildBeadSpec, EpicBeadResult, EpicScaleResult,
};