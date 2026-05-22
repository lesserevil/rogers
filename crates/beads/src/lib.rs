//! Beads database module for Rodgers.
//!
//! This module provides database operations for tracking work via beads,
//! which uses Dolt as the underlying storage backend. Dolt provides
//! Git-like version control capabilities for the data.
//!
//! ## Tables
//!
//! - **rodgers_epics**: Top-level work units covering features or bug fixes
//! - **rodgers_children**: Sub-work items belonging to epics
//! - **rodgers_state**: Key-value store for scheduler state and configuration

pub mod client;
pub mod controller;
pub mod migration;
pub mod schema;

pub use client::BeadsClient;
pub use controller::BeadController;
pub use migration::{run_migrations, verify_schema};
pub use schema::{Child, Epic, State, SCHEMA_VERSION};

// Re-export core types for convenience
pub use rogers_core::error::{Result, RogersError};
