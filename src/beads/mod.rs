//! Beads database client for Rodgers
//!
//! Provides methods for interacting with the beads (dolt) database.

use crate::error::{Result, RogersError};
use serde::{Deserialize, Serialize};

/// Bead status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeadStatus {
    Open,
    InProgress,
    Closed,
    Deferred,
}

impl std::fmt::Display for BeadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeadStatus::Open => write!(f, "open"),
            BeadStatus::InProgress => write!(f, "in_progress"),
            BeadStatus::Closed => write!(f, "closed"),
            BeadStatus::Deferred => write!(f, "deferred"),
        }
    }
}

/// A bead from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead {
    /// Unique bead identifier (e.g., "b-001")
    pub id: String,
    /// Bead title
    pub title: String,
    /// Current status
    pub status: BeadStatus,
    /// URL to linked GitHub issue (if any)
    pub github_issue_url: Option<String>,
    /// Discovered from dependency (parent bead ID)
    pub discovered_from: Option<String>,
    /// Plan reference (e.g., "plans/doctor-plan.md §5")
    pub plan: Option<String>,
}

/// Trait for bead data source - allows mocking in tests
pub trait BeadSource: Send + Sync {
    /// Fetch all beads with a specific status
    fn get_beads_by_status(
        &self,
        status: BeadStatus,
    ) -> impl std::future::Future<Output = Result<Vec<Bead>>> + Send;

    /// Fetch beads lazily/paginated - returns beads starting from offset
    fn get_beads_paginated(
        &self,
        status: Option<BeadStatus>,
        offset: usize,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<Vec<Bead>>> + Send;

    /// Fetch a single bead by ID
    fn get_bead(&self, id: &str) -> impl std::future::Future<Output = Result<Option<Bead>>> + Send;
}

/// Mock bead source for testing
pub struct MockBeadSource {
    beads: Vec<Bead>,
}

impl MockBeadSource {
    pub fn new(beads: Vec<Bead>) -> Self {
        Self { beads }
    }
}

impl BeadSource for MockBeadSource {
    async fn get_beads_by_status(&self, status: BeadStatus) -> Result<Vec<Bead>> {
        Ok(self
            .beads
            .iter()
            .filter(|b| b.status == status)
            .cloned()
            .collect())
    }

    async fn get_beads_paginated(
        &self,
        status: Option<BeadStatus>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Bead>> {
        let filtered: Vec<Bead> = match status {
            Some(s) => self
                .beads
                .iter()
                .filter(|b| b.status == s)
                .cloned()
                .collect(),
            None => self.beads.clone(),
        };

        let end = std::cmp::min(offset + limit, filtered.len());
        if offset >= filtered.len() {
            return Ok(Vec::new());
        }

        Ok(filtered[offset..end].to_vec())
    }

    async fn get_bead(&self, id: &str) -> Result<Option<Bead>> {
        Ok(self.beads.iter().find(|b| b.id == id).cloned())
    }
}

/// Beads database client
///
/// In production, this would use dolt SQL queries.
/// For now, it provides the client interface structure.
pub struct BeadsClient {
    /// Remote URL for dolt database
    remote: String,
    /// Database name
    database: String,
}

impl BeadsClient {
    pub fn new(remote: &str, database: Option<&str>) -> Self {
        let database = database.unwrap_or("message.hibernate").to_string();
        Self {
            remote: remote.to_string(),
            database,
        }
    }

    /// Fetch all closed beads from the database
    ///
    /// This would execute: `SELECT id, title, status, github_issue_url FROM children WHERE status = 'closed'`
    pub async fn get_closed_beads(&self) -> Result<Vec<Bead>> {
        // In production, this would run a dolt SQL query against the remote
        // For now, we return an empty vector - the actual implementation
        // would use a dolt client to execute queries

        // Real implementation would:
        // 1. Connect to dolt remote at self.remote
        // 2. Query self.database
        // 3. Execute: SELECT id, title, status, github_issue_url, discovered_from FROM children WHERE status = 'closed'
        // 4. Map results to Bead structs

        tracing::debug!(
            "Fetching closed beads from {} / {}",
            self.remote,
            self.database
        );

        // Placeholder - actual implementation would query dolt
        Ok(Vec::new())
    }

    /// Fetch beads by status
    pub async fn get_beads_by_status(&self, status: BeadStatus) -> Result<Vec<Bead>> {
        tracing::debug!(
            "Fetching {:?} beads from {} / {}",
            status,
            self.remote,
            self.database
        );

        // Placeholder for actual dolt query
        Ok(Vec::new())
    }

    /// Fetch beads with pagination support
    pub async fn get_beads_paginated(
        &self,
        status: Option<BeadStatus>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Bead>> {
        tracing::debug!(
            "Fetching beads (status={:?}, offset={}, limit={}) from {} / {}",
            status,
            offset,
            limit,
            self.remote,
            self.database
        );

        // Placeholder for actual dolt query with LIMIT/OFFSET
        Ok(Vec::new())
    }

    /// Check if the beads database is reachable
    pub async fn check_connection(&self) -> Result<()> {
        tracing::debug!("Checking connection to {} / {}", self.remote, self.database);

        // In production, would execute a test query like "SELECT 1"
        // or check dolt remote connectivity
        Ok(())
    }
}

/// Parse bead status from string
pub fn parse_bead_status(s: &str) -> Option<BeadStatus> {
    match s.to_lowercase().as_str() {
        "open" => Some(BeadStatus::Open),
        "in_progress" | "in-progress" | "progress" => Some(BeadStatus::InProgress),
        "closed" | "close" => Some(BeadStatus::Closed),
        "deferred" | "defer" => Some(BeadStatus::Deferred),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bead_status() {
        assert_eq!(parse_bead_status("open"), Some(BeadStatus::Open));
        assert_eq!(parse_bead_status("OPEN"), Some(BeadStatus::Open));
        assert_eq!(parse_bead_status("closed"), Some(BeadStatus::Closed));
        assert_eq!(
            parse_bead_status("in_progress"),
            Some(BeadStatus::InProgress)
        );
        assert_eq!(
            parse_bead_status("in-progress"),
            Some(BeadStatus::InProgress)
        );
        assert_eq!(parse_bead_status("deferred"), Some(BeadStatus::Deferred));
        assert_eq!(parse_bead_status("invalid"), None);
    }

    #[test]
    fn test_bead_status_display() {
        assert_eq!(BeadStatus::Open.to_string(), "open");
        assert_eq!(BeadStatus::InProgress.to_string(), "in_progress");
        assert_eq!(BeadStatus::Closed.to_string(), "closed");
        assert_eq!(BeadStatus::Deferred.to_string(), "deferred");
    }

    #[tokio::test]
    async fn test_mock_bead_source_get_by_status() {
        let beads = vec![
            Bead {
                id: "b-001".into(),
                title: "Test 1".into(),
                status: BeadStatus::Closed,
                github_issue_url: Some("https://github.com/owner/repo/issues/123".into()),
                discovered_from: None,
                plan: None,
            },
            Bead {
                id: "b-002".into(),
                title: "Test 2".into(),
                status: BeadStatus::Open,
                github_issue_url: Some("https://github.com/owner/repo/issues/124".into()),
                discovered_from: None,
                plan: None,
            },
            Bead {
                id: "b-003".into(),
                title: "Test 3".into(),
                status: BeadStatus::Closed,
                github_issue_url: None,
                discovered_from: Some("b-001".into()),
                plan: None,
            },
        ];

        let source = MockBeadSource::new(beads);
        let closed = source
            .get_beads_by_status(BeadStatus::Closed)
            .await
            .unwrap();

        assert_eq!(closed.len(), 2);
        assert!(closed.iter().any(|b| b.id == "b-001"));
        assert!(closed.iter().any(|b| b.id == "b-003"));
    }

    #[tokio::test]
    async fn test_mock_bead_source_pagination() {
        let beads: Vec<Bead> = (1..=100)
            .map(|i| Bead {
                id: format!("b-{:03}", i),
                title: format!("Bead {}", i),
                status: BeadStatus::Open,
                github_issue_url: None,
                discovered_from: None,
                plan: None,
            })
            .collect();

        let source = MockBeadSource::new(beads);

        // Test first page
        let page1 = source.get_beads_paginated(None, 0, 10).await.unwrap();
        assert_eq!(page1.len(), 10);
        assert_eq!(page1[0].id, "b-001");
        assert_eq!(page1[9].id, "b-010");

        // Test second page
        let page2 = source.get_beads_paginated(None, 10, 10).await.unwrap();
        assert_eq!(page2.len(), 10);
        assert_eq!(page2[0].id, "b-011");

        // Test empty page
        let empty = source.get_beads_paginated(None, 1000, 10).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_mock_bead_source_get_single() {
        let beads = vec![
            Bead {
                id: "b-001".into(),
                title: "Test 1".into(),
                status: BeadStatus::Closed,
                github_issue_url: None,
                discovered_from: None,
                plan: None,
            },
            Bead {
                id: "b-002".into(),
                title: "Test 2".into(),
                status: BeadStatus::Open,
                github_issue_url: None,
                discovered_from: None,
                plan: None,
            },
        ];

        let source = MockBeadSource::new(beads);

        let bead = source.get_bead("b-001").await.unwrap();
        assert!(bead.is_some());
        assert_eq!(bead.unwrap().id, "b-001");

        let not_found = source.get_bead("b-999").await.unwrap();
        assert!(not_found.is_none());
    }
}
