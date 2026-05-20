//! Beads (bd) client for filing and updating beads.
//!
//! Rodgers tracks work via beads stored in a local Dolt database. This module
//! provides a builder-pattern client for creating and updating beads.

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::RogersError;

// ---------------------------------------------------------------------------
// Bead client
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct BeadClient {
    title: Option<String>,
    description: Option<String>,
    bead_type: Option<String>,
    status: Option<String>,
    tag: Option<String>,
    acceptance: Option<String>,
    parent_id: Option<String>,
    priority: Option<u8>,
    assignee: Option<String>,
    deps: Vec<String>,
    external_ref: Option<String>,
}

impl BeadClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn file_bead(mut self, title: &str, description: &str, bead_type: &str) -> Self {
        self.title = Some(title.to_string());
        self.description = Some(description.to_string());
        self.bead_type = Some(bead_type.to_string());
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    pub fn with_status(mut self, status: &str) -> Self {
        self.status = Some(status.to_string());
        self
    }

    pub fn with_acceptance(mut self, acceptance: &str) -> Self {
        self.acceptance = Some(acceptance.to_string());
        self
    }

    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_id = Some(parent_id.to_string());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_assignee(mut self, assignee: &str) -> Self {
        self.assignee = Some(assignee.to_string());
        self
    }

    /// Add a dependency for the bead (e.g. "discovered-from:#42").
    ///
    /// May be called multiple times to add multiple dependencies.
    pub fn with_deps(mut self, deps: &str) -> Self {
        self.deps.push(deps.to_string());
        self
    }

    /// Set an external reference (e.g. "gh-42" for GitHub PR #42).
    pub fn with_external_ref(mut self, external_ref: &str) -> Self {
        self.external_ref = Some(external_ref.to_string());
        self
    }

    /// Submit the bead to the bd database.
    ///
    /// Currently stubs the actual bd invocation until the GitHub API client
    /// integration is in place. The stub logs the bead params for verification.
    pub async fn submit(self) -> Result<BeadResult, RogersError> {
        info!(
            "Filing bead: {} (type={}) priority={:?}",
            self.title.as_ref().unwrap_or(&"<untitled>".to_string()),
            self.bead_type.as_ref().unwrap_or(&"unknown".to_string()),
            self.priority,
        );

        // Delegate to the real `bd create` CLI invocation
        let args = build_bd_create_args(&self);
        let output = std::process::Command::new("bd")
            .args(&args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    RogersError::Beads(
                        "bd binary not found on PATH. Install beads and ensure it is on PATH."
                            .into(),
                    )
                } else {
                    RogersError::Beads(e.to_string())
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("bd create returned non-zero: {}", stderr);
            return Err(RogersError::Beads(format!("bd create failed: {}", stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("bd create succeeded: {}", stdout.trim());

        parse_bd_result(&stdout)
    }
}

/// Build the argument list for `bd create`.
fn build_bd_create_args(client: &BeadClient) -> Vec<String> {
    let mut args = vec!["create".to_string()];

    if let Some(ref title) = client.title {
        args.push("--title".to_string());
        args.push(title.clone());
    }

    if let Some(ref description) = client.description {
        args.push("--description".to_string());
        args.push(description.clone());
    }

    if let Some(ref t) = client.bead_type {
        args.push("--type".to_string());
        args.push(t.clone());
    }

    if let Some(ref status) = client.status {
        args.push("--status".to_string());
        args.push(status.clone());
    }

    if let Some(ref tag) = client.tag {
        args.push("--tag".to_string());
        args.push(tag.clone());
    }

    if let Some(ref acceptance) = client.acceptance {
        args.push("--acceptance".to_string());
        args.push(acceptance.clone());
    }

    if let Some(ref parent) = client.parent_id {
        args.push("--parent".to_string());
        args.push(parent.clone());
    }

    if let Some(priority) = client.priority {
        args.push("--priority".to_string());
        args.push(priority.to_string());
    }

    if let Some(ref assignee) = client.assignee {
        args.push("--assignee".to_string());
        args.push(assignee.clone());
    }

    if !client.deps.is_empty() {
        args.push("--deps".to_string());
        args.push(client.deps.join(","));
    }

    if let Some(ref external_ref) = client.external_ref {
        args.push("--external-ref".to_string());
        args.push(external_ref.clone());
    }

    args
}

/// Parse `{id, title, url, created_at}` from bd stdout.
fn parse_bd_result(stdout: &str) -> Result<BeadResult, RogersError> {
    // bd may output JSON or human-readable text. Try JSON first.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(stdout) {
        return Ok(BeadResult {
            id: v
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            url: v
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            title: v
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string(),
            created_at: v
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }

    // Fallback: extract ID from lines like "Created as foo-1" or "id: foo-1"
    let id = stdout
        .lines()
        .find_map(|line| {
            if line.starts_with("Created as ") {
                Some(line.trim_start_matches("Created as ").trim().to_string())
            } else if line.starts_with("id: ") {
                Some(line.trim_start_matches("id: ").trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(BeadResult {
        id,
        url: String::new(),
        title: String::new(),
        created_at: String::new(),
    })
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadResult {
    pub id: String,
    pub url: String,
    pub title: String,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_bd_args_basic() {
        let client = BeadClient::new()
            .file_bead("Test bead", "Description", "chore")
            .with_tag("rodgers:type=backport")
            .with_priority(1);

        let args = build_bd_create_args(&client);
        assert!(args.contains(&"--title".to_string()));
        assert!(args.contains(&"Test bead".to_string()));
        assert!(args.contains(&"--type".to_string()));
        assert!(args.contains(&"chore".to_string()));
        assert!(args.contains(&"--tag".to_string()));
        assert!(args.contains(&"--priority".to_string()));
        assert!(args.contains(&"1".to_string()));
    }

    #[test]
    fn test_build_bd_args_with_parent() {
        let client = BeadClient::new()
            .file_bead("Child", "child desc", "feature")
            .with_parent("epic-42");

        let args = build_bd_create_args(&client);
        assert!(args.contains(&"--parent".to_string()));
        assert!(args.contains(&"epic-42".to_string()));
    }

    #[test]
    fn test_parse_bd_json_result() {
        let json = r#"{"id":"bead-1","title":"Test","url":"https://beads.local/bead-1","created_at":"2024-01-01T00:00:00Z"}"#;
        let result = parse_bd_result(json).unwrap();
        assert_eq!(result.id, "bead-1");
        assert_eq!(result.url, "https://beads.local/bead-1");
    }

    #[test]
    fn test_parse_bd_text_result_created_as() {
        let text = "Created as bead-42\nSome other output";
        let result = parse_bd_result(text).unwrap();
        assert_eq!(result.id, "bead-42");
    }
}
