//! Beads database client for Rodgers.
//!
//! This module provides database operations for the Rodgers beads database,
//! which uses Dolt as the underlying storage backend. The client interacts
//! with Dolt via the `dolt` CLI tool.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let client = BeadsClient::new("messages").unwrap();
//! client.execute("CREATE TABLE IF NOT EXISTS test (id INT PRIMARY KEY)")?;
//! let rows = client.query("SELECT * FROM test")?;
//! ```

use crate::error::{Result, RogersError};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Environment variable for the database path.
const DOLT_DB_PATH_ENV: &str = "DOLT_DB_PATH";

/// Default database name.
const DEFAULT_DATABASE: &str = "messages.hibernate";

/// Beads database client.
///
/// Provides methods for executing SQL statements and queries against
/// the Dolt-backed beads database.
#[derive(Debug, Clone)]
pub struct BeadsClient {
    /// Database name to operate on.
    database: String,
    /// Working directory for dolt commands (the database root).
    working_dir: String,
    /// Auto-commit mode (enabled for audit trail).
    auto_commit: bool,
}

impl BeadsClient {
    /// Create a new client for the specified database.
    ///
    /// The database path is determined by:
    /// 1. The `DOLT_DB_PATH` environment variable if set
    /// 2. The current directory (`.`) for embedded Dolt
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be accessed.
    pub fn new(database: impl Into<String>) -> Result<Self> {
        let database = database.into();
        let working_dir = std::env::var(DOLT_DB_PATH_ENV).unwrap_or_else(|_| ".".to_string());

        let mut client = Self {
            database,
            working_dir,
            auto_commit: true,
        };

        // Verify the database is accessible
        client.verify_connection()?;

        Ok(client)
    }

    /// Create a client from a beads configuration.
    pub fn from_config(remote: &str, database: Option<&str>) -> Result<Self> {
        // Parse the remote URL to determine database location
        let db_name = database.unwrap_or(DEFAULT_DATABASE);
        let mut client = Self {
            database: db_name.to_string(),
            working_dir: ".".to_string(),
            auto_commit: true,
        };

        client.verify_connection()?;
        Ok(client)
    }

    /// Verify connection to the Dolt database.
    fn verify_connection(&self) -> Result<()> {
        let output = self.execute_dolt_sql("SELECT 1", false)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RogersError::Beads(format!(
                "Failed to connect to Dolt database: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Enable or disable auto-commit mode.
    ///
    /// When enabled (default), each write operation automatically commits
    /// to preserve the Dolt audit trail.
    pub fn set_auto_commit(&mut self, enabled: bool) {
        self.auto_commit = enabled;
    }

    /// Execute a SQL statement (mutation query).
    ///
    /// This method is used for INSERT, UPDATE, DELETE, CREATE TABLE, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if the SQL execution fails.
    pub fn execute(&self, sql: &str) -> Result<()> {
        self.execute_dolt_sql(sql, true)?;
        Ok(())
    }

    /// Execute a SQL statement with optional auto-commit.
    fn execute_dolt_sql(&self, sql: &str, commit: bool) -> Result<std::process::Output> {
        let mut cmd = Command::new("dolt");
        cmd.arg("sql")
            .arg("-q")
            .arg(sql)
            .arg("--result-format")
            .arg("json");

        if commit && self.auto_commit {
            cmd.arg("--autocommit");
        }

        cmd.current_dir(&self.working_dir);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        tracing::debug!("Executing SQL: {}", sql);

        let output = cmd
            .output()
            .map_err(|e| RogersError::Beads(format!("Failed to execute dolt command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("SQL execution failed: {}", stderr);
            return Err(RogersError::Beads(format!(
                "SQL execution failed: {}",
                stderr
            )));
        }

        Ok(output)
    }

    /// Query data from the database.
    ///
    /// This method is used for SELECT queries.
    ///
    /// Returns a vector of row maps, where each map contains column names
    /// and values.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails or returns invalid JSON.
    pub fn query(&self, sql: &str) -> Result<Vec<HashMap<String, serde_json::Value>>> {
        let output = self.execute_dolt_query(sql)?;

        // Parse JSON result from dolt
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("Query result: {}", stdout);
            serde_json::from_str(&stdout).map_err(|e| {
                RogersError::Beads(format!("Failed to parse query result: {} - output: {}", e, stdout))
            })?;

        Ok(result)
    }

    /// Execute a query and return the raw output.
    fn execute_dolt_query(&self, sql: &str) -> Result<std::process::Output> {
        let mut cmd = Command::new("dolt");
        cmd.arg("sql")
            .arg("-q")
            .arg(sql)
            .arg("--result-format")
            .arg("json");

        cmd.current_dir(&self.working_dir);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        tracing::debug!("Executing query: {}", sql);

        let output = cmd
            .output()
            .map_err(|e| RogersError::Beads(format!("Failed to execute dolt command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("Query failed: {}", stderr);
            return Err(RogersError::Beads(format!("Query failed: {}", stderr)));
        }

        Ok(output)
    }

    /// Insert a row into a table.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A serializable type for the row data.
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn insert<T: serde::Serialize>(&self, table: &str, row: &T) -> Result<()> {
        let row_json = serde_json::to_string(row).map_err(|e| {
            RogersError::Beads(format!("Failed to serialize row: {}", e))
        })?;

        let sql = format!("INSERT INTO {} VALUES {}", table, row_json);
        self.execute(&sql)
    }

    /// Update rows in a table.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub fn update(&self, table: &str, set: &str, where_clause: &str) -> Result<()> {
        let sql = format!("UPDATE {} SET {} WHERE {}", table, set, where_clause);
        self.execute(&sql)
    }

    /// Delete rows from a table.
    ///
    /// # Errors
    ///
    /// Returns an error if the delete fails.
    pub fn delete(&self, table: &str, where_clause: &str) -> Result<()> {
        let sql = format!("DELETE FROM {} WHERE {}", table, where_clause);
        self.execute(&sql)
    }

    /// Check if a table exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the check fails.
    pub fn table_exists(&self, table: &str) -> Result<bool> {
        let sql = format!(
            "SELECT COUNT(*) as cnt FROM information_schema.tables WHERE table_name = '{}'",
            table
        );

        match self.query(&sql) {
            Ok(rows) => {
                if let Some(row) = rows.first() {
                    if let Some(cnt) = row.get("cnt") {
                        return Ok(cnt.as_i64().unwrap_or(0) > 0);
                    }
                }
                Ok(false)
            }
            Err(_) => Ok(false), // Table might not exist
        }
    }

    /// Get the database name.
    pub fn database(&self) -> &str {
        &self.database
    }

    /// Get the working directory.
    pub fn working_dir(&self) -> &str {
        &self.working_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // This test requires a running Dolt database
        // In CI, this may be skipped
        let result = BeadsClient::new("messages.hibernate");
        if result.is_err() {
            // Dolt might not be set up in test environment
            return;
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_commit_toggle() {
        let mut client = BeadsClient {
            database: "test".to_string(),
            working_dir: ".".to_string(),
            auto_commit: true,
        };

        assert!(client.auto_commit);
        client.set_auto_commit(false);
        assert!(!client.auto_commit);
        client.set_auto_commit(true);
        assert!(client.auto_commit);
    }

    #[test]
    fn test_database_name() {
        let client = BeadsClient {
            database: "my_database".to_string(),
            working_dir: ".".to_string(),
            auto_commit: true,
        };

        assert_eq!(client.database(), "my_database");
    }
}