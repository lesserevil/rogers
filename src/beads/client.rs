//! Bead client for filing and managing beads via bd CLI.
//!
//! This module provides the interface for creating and managing beads
//! through the bd (beads) CLI tool.

use crate::error::{RogersError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Metadata tag for rodgers-specific routing.
pub const RODGERS_TAG_DOCS: &str = "rodgers:type=docs";

/// Bead types recognized by Rodgers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BeadType {
    Bug,
    Chore,
    Feature,
    Epic,
}

impl BeadType {
    fn as_str(&self) -> &'static str {
        match self {
            BeadType::Bug => "bug",
            BeadType::Chore => "chore",
            BeadType::Feature => "feature",
            BeadType::Epic => "epic",
        }
    }
}

/// Response from a bead creation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadCreateResponse {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub created_at: Option<String>,
}

/// Response from listing beads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadInfo {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub bead_type: String,
    pub status: String,
    pub tags: Option<Vec<String>>,
    pub linked_issue: Option<u64>,
    pub created_at: Option<String>,
}

/// Client for interacting with beads via the bd CLI.
pub struct BeadClient {
    // bd CLI is used directly; no persistent state needed
}

impl BeadClient {
    /// Create a new BeadClient.
    pub fn new() -> Self {
        Self {}
    }

    /// File a doc-gap chore bead with rodgers:type=docs.
    ///
    /// This creates a chore bead that tracks documentation work
    /// for external contributors who will write the missing doc.
    ///
    /// # Arguments
    /// * `title` - Bead title (should be "Answer question: [restatement]")
    /// * `description` - Full question text + context + acceptance criteria
    /// * `discovered_from_issue` - Link to the originating GitHub issue
    /// * `acceptance` - What constitutes completion (new doc section answering question)
    ///
    /// # Returns
    /// The created bead information on success.
    pub fn file_doc_gap_bead(
        &self,
        title: &str,
        description: &str,
        discovered_from_issue: &str,
        acceptance: &str,
    ) -> Result<BeadCreateResponse> {
        self.file_bead(
            title,
            description,
            BeadType::Chore,
            Some(RODGERS_TAG_DOCS),
            Some(discovered_from_issue),
            Some(acceptance),
        )
    }

    /// Generic bead filing via bd CLI.
    ///
    /// # Arguments
    /// * `title` - Bead title
    /// * `description` - Full bead description
    /// * `bead_type` - Type of bead to create
    /// * `tag` - Optional rodgers:type tag for routing
    /// * `parent` - Optional parent epic ID
    /// * `acceptance` - Acceptance criteria
    fn file_bead(
        &self,
        title: &str,
        description: &str,
        bead_type: BeadType,
        tag: Option<&str>,
        parent: Option<&str>,
        acceptance: Option<&str>,
    ) -> Result<BeadCreateResponse> {
        let mut cmd = Command::new("bd");
        cmd.arg("create");

        // Title
        cmd.arg("--title").arg(title);

        // Description (use heredoc-style inline)
        cmd.arg("--description").arg(description);

        // Type
        cmd.arg("--type").arg(bead_type.as_str());

        // Tag
        if let Some(tag) = tag {
            cmd.arg("--add-label").arg(tag);
        }

        // Priority (default to 2 - medium)
        cmd.arg("--priority").arg("2");

        // Parent epic link (discovered-from)
        if let Some(parent) = parent {
            cmd.arg("--add-label").arg(format!("discovered-from={}", parent));
        }

        // Acceptance criteria
        if let Some(acceptance) = acceptance {
            cmd.arg("--acceptance").arg(acceptance);
        }

        let output = cmd.output().map_err(|e| {
            RogersError::Beads(format!("failed to execute bd create: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RogersError::Beads(format!(
                "bd create failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_bead_create_response(&stdout)
    }

    /// List beads with optional filters.
    pub fn list_beads(
        &self,
        status: Option<&str>,
        bead_type: Option<&str>,
        tag: Option<&str>,
    ) -> Result<Vec<BeadInfo>> {
        let mut cmd = Command::new("bd");
        cmd.arg("ls");

        if let Some(status) = status {
            cmd.arg("--status").arg(status);
        }

        if let Some(bead_type) = bead_type {
            cmd.arg("--type").arg(bead_type);
        }

        if let Some(tag) = tag {
            cmd.arg("--label").arg(tag);
        }

        let output = cmd.output().map_err(|e| {
            RogersError::Beads(format!("failed to execute bd ls: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RogersError::Beads(format!("bd ls failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_bead_list(&stdout)
    }

    /// Close a bead by ID.
    pub fn close_bead(&self, id: &str) -> Result<()> {
        let output = Command::new("bd")
            .arg("update")
            .arg(id)
            .arg("--status")
            .arg("closed")
            .output()
            .map_err(|e| {
                RogersError::Beads(format!("failed to execute bd update: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RogersError::Beads(format!(
                "bd close failed for {}: {}",
                id, stderr
            )));
        }

        Ok(())
    }

    /// Get a single bead by ID.
    pub fn get_bead(&self, id: &str) -> Result<BeadInfo> {
        let output = Command::new("bd")
            .arg("show")
            .arg(id)
            .output()
            .map_err(|e| {
                RogersError::Beads(format!("failed to execute bd show: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RogersError::Beads(format!(
                "bd show failed for {}: {}",
                id, stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_bead_show(&stdout)
    }
}

impl Default for BeadClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse bd create output to extract bead info.
/// bd create outputs something like:
/// "Created bead 'id' in database message.hibernate.
/// ID: oompah-zlz_2-4jq
/// Title: Answer question: How does X work?"
fn parse_bead_create_response(output: &str) -> Result<BeadCreateResponse> {
    let id = extract_field(output, "ID:")
        .ok_or_else(|| RogersError::Beads("missing ID in bd create output".into()))?;

    let title = extract_field(output, "Title:")
        .ok_or_else(|| RogersError::Beads("missing Title in bd create output".into()))?;

    Ok(BeadCreateResponse {
        id,
        title,
        url: None, // bd create doesn't output URL in stdout
        created_at: None,
    })
}

/// Parse bd ls output to extract bead list.
fn parse_bead_list(output: &str) -> Result<Vec<BeadInfo>> {
    // bd ls outputs table format:
    // ID | Title | Type | Status | Labels | Linked Issue | Created
    // -----|------|------|--------|--------|--------------|--------
    // ...

    let mut beads = Vec::new();
    let lines: Vec<&str> = output.lines().collect();

    for line in lines {
        // Skip header rows and separators
        if line.starts_with("ID") || line.starts_with("---") || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            beads.push(BeadInfo {
                id: parts[0].trim().to_string(),
                title: parts[1].trim().to_string(),
                bead_type: parts[2].trim().to_string(),
                status: parts[3].trim().to_string(),
                tags: None,
                linked_issue: None,
                created_at: None,
            });
        }
    }

    Ok(beads)
}

/// Parse bd show output to extract bead info.
fn parse_bead_show(output: &str) -> Result<BeadInfo> {
    let id = extract_field(output, "ID:")
        .ok_or_else(|| RogersError::Beads("missing ID in bd show output".into()))?;

    let title = extract_field(output, "Title:")
        .ok_or_else(|| RogersError::Beads("missing Title in bd show output".into()))?;

    let bead_type = extract_field(output, "Type:")
        .unwrap_or_else(|| "chore".to_string());

    let status = extract_field(output, "Status:")
        .unwrap_or_else(|| "open".to_string());

    Ok(BeadInfo {
        id,
        title,
        bead_type,
        status,
        tags: None,
        linked_issue: None,
        created_at: None,
    })
}

/// Extract a field value from bd command output.
/// Looks for lines like "Field: value" and returns "value".
fn extract_field(output: &str, field_prefix: &str) -> Option<String> {
    for line in output.lines() {
        if line.starts_with(field_prefix) {
            let value = line.trim_start_matches(field_prefix).trim();
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_type_as_str() {
        assert_eq!(BeadType::Bug.as_str(), "bug");
        assert_eq!(BeadType::Chore.as_str(), "chore");
        assert_eq!(BeadType::Feature.as_str(), "feature");
        assert_eq!(BeadType::Epic.as_str(), "epic");
    }

    #[test]
    fn test_extract_field() {
        let output = "ID: oompah-zlz_2-4jq\nTitle: Test bead\nStatus: open";
        assert_eq!(extract_field(output, "ID:"), Some("oompah-zlz_2-4jq".to_string()));
        assert_eq!(extract_field(output, "Title:"), Some("Test bead".to_string()));
        assert_eq!(extract_field(output, "Status:"), Some("open".to_string()));
        assert_eq!(extract_field(output, "Missing:"), None);
    }

    #[test]
    fn test_parse_bead_create_response() {
        let output = "Created bead in database.\nID: test-123\nTitle: Test Bead";
        let result = parse_bead_create_response(output);
        assert!(result.is_ok());
        let bead = result.unwrap();
        assert_eq!(bead.id, "test-123");
        assert_eq!(bead.title, "Test Bead");
    }

    #[test]
    fn test_parse_bead_list() {
        let output = r#"ID | Title | Type | Status | Labels | Linked Issue | Created
-----|------|------|--------|--------|--------------|--------
abc-1 | Test 1 | chore | open | rodgers:type=docs | 42 | 2024-01-01
def-2 | Test 2 | bug | closed | | | 2024-01-02"#;
        let result = parse_bead_list(output);
        assert!(result.is_ok());
        let beads = result.unwrap();
        assert_eq!(beads.len(), 2);
        assert_eq!(beads[0].id, "abc-1");
        assert_eq!(beads[0].title, "Test 1");
        assert_eq!(beads[1].id, "def-2");
    }
}