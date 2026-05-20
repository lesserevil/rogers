//! Feature and bug analysis module.
//!
//! This module provides epic detection and breakdown analysis for
//! feature requests and bug reports.

pub mod breakdown;

pub use breakdown::{
    BreakdownAnalyzer, BreakdownComment, ChildBeadRequest, EpicBreakdown,
};