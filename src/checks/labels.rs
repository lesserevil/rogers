//! Required Labels audit check.
//!
//! Verifies that the repository has all required Rodgers labels defined.
//! Fetches labels via the GitHub API and compares against
//! `RODGERS_REQUIRED_LABELS` with case-insensitive matching.
//!
//! - Blocker if any required label is missing
//! - Auto-fixable: missing labels can be created via the GitHub API with `--fix`

use crate::checks::{CheckResult, Fixability, InitCheck, Severity};
use crate::error::Result;
use crate::github::GitHubClient;
use crate::labels::RODGERS_REQUIRED_LABELS;

/// Check for required Rodgers labels.
pub struct LabelsCheck;

impl InitCheck for LabelsCheck {
    fn name(&self) -> &'static str {
        "required_labels"
    }

    async fn check(&self, github: &GitHubClient, owner: &str, repo: &str) -> Result<CheckResult> {
        // Fetch all existing labels (handles pagination internally).
        let existing = github.list_labels(owner, repo).await?;

        // Build a case-insensitive set of existing label names.
        let existing_names: std::collections::HashSet<String> =
            existing.iter().map(|l| l.name.to_lowercase()).collect();

        // Find missing required labels.
        let missing: Vec<&str> = RODGERS_REQUIRED_LABELS
            .iter()
            .filter(|def| !existing_names.contains(&def.name.to_lowercase()))
            .map(|def| def.name)
            .collect();

        if missing.is_empty() {
            // All required labels are present → Info.
            let present: Vec<&str> = RODGERS_REQUIRED_LABELS.iter().map(|l| l.name).collect();
            Ok(CheckResult {
                severity: Severity::Info,
                description: format!(
                    "All {} required labels present: {}",
                    present.len(),
                    present.join(", ")
                ),
                fixability: Fixability::NotApplicable,
                fix_instructions: None,
            })
        } else {
            // Some labels are missing → Blocker with auto-fix.
            Ok(CheckResult {
                severity: Severity::Blocker,
                description: format!("Required labels missing: {}", missing.join(", ")),
                fixability: Fixability::Auto,
                fix_instructions: Some(format!(
                    "Running with `--fix` will create the following labels via the GitHub API:\n  {}",
                    missing.join("\n  ")
                )),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_client(server: &MockServer) -> GitHubClient {
        GitHubClient::new("").with_base_url(&server.uri())
    }

    /// Test: all required labels present → Info.
    #[tokio::test]
    async fn test_all_labels_present_returns_info() {
        let server = MockServer::start().await;

        let labels: Vec<serde_json::Value> = RODGERS_REQUIRED_LABELS
            .iter()
            .map(|l| {
                serde_json::json!({
                    "id": 100 + l.name.len() as u64,
                    "name": l.name,
                    "color": l.color,
                    "default": false,
                    "description": l.description,
                    "url": format!("https://api.github.com/repos/test/test/labels/{}", l.name)
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(labels))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = LabelsCheck;
        let result = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.severity, Severity::Info);
        assert!(
            result
                .description
                .contains("All")
                .then(|| true)
                .unwrap_or(false)
        );
        for label in RODGERS_REQUIRED_LABELS.iter() {
            assert!(
                result.description.contains(label.name),
                "description should contain label name '{}'",
                label.name
            );
        }
        assert_eq!(result.fixability, Fixability::NotApplicable);
        assert!(result.fix_instructions.is_none());
    }

    /// Test: some labels missing → Blocker with fix instructions.
    #[tokio::test]
    async fn test_some_labels_missing_returns_blocker() {
        let server = MockServer::start().await;

        // Only provide "bug" and "feature" labels — the rest are missing.
        let partial_labels: Vec<serde_json::Value> = vec![
            serde_json::json!({
                "id": 1,
                "name": "bug",
                "color": "d73a4a",
                "default": false,
                "description": "A bug report",
                "url": "https://api.github.com/repos/test/test/labels/bug"
            }),
            serde_json::json!({
                "id": 2,
                "name": "feature",
                "color": "a2eeef",
                "default": false,
                "description": "A feature request",
                "url": "https://api.github.com/repos/test/test/labels/feature"
            }),
        ];

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(partial_labels))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = LabelsCheck;
        let result = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.severity, Severity::Blocker);
        assert!(result.description.contains("missing"));
        assert_eq!(result.fixability, Fixability::Auto);
        assert!(result.fix_instructions.is_some());
        let fix_instructions = result.fix_instructions.unwrap();
        assert!(fix_instructions.contains("--fix"));
    }

    /// Test: no labels present → Blocker listing all required labels.
    #[tokio::test]
    async fn test_no_labels_returns_blocker_with_all_missing() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Vec::<serde_json::Value>::new()))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = LabelsCheck;
        let result = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.severity, Severity::Blocker);
        for label in RODGERS_REQUIRED_LABELS.iter() {
            assert!(
                result.description.contains(label.name),
                "missing description should list label '{}'",
                label.name
            );
        }
        assert_eq!(result.fixability, Fixability::Auto);
        assert!(result.fix_instructions.is_some());
    }

    /// Test: case-insensitive label matching.
    /// "BUG" should match "bug".
    #[tokio::test]
    async fn test_case_insensitive_matching() {
        let server = MockServer::start().await;

        // Provide labels with different casing.
        let case_labels: Vec<serde_json::Value> = RODGERS_REQUIRED_LABELS
            .iter()
            .map(|l| {
                let mut name = l.name.to_string();
                // Flip the case of some labels to test case-insensitive matching.
                if name == "bug" {
                    name = "BUG".to_string();
                } else if name == "feature" {
                    name = "Feature".to_string();
                }
                serde_json::json!({
                    "id": 100 + name.len() as u64,
                    "name": name,
                    "color": l.color,
                    "default": false,
                    "description": l.description,
                    "url": format!("https://api.github.com/repos/test/test/labels/{}", name)
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(case_labels))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = LabelsCheck;
        let result = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        // Even with different casing, all labels should be found → Info.
        assert_eq!(result.severity, Severity::Info);
        assert!(result.fixability == Fixability::NotApplicable);
    }

    /// Test: empty label name handling — shouldn't crash.
    #[tokio::test]
    async fn test_empty_response_handled() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = LabelsCheck;
        let result = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.severity, Severity::Blocker);
        assert_eq!(result.fixability, Fixability::Auto);
    }

    /// Test: check name returns correct string.
    #[tokio::test]
    async fn test_check_name() {
        let check = LabelsCheck;
        assert_eq!(check.name(), "required_labels");
    }

    /// Test: verify fix instructions mention the specific missing labels.
    #[tokio::test]
    async fn test_fix_instructions_mention_missing_labels() {
        let server = MockServer::start().await;

        // Only "bug" exists, everything else is missing.
        let labels = vec![serde_json::json!({
            "id": 1,
            "name": "bug",
            "color": "d73a4a",
            "default": false,
            "description": "A bug report",
            "url": "https://api.github.com/repos/test/test/labels/bug"
        })];

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(labels))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = LabelsCheck;
        let result = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        let fix_instructions = result.fix_instructions.unwrap();
        // Verify that "feature" (a missing label) appears in the fix instructions
        assert!(fix_instructions.contains("feature"));
        assert!(fix_instructions.contains("question"));
        assert!(fix_instructions.contains("needs-information"));
    }
}
