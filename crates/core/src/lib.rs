//! Rogers core types shared across workspace crates.
//!
//! This crate contains the error types, configuration schema, and label
//! definitions that are shared between all Rodgers workspace members.

pub mod error;
pub mod labels;

pub use error::{Result, RogersError};
pub use labels::*;
