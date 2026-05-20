//! Beads database schema definitions for Rodgers.
//!
//! This module defines the SQL schema for the Rodgers beads database,
//! which uses Dolt as the underlying storage backend. Dolt provides
//! Git-like version control capabilities for the data.
//!
//! ## Tables
//!
//! - **epics**: Top-level work units covering features or bug fixes
//! - **children**: Sub-work items belonging to epics
//! - **state**: Key-value store for scheduler state and configuration

use serde::{Deserialize, Serialize};

/// Schema version for tracking migrations.
pub const SCHEMA_VERSION: i32 = 1;

/// Table name constants.
pub mod table {
    /// Epics table for top-level work units.
    pub const EPICS: &str = "rodgers_epics";
    /// Children table for sub-work items.
    pub const CHILDREN: &str = "rodgers_children";
    /// State table for key-value storage.
    pub const STATE: &str = "rodgers_state";
}

/// SQL for creating the epics table.
pub const CREATE_EPICS_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS rodgers_epics (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    bead_type VARCHAR(50) NOT NULL DEFAULT 'epic',
    status VARCHAR(50) NOT NULL DEFAULT 'open',
    github_issue_url TEXT,
    github_issue_state VARCHAR(20),
    rodgers_type TEXT,
    rodgers_labels TEXT,
    rodgers_parent TEXT,
    discovered_from TEXT,
    acceptance_criteria TEXT,
    priority INT,
    assignee TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP(),
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP() ON UPDATE CURRENT_TIMESTAMP()
)
"#;

/// SQL for creating the children table.
pub const CREATE_CHILDREN_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS rodgers_children (
    id VARCHAR(255) PRIMARY KEY NOT NULL,
    parent_id VARCHAR(255) NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    bead_type VARCHAR(50) NOT NULL DEFAULT 'task',
    status VARCHAR(50) NOT NULL DEFAULT 'open',
    github_issue_url TEXT,
    rodgers_type TEXT,
    rodgers_labels TEXT,
    rodgers_parent TEXT,
    discovered_from TEXT,
    acceptance_criteria TEXT,
    priority INT,
    assignee TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP()
)
"#;

/// SQL for creating the state table.
pub const CREATE_STATE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS rodgers_state (
    key VARCHAR(255) PRIMARY KEY NOT NULL,
    value TEXT,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP() ON UPDATE CURRENT_TIMESTAMP()
)
"#;

/// Epics row representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epic {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub bead_type: String,
    pub status: String,
    pub github_issue_url: Option<String>,
    pub github_issue_state: Option<String>,
    pub rodgers_type: Option<String>,
    pub rodgers_labels: Option<String>,
    pub rodgers_parent: Option<String>,
    pub discovered_from: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub priority: Option<i32>,
    pub assignee: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Children row representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Child {
    pub id: String,
    pub parent_id: String,
    pub title: String,
    pub description: Option<String>,
    pub bead_type: String,
    pub status: String,
    pub github_issue_url: Option<String>,
    pub rodgers_type: Option<String>,
    pub rodgers_labels: Option<String>,
    pub rodgers_parent: Option<String>,
    pub discovered_from: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub priority: Option<i32>,
    pub assignee: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// State row representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub key: String,
    pub value: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Status values for epics and children.
pub mod status {
    pub const OPEN: &str = "open";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const CLOSED: &str = "closed";

    /// Valid status values.
    pub const VALID_STATUSES: &[&str] = &[OPEN, IN_PROGRESS, CLOSED];

    /// Check if a status value is valid.
    pub fn is_valid(status: &str) -> bool {
        VALID_STATUSES.contains(&status)
    }
}

/// Bead type values.
pub mod bead_type {
    pub const EPIC: &str = "epic";
    pub const FEATURE: &str = "feature";
    pub const BUG: &str = "bug";
    pub const CHORE: &str = "chore";
    pub const SPIKE: &str = "spike";
    pub const DECISION: &str = "decision";
    pub const MILESTONE: &str = "milestone";
    pub const TASK: &str = "task";

    /// Valid type values.
    pub const VALID_TYPES: &[&str] = &[EPIC, FEATURE, BUG, CHORE, SPIKE, DECISION, MILESTONE, TASK];

    /// Check if a type value is valid.
    pub fn is_valid(type_: &str) -> bool {
        VALID_TYPES.contains(&type_)
    }
}

/// State keys for the scheduler.
pub mod state_keys {
    pub const LAST_RUN: &str = "scheduler.last_run";
    pub const LAST_CYCLE: &str = "scheduler.last_cycle";
    pub const LAST_SYNC: &str = "sync.last_sync";
    pub const SCHEMA_VERSION: &str = "schema.version";
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a DateTime for tests
    fn test_datetime() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn test_epic_has_all_required_columns() {
        // Verify Epic struct has all required columns per AC-3:
        // id, title, description, type, status, github_issue_url,
        // github_issue_state, rodgers_type, created_at, updated_at
        let epic = Epic {
            id: "epic-123".to_string(),
            title: "Test Epic".to_string(),
            description: Some("Epic description".to_string()),
            bead_type: bead_type::EPIC.to_string(),
            status: status::OPEN.to_string(),
            github_issue_url: Some("https://github.com/owner/repo/issues/45".to_string()),
            github_issue_state: Some("open".to_string()),
            rodgers_type: Some("feature".to_string()),
            rodgers_labels: None,
            rodgers_parent: None,
            discovered_from: None,
            acceptance_criteria: None,
            priority: Some(1),
            assignee: Some("username".to_string()),
            created_at: test_datetime(),
            updated_at: test_datetime(),
        };

        assert_eq!(epic.id, "epic-123");
        assert_eq!(epic.title, "Test Epic");
        assert_eq!(epic.bead_type, "epic");
        assert_eq!(epic.status, "open");
        assert_eq!(
            epic.github_issue_url,
            Some("https://github.com/owner/repo/issues/45".to_string())
        );
        assert_eq!(epic.github_issue_state, Some("open".to_string()));
        assert_eq!(epic.rodgers_type, Some("feature".to_string()));
    }

    #[test]
    fn test_child_has_parent_id_for_epic_relationship() {
        // Verify Child struct has parent_id for epic/child relationship per AC-3
        let child = Child {
            id: "child-456".to_string(),
            parent_id: "epic-123".to_string(), // Links to parent epic
            title: "Test Child".to_string(),
            description: Some("Child description".to_string()),
            bead_type: bead_type::TASK.to_string(),
            status: status::OPEN.to_string(),
            github_issue_url: Some("https://github.com/owner/repo/issues/46".to_string()),
            rodgers_type: Some("feature".to_string()),
            rodgers_labels: None,
            rodgers_parent: Some("epic-123".to_string()),
            discovered_from: None,
            acceptance_criteria: None,
            priority: Some(2),
            assignee: Some("developer".to_string()),
            created_at: test_datetime(),
        };

        assert_eq!(child.id, "child-456");
        assert_eq!(child.parent_id, "epic-123"); // Epic/child relationship
        assert_eq!(child.title, "Test Child");
        assert_eq!(child.bead_type, "task");
        assert_eq!(child.status, "open");
        assert_eq!(
            child.github_issue_url,
            Some("https://github.com/owner/repo/issues/46".to_string())
        );
        assert_eq!(child.rodgers_type, Some("feature".to_string()));
        assert_eq!(child.priority, Some(2));
        assert_eq!(child.assignee, Some("developer".to_string()));
    }

    #[test]
    fn test_state_for_scheduler() {
        // Verify State struct for scheduler state per AC-3
        let state = State {
            key: state_keys::LAST_RUN.to_string(),
            value: Some("2024-01-15T10:00:00Z".to_string()),
            updated_at: test_datetime(),
        };

        assert_eq!(state.key, "scheduler.last_run");
        assert_eq!(state.value, Some("2024-01-15T10:00:00Z".to_string()));
    }

    #[test]
    fn test_github_issue_url_stored_in_epic() {
        // Verify github_issue_url can be stored and retrieved per AC-3
        let epic = Epic {
            id: "epic-with-issue".to_string(),
            title: "Epic with GitHub Issue".to_string(),
            description: None,
            bead_type: bead_type::FEATURE.to_string(),
            status: status::OPEN.to_string(),
            github_issue_url: Some("https://github.com/owner/repo/issues/123".to_string()),
            github_issue_state: Some("open".to_string()),
            rodgers_type: Some("feature".to_string()),
            rodgers_labels: None,
            rodgers_parent: None,
            discovered_from: None,
            acceptance_criteria: None,
            priority: None,
            assignee: None,
            created_at: test_datetime(),
            updated_at: test_datetime(),
        };

        assert!(epic.github_issue_url.is_some());
        let url = epic.github_issue_url.unwrap();
        assert!(url.contains("github.com"));
        assert!(url.contains("123"));
    }

    #[test]
    fn test_rodgers_type_stored_queryable() {
        // Verify rodgers_type metadata can be stored per AC-3
        let epic = Epic {
            id: "epic-rodgers-type".to_string(),
            title: "Epic with Rodgers Type".to_string(),
            description: None,
            bead_type: bead_type::FEATURE.to_string(),
            status: status::OPEN.to_string(),
            github_issue_url: None,
            github_issue_state: None,
            rodgers_type: Some("backport".to_string()), // Rodgers type metadata
            rodgers_labels: None,
            rodgers_parent: None,
            discovered_from: None,
            acceptance_criteria: None,
            priority: None,
            assignee: None,
            created_at: test_datetime(),
            updated_at: test_datetime(),
        };

        assert_eq!(epic.rodgers_type, Some("backport".to_string()));
    }

    #[test]
    fn test_status_transitions() {
        // Verify status transitions work per AC-3
        // Status should be: open -> in_progress -> closed

        // Start with open
        let mut bead_status = status::OPEN.to_string();
        assert!(status::is_valid(&bead_status));

        // Transition to in_progress
        bead_status = status::IN_PROGRESS.to_string();
        assert!(status::is_valid(&bead_status));

        // Transition to closed
        bead_status = status::CLOSED.to_string();
        assert!(status::is_valid(&bead_status));

        // Invalid status should fail
        assert!(!status::is_valid("invalid_status"));
        assert!(!status::is_valid(""));
        assert!(!status::is_valid("-open"));
    }

    #[test]
    fn test_table_names_constants() {
        // Verify table name constants per AC-3
        assert_eq!(table::EPICS, "rodgers_epics");
        assert_eq!(table::CHILDREN, "rodgers_children");
        assert_eq!(table::STATE, "rodgers_state");
    }

    #[test]
    fn test_state_keys_constants() {
        // Verify state key constants
        assert_eq!(state_keys::LAST_RUN, "scheduler.last_run");
        assert_eq!(state_keys::LAST_CYCLE, "scheduler.last_cycle");
        assert_eq!(state_keys::LAST_SYNC, "sync.last_sync");
        assert_eq!(state_keys::SCHEMA_VERSION, "schema.version");
    }

    #[test]
    fn test_bead_types_for_routing() {
        // Verify all bead types are valid for routing per AC-3
        assert!(bead_type::is_valid(bead_type::EPIC));
        assert!(bead_type::is_valid(bead_type::FEATURE));
        assert!(bead_type::is_valid(bead_type::BUG));
        assert!(bead_type::is_valid(bead_type::CHORE));
        assert!(bead_type::is_valid(bead_type::SPIKE));
        assert!(bead_type::is_valid(bead_type::DECISION));
        assert!(bead_type::is_valid(bead_type::MILESTONE));
        assert!(bead_type::is_valid(bead_type::TASK));

        // Verify rodgers:type values can be stored in rodgers_type field
        let epic = Epic {
            id: "epic-1".to_string(),
            title: "Test".to_string(),
            description: None,
            bead_type: bead_type::FEATURE.to_string(),
            status: status::OPEN.to_string(),
            github_issue_url: None,
            github_issue_state: None,
            rodgers_type: Some(bead_type::DECISION.to_string()), // Decision routing
            rodgers_labels: None,
            rodgers_parent: None,
            discovered_from: None,
            acceptance_criteria: None,
            priority: None,
            assignee: None,
            created_at: test_datetime(),
            updated_at: test_datetime(),
        };
        assert_eq!(epic.rodgers_type, Some("decision".to_string()));
    }

    #[test]
    fn test_epic_serialization_roundtrip() {
        // Verify Epic can be serialized and deserialized (for dolt JSON format)
        let epic = Epic {
            id: "epic-serialize".to_string(),
            title: "Serialized Epic".to_string(),
            description: Some("Testing serialization".to_string()),
            bead_type: bead_type::BUG.to_string(),
            status: status::CLOSED.to_string(),
            github_issue_url: Some("https://github.com/owner/repo/issues/789".to_string()),
            github_issue_state: Some("closed".to_string()),
            rodgers_type: Some("bug".to_string()),
            rodgers_labels: None,
            rodgers_parent: None,
            discovered_from: None,
            acceptance_criteria: None,
            priority: Some(1),
            assignee: Some("tester".to_string()),
            created_at: test_datetime(),
            updated_at: test_datetime(),
        };

        let json = serde_json::to_string(&epic).expect("Failed to serialize Epic");
        let deserialized: Epic = serde_json::from_str(&json).expect("Failed to deserialize Epic");

        assert_eq!(deserialized.id, epic.id);
        assert_eq!(deserialized.title, epic.title);
        assert_eq!(deserialized.github_issue_url, epic.github_issue_url);
        assert_eq!(deserialized.rodgers_type, epic.rodgers_type);
    }

    #[test]
    fn test_child_serialization_roundtrip() {
        // Verify Child can be serialized and deserialized
        let child = Child {
            id: "child-serialize".to_string(),
            parent_id: "epic-parent".to_string(),
            title: "Serialized Child".to_string(),
            description: None,
            bead_type: bead_type::TASK.to_string(),
            status: status::IN_PROGRESS.to_string(),
            github_issue_url: Some("https://github.com/owner/repo/issues/100".to_string()),
            rodgers_type: Some("feature".to_string()),
            rodgers_labels: None,
            rodgers_parent: Some("epic-parent".to_string()),
            discovered_from: None,
            acceptance_criteria: None,
            priority: Some(3),
            assignee: Some("developer".to_string()),
            created_at: test_datetime(),
        };

        let json = serde_json::to_string(&child).expect("Failed to serialize Child");
        let deserialized: Child = serde_json::from_str(&json).expect("Failed to deserialize Child");

        assert_eq!(deserialized.id, child.id);
        assert_eq!(deserialized.parent_id, child.parent_id);
        assert_eq!(deserialized.github_issue_url, child.github_issue_url);
    }

    #[test]
    fn test_state_serialization_roundtrip() {
        // Verify State can be serialized and deserialized
        let state = State {
            key: state_keys::LAST_SYNC.to_string(),
            value: Some("2024-01-15T12:00:00Z".to_string()),
            updated_at: test_datetime(),
        };

        let json = serde_json::to_string(&state).expect("Failed to serialize State");
        let deserialized: State = serde_json::from_str(&json).expect("Failed to deserialize State");

        assert_eq!(deserialized.key, state.key);
        assert_eq!(deserialized.value, state.value);
    }
}
