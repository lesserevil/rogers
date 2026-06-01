//! Shared Backlog.md task types used by workflow managers.

use serde::{Deserialize, Serialize};

/// Schema version for the in-repo task metadata contract.
pub const SCHEMA_VERSION: i32 = 1;

/// Task status values used internally by workflow managers.
pub mod status {
    pub const OPEN: &str = "open";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const CLOSED: &str = "closed";

    pub const VALID_STATUSES: &[&str] = &[OPEN, IN_PROGRESS, CLOSED];

    pub fn is_valid(status: &str) -> bool {
        VALID_STATUSES.contains(&status)
    }
}

/// Task type values.
pub mod task_type {
    pub const EPIC: &str = "epic";
    pub const FEATURE: &str = "feature";
    pub const BUG: &str = "bug";
    pub const CHORE: &str = "chore";
    pub const SPIKE: &str = "spike";
    pub const DECISION: &str = "decision";
    pub const MILESTONE: &str = "milestone";
    pub const TASK: &str = "task";

    pub const VALID_TYPES: &[&str] = &[EPIC, FEATURE, BUG, CHORE, SPIKE, DECISION, MILESTONE, TASK];

    pub fn is_valid(type_: &str) -> bool {
        VALID_TYPES.contains(&type_)
    }
}

/// Top-level task representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epic {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub task_type: String,
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

/// Child task representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Child {
    pub id: String,
    pub parent_id: String,
    pub title: String,
    pub description: Option<String>,
    pub task_type: String,
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

/// State row retained for compatibility with workflow code that records scheduler state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub key: String,
    pub value: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
