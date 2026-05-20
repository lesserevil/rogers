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
    pub const VALID_TYPES: &[&str] = &[
        EPIC, FEATURE, BUG, CHORE, SPIKE, DECISION, MILESTONE, TASK,
    ];

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