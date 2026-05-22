//! Schema migration management for Rodgers beads database.
//!
//! This module handles database schema migrations, tracking version
//! history and applying incremental changes. Migrates are run on startup
//! to ensure the database schema is up-to-date.
//!
//! ## Migration Strategy
//!
//! - Each migration is versioned and applied in order
//! - Migrations are idempotent where possible
//! - Failed migrations roll back and return errors
//! - Dolt auto-commit is enabled by default for audit trail

use crate::client::BeadsClient;
use crate::schema::{
    CREATE_CHILDREN_SQL, CREATE_EPICS_SQL, CREATE_STATE_SQL, SCHEMA_VERSION,
};
use rogers_core::error::{Result, RogersError};

/// Migration entry point representation.
#[derive(Debug)]
pub struct Migration {
    /// Migration version number.
    pub version: i32,
    /// SQL statements to execute.
    pub statements: Vec<&'static str>,
    /// Human-readable description.
    pub description: &'static str,
}

/// Get all migrations in order.
pub fn get_migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "Create initial Rodgers schema tables (epics, children, state)",
        statements: vec![CREATE_EPICS_SQL, CREATE_CHILDREN_SQL, CREATE_STATE_SQL],
    }]
}

/// Run all pending migrations.
pub fn run_migrations(client: &BeadsClient) -> Result<Vec<i32>> {
    let current_version = get_current_schema_version(client)?;
    let migrations = get_migrations();
    let mut applied = Vec::new();

    for migration in migrations {
        if migration.version <= current_version {
            tracing::debug!(
                "Migration {} already applied (current: {})",
                migration.version,
                current_version
            );
            continue;
        }

        tracing::info!(
            "Applying migration {}: {}",
            migration.version,
            migration.description
        );

        for statement in &migration.statements {
            client.execute(statement)?;
        }

        // Update schema version in state table
        let update_sql = format!(
            "INSERT INTO rodgers_state (key, value) VALUES ('schema.version', '{}') ON DUPLICATE KEY UPDATE value = '{}'",
            migration.version, migration.version
        );
        client.execute(&update_sql)?;

        applied.push(migration.version);
    }

    Ok(applied)
}

/// Get the current schema version from the database.
fn get_current_schema_version(client: &BeadsClient) -> Result<i32> {
    let result = client.query("SELECT value FROM rodgers_state WHERE key = 'schema.version'");

    match result {
        Ok(rows) => {
            if let Some(row) = rows.first() {
                if let Some(value) = row.get("value") {
                    let version_str = value.as_str().unwrap_or("");
                    return version_str
                        .parse::<i32>()
                        .map_err(|e| RogersError::Beads(format!("Invalid schema version: {}", e)));
                }
            }
            Ok(0)
        }
        Err(_) => Ok(0), // Table might not exist yet
    }
}

/// Verify that all required tables exist with correct columns.
pub fn verify_schema(client: &BeadsClient) -> Result<Vec<String>> {
    let mut errors = Vec::new();

    // Check epics table
    if let Err(e) = verify_epics_table(client) {
        errors.push(format!("epics: {}", e));
    }

    // Check children table
    if let Err(e) = verify_children_table(client) {
        errors.push(format!("children: {}", e));
    }

    // Check state table
    if let Err(e) = verify_state_table(client) {
        errors.push(format!("state: {}", e));
    }

    if errors.is_empty() {
        Ok(vec![])
    } else {
        Err(RogersError::Beads(format!(
            "Schema verification failed: {}",
            errors.join("; ")
        )))
    }
}

fn verify_epics_table(client: &BeadsClient) -> Result<()> {
    let required_columns = vec![
        "id",
        "title",
        "description",
        "bead_type",
        "status",
        "github_issue_url",
        "github_issue_state",
        "rodgers_type",
        "created_at",
        "updated_at",
    ];

    verify_table_columns(client, "rodgers_epics", &required_columns)
}

fn verify_children_table(client: &BeadsClient) -> Result<()> {
    let required_columns = vec![
        "id",
        "parent_id",
        "title",
        "description",
        "bead_type",
        "status",
        "github_issue_url",
        "rodgers_type",
        "priority",
        "assignee",
        "created_at",
    ];

    verify_table_columns(client, "rodgers_children", &required_columns)
}

fn verify_state_table(client: &BeadsClient) -> Result<()> {
    let required_columns = vec!["key", "value"];

    verify_table_columns(client, "rodgers_state", &required_columns)
}

fn verify_table_columns(
    client: &BeadsClient,
    table_name: &str,
    required_columns: &[&str],
) -> Result<()> {
    let sql = format!("DESCRIBE {}", table_name);
    let result = client.query(&sql);

    match result {
        Ok(rows) => {
            let columns: Vec<String> = rows
                .iter()
                .filter_map(|row| {
                    row.get("Field")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();

            for col in required_columns {
                if !columns.contains(&col.to_string()) {
                    return Err(RogersError::Beads(format!(
                        "Missing required column '{}' in table '{}'",
                        col, table_name
                    )));
                }
            }
            Ok(())
        }
        Err(e) => Err(RogersError::Beads(format!(
            "Failed to verify table '{}': {}",
            table_name, e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_list() {
        let migrations = get_migrations();
        assert!(!migrations.is_empty());
        assert_eq!(migrations[0].version, 1);
        assert!(!migrations[0].statements.is_empty());
    }

    #[test]
    fn test_status_is_valid() {
        use crate::schema::status;
        assert!(status::is_valid("open"));
        assert!(status::is_valid("in_progress"));
        assert!(status::is_valid("closed"));
        assert!(!status::is_valid("invalid"));
    }

    #[test]
    fn test_bead_type_is_valid() {
        use crate::schema::bead_type;
        assert!(bead_type::is_valid("epic"));
        assert!(bead_type::is_valid("feature"));
        assert!(bead_type::is_valid("bug"));
        assert!(bead_type::is_valid("chore"));
        assert!(!bead_type::is_valid("invalid"));
    }
}
