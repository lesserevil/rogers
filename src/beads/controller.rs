//! Bead controller for high-level bead operations.
//!
//! Provides the high-level interface for creating and managing epics
//! and child beads. Coordinates between GitHub issues and beads database.

use crate::beads::client::BeadsClient;
use crate::beads::schema::{bead_type, status, Child, Epic};
use crate::error::{Result, RogersError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Bead controller for epic/child bead operations.
#[derive(Debug, Clone)]
pub struct BeadController {
    /// Database client.
    db: Arc<BeadsClient>,
}

impl BeadController {
    /// Create a new bead controller.
    pub fn new(db: Arc<BeadsClient>) -> Self {
        Self { db }
    }

    /// Create a new bead controller from configuration.
    pub fn from_client(db: BeadsClient) -> Self {
        Self { db: Arc::new(db) }
    }

    /// File an epic bead for a GitHub issue.
    ///
    /// The epic is created with status "deferred" initially,
    /// linking to the GitHub issue.
    pub async fn file_epic(&self, request: CreateEpicRequest) -> Result<Epic> {
        let epic = Epic {
            id: nanoid::simple(),
            title: request.title,
            description: request.description,
            bead_type: request
                .bead_type
                .unwrap_or_else(|| bead_type::EPIC.to_string()),
            // Create the epic with status "deferred" (closed initially since it's deferred work)
            status: status::CLOSED.to_string(), // Deferred - closed initially
            github_issue_url: request.github_issue_url,
            github_issue_state: Some("open".to_string()),
            rodgers_type: request.rodgers_type,
            rodgers_labels: request.rodgers_labels,
            rodgers_parent: None,
            discovered_from: request.discovered_from,
            acceptance_criteria: request.acceptance_criteria,
            priority: request.priority,
            assignee: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.insert_epic(&epic).await?;
        Ok(epic)
    }

    /// File child beads for an epic.
    ///
    /// All children are created with status "deferred" (closed) initially.
    /// They are batch-opened when the human signal is received.
    pub async fn file_children(
        &self,
        parent_id: &str,
        requests: Vec<CreateChildRequest>,
    ) -> Result<Vec<Child>> {
        let mut children = Vec::new();

        for request in requests {
            let child = Child {
                id: nanoid::simple(),
                parent_id: parent_id.to_string(),
                title: request.title,
                description: request.description,
                bead_type: request
                    .bead_type
                    .unwrap_or_else(|| bead_type::TASK.to_string()),
                // Create the epic with status "deferred" (closed initially since it's deferred work)
                status: status::CLOSED.to_string(), // Deferred - closed initially
                github_issue_url: None,
                rodgers_type: request.rodgers_type,
                rodgers_labels: request.rodgers_labels,
                rodgers_parent: Some(parent_id.to_string()),
                discovered_from: None,
                acceptance_criteria: request.acceptance_criteria,
                priority: request.priority,
                assignee: None,
                created_at: Utc::now(),
            };

            self.insert_child(&child).await?;
            children.push(child);
        }

        Ok(children)
    }

    /// Batch open child beads (human signal received).
    ///
    /// Changes all children of the given epic from "deferred" to "open".
    pub async fn batch_open_children(&self, parent_id: &str) -> Result<Vec<Child>> {
        // Update all children to open status
        let sql = format!(
            "UPDATE rodgers_children SET status = '{}', updated_at = NOW() WHERE parent_id = '{}' AND status = '{}'",
            status::OPEN, parent_id, status::CLOSED
        );

        self.db.execute(&sql)?;
        drop(sql);

        // Fetch updated children
        let sql = format!(
            "SELECT * FROM rodgers_children WHERE parent_id = '{}'",
            parent_id
        );

        let rows = self.db.query(&sql)?;
        let children = rows
            .into_iter()
            .filter_map(|row| self.row_to_child(&row).ok())
            .collect();

        Ok(children)
    }

    /// Get children for an epic.
    pub async fn get_children(&self, parent_id: &str) -> Result<Vec<Child>> {
        let sql = format!(
            "SELECT * FROM rodgers_children WHERE parent_id = '{}'",
            parent_id
        );

        let rows = self.db.query(&sql)?;
        let children = rows
            .into_iter()
            .filter_map(|row| self.row_to_child(&row).ok())
            .collect();

        Ok(children)
    }

    /// Get an epic by GitHub issue URL.
    pub async fn get_epic_by_issue(&self, issue_url: &str) -> Result<Option<Epic>> {
        let sql = format!(
            "SELECT * FROM rodgers_epics WHERE github_issue_url = '{}'",
            sql_escape(issue_url)
        );

        let rows = self.db.query(&sql)?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(self.row_to_epic(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Check if an epic has children.
    pub async fn epic_has_children(&self, epic_id: &str) -> Result<bool> {
        let sql = format!(
            "SELECT COUNT(*) as cnt FROM rodgers_children WHERE parent_id = '{}'",
            sql_escape(epic_id)
        );

        let rows = self.db.query(&sql)?;
        if let Some(row) = rows.first() {
            if let Some(cnt) = row.get("cnt") {
                return Ok(cnt.as_i64().unwrap_or(0) > 0);
            }
        }
        Ok(false)
    }

    /// Update epic status.
    pub async fn update_epic_status(&self, epic_id: &str, new_status: &str) -> Result<()> {
        if !status::is_valid(new_status) {
            return Err(RogersError::Config(format!(
                "Invalid status: {}",
                new_status
            )));
        }

        let sql = format!(
            "UPDATE rodgers_epics SET status = '{}', updated_at = NOW() WHERE id = '{}'",
            sql_escape(new_status),
            sql_escape(epic_id)
        );

        self.db.execute(&sql)?;
        Ok(())
    }

    /// Update child status.
    pub async fn update_child_status(&self, child_id: &str, new_status: &str) -> Result<()> {
        if !status::is_valid(new_status) {
            return Err(RogersError::Config(format!(
                "Invalid status: {}",
                new_status
            )));
        }

        let sql = format!(
            "UPDATE rodgers_children SET status = '{}' WHERE id = '{}'",
            sql_escape(new_status),
            sql_escape(child_id)
        );

        self.db.execute(&sql)?;
        Ok(())
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    /// Insert an epic into the database.
    async fn insert_epic(&self, epic: &Epic) -> Result<()> {
        let sql = format!(
            r#"INSERT INTO rodgers_epics (
                id, title, description, bead_type, status,
                github_issue_url, github_issue_state,
                rodgers_type, rodgers_labels, rodgers_parent,
                discovered_from, acceptance_criteria,
                priority, assignee, created_at, updated_at
            ) VALUES (
                '{}', '{}', {}, '{}', '{}',
                {}, {},
                {}, {}, {},
                {}, {},
                {}, {}, '{}', '{}'
            )"#,
            sql_escape(&epic.id),
            sql_escape(&epic.title),
            epic.description
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            sql_escape(&epic.bead_type),
            sql_escape(&epic.status),
            epic.github_issue_url
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.github_issue_state
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.rodgers_type
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.rodgers_labels
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.rodgers_parent
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.discovered_from
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.acceptance_criteria
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.priority
                .map(|p| p.to_string())
                .unwrap_or_else(|| "NULL".to_string()),
            epic.assignee
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            epic.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            epic.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        self.db.execute(&sql)?;
        Ok(())
    }

    /// Insert a child into the database.
    async fn insert_child(&self, child: &Child) -> Result<()> {
        let sql = format!(
            r#"INSERT INTO rodgers_children (
                id, parent_id, title, description, bead_type, status,
                github_issue_url,
                rodgers_type, rodgers_labels, rodgers_parent, discovered_from,
                acceptance_criteria, priority, assignee, created_at
            ) VALUES (
                '{}', '{}', '{}', {}, '{}', '{}',
                {},
                {}, {}, {}, {},
                {}, {}, {}, '{}'
            )"#,
            sql_escape(&child.id),
            sql_escape(&child.parent_id),
            sql_escape(&child.title),
            child
                .description
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            sql_escape(&child.bead_type),
            sql_escape(&child.status),
            child
                .github_issue_url
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            child
                .rodgers_type
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            child
                .rodgers_labels
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            child
                .rodgers_parent
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            child
                .discovered_from
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            child
                .acceptance_criteria
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            child
                .priority
                .map(|p| p.to_string())
                .unwrap_or_else(|| "NULL".to_string()),
            child
                .assignee
                .as_ref()
                .map(|s| format!("'{}'", sql_escape(s)))
                .unwrap_or_else(|| "NULL".to_string()),
            child.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        self.db.execute(&sql)?;
        Ok(())
    }

    /// Convert a database row to an Epic.
    fn row_to_epic(
        &self,
        row: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<Epic> {
        let get_str = |key: &str| -> Option<String> {
            row.get(key).and_then(|v| v.as_str()).map(String::from)
        };
        let get_i64 =
            |key: &str| -> Option<i32> { row.get(key).and_then(|v| v.as_i64()).map(|n| n as i32) };
        let get_datetime = |key: &str| -> chrono::DateTime<Utc> {
            row.get(key)
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now)
        };

        Ok(Epic {
            id: get_str("id").unwrap_or_default(),
            title: get_str("title").unwrap_or_default(),
            description: get_str("description"),
            bead_type: get_str("bead_type").unwrap_or_else(|| "epic".to_string()),
            status: get_str("status").unwrap_or_else(|| "open".to_string()),
            github_issue_url: get_str("github_issue_url"),
            github_issue_state: get_str("github_issue_state"),
            rodgers_type: get_str("rodgers_type"),
            rodgers_labels: get_str("rodgers_labels"),
            rodgers_parent: get_str("rodgers_parent"),
            discovered_from: get_str("discovered_from"),
            acceptance_criteria: get_str("acceptance_criteria"),
            priority: get_i64("priority"),
            assignee: get_str("assignee"),
            created_at: get_datetime("created_at"),
            updated_at: get_datetime("updated_at"),
        })
    }

    /// Convert a database row to a Child.
    fn row_to_child(
        &self,
        row: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<Child> {
        let get_str = |key: &str| -> Option<String> {
            row.get(key).and_then(|v| v.as_str()).map(String::from)
        };
        let get_i64 =
            |key: &str| -> Option<i32> { row.get(key).and_then(|v| v.as_i64()).map(|n| n as i32) };
        let get_datetime = |key: &str| -> chrono::DateTime<Utc> {
            row.get(key)
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now)
        };

        Ok(Child {
            id: get_str("id").unwrap_or_default(),
            parent_id: get_str("parent_id").unwrap_or_default(),
            title: get_str("title").unwrap_or_default(),
            description: get_str("description"),
            bead_type: get_str("bead_type").unwrap_or_else(|| "task".to_string()),
            status: get_str("status").unwrap_or_else(|| "open".to_string()),
            github_issue_url: get_str("github_issue_url"),
            rodgers_type: get_str("rodgers_type"),
            rodgers_labels: get_str("rodgers_labels"),
            rodgers_parent: get_str("rodgers_parent"),
            discovered_from: get_str("discovered_from"),
            acceptance_criteria: get_str("acceptance_criteria"),
            priority: get_i64("priority"),
            assignee: get_str("assignee"),
            created_at: get_datetime("created_at"),
        })
    }
}

/// Escape a string for SQL (simple escaping for single quotes).
fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}

// ─── Request/Response types ─────────────────────────────────────────────────

/// Request to create an epic bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEpicRequest {
    /// Epic title (typically from GitHub issue).
    pub title: String,
    /// Epic description.
    pub description: Option<String>,
    /// Bead type (default: epic).
    pub bead_type: Option<String>,
    /// GitHub issue URL this epic is linked to.
    pub github_issue_url: Option<String>,
    /// Rodgers type metadata.
    pub rodgers_type: Option<String>,
    /// Rodgers labels.
    pub rodgers_labels: Option<String>,
    /// What this work was discovered from.
    pub discovered_from: Option<String>,
    /// Acceptance criteria for this epic.
    pub acceptance_criteria: Option<String>,
    /// Priority (1=highest, 5=lowest).
    pub priority: Option<i32>,
}

/// Request to create a child bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChildRequest {
    /// Child bead title.
    pub title: String,
    /// Child bead description.
    pub description: Option<String>,
    /// Bead type (default: task).
    pub bead_type: Option<String>,
    /// Rodgers type metadata.
    pub rodgers_type: Option<String>,
    /// Rodgers labels.
    pub rodgers_labels: Option<String>,
    /// Acceptance criteria for this child.
    pub acceptance_criteria: Option<String>,
    /// Priority (1=highest, 5=lowest).
    pub priority: Option<i32>,
}

/// Breakdown result including epic and children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownResult {
    /// The created epic bead.
    pub epic: Epic,
    /// The created child beads.
    pub children: Vec<Child>,
    /// URLs to the beads (for posting in comment).
    pub epic_url: Option<String>,
    pub child_urls: Vec<Option<String>>,
}

// ─── nanoid wrapper for unique IDs ────────────────────────────────────────

mod nanoid {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(1);

    /// Generate a simple unique ID (not cryptographically secure,
    /// but sufficient for local bead IDs).
    pub fn simple() -> String {
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{}-{}", timestamp, counter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_escape() {
        assert_eq!(sql_escape("test"), "test");
        assert_eq!(sql_escape("O'Reilly"), "O''Reilly");
        assert_eq!(sql_escape("it's fine"), "it''s fine");
    }

    #[test]
    fn test_create_epic_request() {
        let request = CreateEpicRequest {
            title: "Test Epic".to_string(),
            description: Some("Test description".to_string()),
            bead_type: None,
            github_issue_url: Some("https://github.com/test/repo/issues/123".to_string()),
            rodgers_type: Some("epic".to_string()),
            rodgers_labels: None,
            discovered_from: None,
            acceptance_criteria: Some("- [ ] AC-1: Test".to_string()),
            priority: Some(1),
        };

        assert_eq!(request.title, "Test Epic");
        assert!(request.acceptance_criteria.is_some());
    }

    #[test]
    fn test_create_child_request() {
        let request = CreateChildRequest {
            title: "Test Child".to_string(),
            description: Some("Child description".to_string()),
            bead_type: Some("feature".to_string()),
            rodgers_type: Some("feature".to_string()),
            rodgers_labels: None,
            acceptance_criteria: Some("- [ ] AC-1: Test child".to_string()),
            priority: Some(2),
        };

        assert_eq!(request.title, "Test Child");
        assert_eq!(request.bead_type, Some("feature".to_string()));
    }

    #[test]
    fn test_nanoid_unique() {
        let id1 = nanoid::simple();
        let id2 = nanoid::simple();
        assert_ne!(id1, id2);
        assert!(id1.contains("-"));
        assert!(id2.contains("-"));
    }
}
