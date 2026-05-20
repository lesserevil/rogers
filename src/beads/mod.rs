//! Beads client for creating tracking beads.
//!
//! This module provides the interface for filing epic and child beads
//! against GitHub issues.
pub mod client;

pub use client::{
    BeadClient, BeadStatus, BeadType, ChildBeadSpec, EpicBeadResult, EpicScaleResult,
};

// ===== Backward compatibility types (for doctor module) =====

use serde::{Deserialize, Serialize};

/// Backward compatibility: Bead status enumeration  
/// Re-exported as `BeadStatus` for compatibility with old code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeadStatusCompat {
    Open,
    InProgress,
    Closed,
    Deferred,
}

impl std::fmt::Display for BeadStatusCompat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeadStatusCompat::Open => write!(f, "open"),
            BeadStatusCompat::InProgress => write!(f, "in_progress"),
            BeadStatusCompat::Closed => write!(f, "closed"),
            BeadStatusCompat::Deferred => write!(f, "deferred"),
        }
    }
}

/// Backward compatibility: A bead from the database
/// Re-exported as `Bead` for doctor module compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead {
    /// Unique bead identifier (e.g., "b-001")
    pub id: String,
    /// Bead title
    pub title: String,
    /// Current status
    pub status: BeadStatusCompat,
    /// URL to linked GitHub issue (if any)
    pub github_issue_url: Option<String>,
    /// Discovered from dependency (parent bead ID)
    pub discovered_from: Option<String>,
    /// Plan reference (e.g., "plans/doctor-plan.md §5")
    pub plan: Option<String>,
}

/// Backward compatibility: Beads database client
/// For drift detection (AC-5), this is a placeholder that returns empty results.
/// The actual implementation would query dolt SQL.
pub struct BeadsClient {
    remote: String,
    database: String,
}

impl BeadsClient {
    /// Create a new beads client
    pub fn new(remote: &str, database: Option<&str>) -> Self {
        let database = database.unwrap_or("message.hibernate").to_string();
        Self {
            remote: remote.to_string(),
            database,
        }
    }

    /// Fetch all closed beads from the database
    ///
    /// Returns empty Vec in this placeholder implementation.
    pub async fn get_closed_beads(&self) -> crate::error::Result<Vec<Bead>> {
        tracing::debug!(
            "Fetching closed beads from {} / {}",
            self.remote,
            self.database
        );
        Ok(Vec::new())
    }

    /// Fetch beads by status (placeholder)
    pub async fn get_beads_by_status(
        &self,
        _status: BeadStatusCompat,
    ) -> crate::error::Result<Vec<Bead>> {
        Ok(Vec::new())
    }

    /// Fetch beads with pagination support (placeholder)
    pub async fn get_beads_paginated(
        &self,
        _status: Option<BeadStatusCompat>,
        _offset: usize,
        _limit: usize,
    ) -> crate::error::Result<Vec<Bead>> {
        Ok(Vec::new())
    }
}
