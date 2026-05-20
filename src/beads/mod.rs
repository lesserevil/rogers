//! Beads module for tracking work items via bd CLI.

pub mod client;

pub use client::{
    BeadClient, BeadCreateResponse, BeadInfo, BeadType, RODGERS_TAG_DOCS,
};