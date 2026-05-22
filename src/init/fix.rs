//! Auto-fix logic for `rogers init --fix`.
//!
//! Handles creating missing required labels and discussion categories via
//! the GitHub API.  Idempotent: safe to re-run (create-if-missing semantics).

use crate::checks::RELEASE_PROPOSALS_CATEGORY;
use crate::error::Result;
use crate::github::GitHubClient;
use crate::labels::RODGERS_REQUIRED_LABELS;

/// Result of an auto-fix operation — what was created and what was skipped.
#[derive(Debug, Clone)]
pub struct FixResult {
    /// Labels that were created by this run.
    pub created: Vec<String>,
    /// Labels that already existed (skipped).
    pub skipped: Vec<String>,
}

/// Ensures all required Rodgers labels exist on the repository.
///
/// Iterates over `RODGERS_REQUIRED_LABELS`, fetches existing labels from GitHub,
/// and creates any that are missing. If a label already exists, it is skipped.
///
/// This is idempotent: running multiple times produces the same result.
///
/// # Errors
/// - Returns an error if the GitHub API cannot be reached.
/// - Individual label creation failures are collected and reported rather than
///   failing the entire operation (tolerance for partial failures).
pub async fn ensure_labels(github: &GitHubClient, owner: &str, repo: &str) -> Result<FixResult> {
    // Fetch all existing labels from the repository.
    let existing = github.list_labels(owner, repo).await?;
    let existing_names: std::collections::HashSet<&str> =
        existing.iter().map(|l| l.name.as_str()).collect();

    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for label_def in RODGERS_REQUIRED_LABELS {
        if existing_names.contains(label_def.name) {
            skipped.push(label_def.name.to_string());
        } else {
            match github.create_label(owner, repo, label_def).await {
                Ok(_) => created.push(label_def.name.to_string()),
                Err(e) => {
                    // Log the error but continue with other labels.
                    // This handles permission errors, rate limits, etc.
                    eprintln!(
                        "Warning: failed to create label '{}': {}",
                        label_def.name, e
                    );
                }
            }
        }
    }

    Ok(FixResult { created, skipped })
}

/// Prints a human-readable report of label fix results.
pub fn print_fix_report(result: &FixResult) {
    if !result.created.is_empty() {
        println!(
            "Created {} label(s): {}",
            result.created.len(),
            result.created.join(", ")
        );
    }
    if !result.skipped.is_empty() {
        println!(
            "Skipped {} existing label(s): {}",
            result.skipped.len(),
            result.skipped.join(", ")
        );
    }
    if result.created.is_empty() && result.skipped.is_empty() {
        println!("No required labels were created or skipped (all existing labels present).");
    }
}

// ─── Discussion Categories ──────────────────────────────────────────────

/// Result of a discussion category fix operation.
#[derive(Debug, Clone)]
pub struct CategoryFixResult {
    /// Categories that were created by this run.
    pub created: Vec<String>,
    /// Categories that already existed (skipped).
    pub skipped: Vec<String>,
}

/// Ensures the Release Proposals discussion category exists.
///
/// Iterates over existing categories, creates `RELEASE_PROPOSALS_CATEGORY`
/// if it doesn't exist. Skips if it already exists.
///
/// This is idempotent: running multiple times produces the same result.
pub async fn ensure_discussion_category(
    github: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<CategoryFixResult> {
    let existing = github.list_discussion_categories(owner, repo).await?;

    let mut created = Vec::new();
    let mut skipped = Vec::new();

    if existing
        .iter()
        .any(|c| c.name == RELEASE_PROPOSALS_CATEGORY)
    {
        skipped.push(RELEASE_PROPOSALS_CATEGORY.to_string());
    } else {
        match github
            .create_discussion_category_idempotent(owner, repo, RELEASE_PROPOSALS_CATEGORY)
            .await
        {
            Ok(_) => created.push(RELEASE_PROPOSALS_CATEGORY.to_string()),
            Err(e) => {
                eprintln!(
                    "Warning: failed to create discussion category '{}': {}",
                    RELEASE_PROPOSALS_CATEGORY, e
                );
            }
        }
    }

    Ok(CategoryFixResult { created, skipped })
}

/// Prints a human-readable report of discussion category fix results.
pub fn print_category_fix_report(result: &CategoryFixResult) {
    if !result.created.is_empty() {
        println!(
            "Created {} category(ies): {}",
            result.created.len(),
            result.created.join(", ")
        );
    }
    if !result.skipped.is_empty() {
        println!(
            "Skipped {} existing category(ies): {}",
            result.skipped.len(),
            result.skipped.join(", ")
        );
    }
    if result.created.is_empty() && result.skipped.is_empty() {
        println!("No discussion categories were created or skipped.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::init_client::{CreateLabelRequest, Label};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_client(server: &MockServer) -> GitHubClient {
        GitHubClient::new("").with_base_url(&server.uri())
    }

    /// Test: all labels missing → all created.
    #[tokio::test]
    async fn test_all_labels_created_when_missing() {
        let server = MockServer::start().await;

        // Empty label list
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Vec::<serde_json::Value>::new()))
            .mount(&server)
            .await;

        // Setup dynamic POST mock that returns the label name from the request body.
        for label_def in RODGERS_REQUIRED_LABELS {
            let name = label_def.name.to_string();
            let response = serde_json::json!({
                "id": name.len() as u64 * 100,
                "name": &name,
                "color": label_def.color,
                "default": false,
                "description": label_def.description,
                "url": format!("https://api.github.com/repos/test/test/labels/{}", name)
            });
            Mock::given(method("POST"))
                .and(path("/repos/test-owner/test-repo/labels"))
                .respond_with(ResponseTemplate::new(201).set_body_json(response))
                .mount(&server)
                .await;
        }

        let client = make_client(&server);
        let result = ensure_labels(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.created.len(), RODGERS_REQUIRED_LABELS.len());
        assert_eq!(result.skipped.len(), 0);

        let expected_names: Vec<&str> = RODGERS_REQUIRED_LABELS.iter().map(|l| l.name).collect();
        for name in expected_names {
            assert!(result.created.contains(&name.to_string()));
        }
    }

    /// Test: all labels exist → all skipped.
    #[tokio::test]
    async fn test_all_labels_skipped_when_existing() {
        let server = MockServer::start().await;

        // All labels already exist
        let labels: Vec<Label> = RODGERS_REQUIRED_LABELS
            .iter()
            .map(|l| Label {
                id: (l.name.len() as u64) * 100,
                name: l.name.to_string(),
                color: l.color.to_string(),
                default: Some(false),
                description: Some(l.description.to_string()),
                url: format!("https://api.github.com/repos/test/test/labels/{}", l.name),
            })
            .collect();
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&labels))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let result = ensure_labels(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.created.len(), 0);
        assert_eq!(result.skipped.len(), RODGERS_REQUIRED_LABELS.len());
    }

    /// Test: partial labels exist → mixed result.
    #[tokio::test]
    async fn test_partial_labels_created() {
        let server = MockServer::start().await;

        // Only "bug" label exists
        let labels = vec![Label {
            id: 1,
            name: "bug".to_string(),
            color: "d73a4a".to_string(),
            default: Some(false),
            description: Some("A bug report".to_string()),
            url: "https://api.github.com/repos/test/test/labels/bug".to_string(),
        }];
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&labels))
            .mount(&server)
            .await;

        // Setup POST mock for the missing labels
        for label_def in RODGERS_REQUIRED_LABELS {
            if label_def.name != "bug" {
                let name = label_def.name.to_string();
                let response = serde_json::json!({
                    "id": name.len() as u64 * 100,
                    "name": &name,
                    "color": label_def.color,
                    "default": false,
                    "description": label_def.description,
                    "url": format!("https://api.github.com/repos/test/test/labels/{}", name)
                });
                Mock::given(method("POST"))
                    .and(path("/repos/test-owner/test-repo/labels"))
                    .respond_with(ResponseTemplate::new(201).set_body_json(response))
                    .mount(&server)
                    .await;
            }
        }

        let client = make_client(&server);
        let result = ensure_labels(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.created.len(), RODGERS_REQUIRED_LABELS.len() - 1);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(result.skipped[0], "bug");
        assert!(result.created.contains(&"feature".to_string()));
    }

    /// Test: idempotent — running twice with no changes in between gives same result.
    #[tokio::test]
    async fn test_idempotent_second_run_skips_all() {
        // First run: no labels exist.
        let server1 = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Vec::<serde_json::Value>::new()))
            .mount(&server1)
            .await;
        for label_def in RODGERS_REQUIRED_LABELS {
            let name = label_def.name.to_string();
            let response = serde_json::json!({
                "id": name.len() as u64 * 100,
                "name": &name,
                "color": label_def.color,
                "default": false,
                "description": label_def.description,
                "url": format!("https://api.github.com/repos/test/test/labels/{}", name)
            });
            Mock::given(method("POST"))
                .and(path("/repos/test-owner/test-repo/labels"))
                .respond_with(ResponseTemplate::new(201).set_body_json(response))
                .mount(&server1)
                .await;
        }
        let client1 = make_client(&server1);
        let result1 = ensure_labels(&client1, "test-owner", "test-repo")
            .await
            .unwrap();
        assert_eq!(result1.created.len(), RODGERS_REQUIRED_LABELS.len());
        assert_eq!(result1.skipped.len(), 0);

        // Second run: all labels now exist.
        let server2 = MockServer::start().await;
        let labels: Vec<Label> = RODGERS_REQUIRED_LABELS
            .iter()
            .map(|l| Label {
                id: (l.name.len() as u64) * 100,
                name: l.name.to_string(),
                color: l.color.to_string(),
                default: Some(false),
                description: Some(l.description.to_string()),
                url: format!("https://api.github.com/repos/test/test/labels/{}", l.name),
            })
            .collect();
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&labels))
            .mount(&server2)
            .await;

        let client2 = make_client(&server2);
        let result2 = ensure_labels(&client2, "test-owner", "test-repo")
            .await
            .unwrap();
        assert_eq!(result2.created.len(), 0);
        assert_eq!(result2.skipped.len(), RODGERS_REQUIRED_LABELS.len());
    }

    /// Test: canonical colors are applied correctly.
    #[tokio::test]
    async fn test_canonical_colors_applied() {
        let server = MockServer::start().await;

        // No labels exist
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Vec::<serde_json::Value>::new()))
            .mount(&server)
            .await;

        // Setup POST mocks
        for label_def in RODGERS_REQUIRED_LABELS {
            let name = label_def.name.to_string();
            let response = serde_json::json!({
                "id": name.len() as u64 * 100,
                "name": &name,
                "color": label_def.color,
                "default": false,
                "description": label_def.description,
                "url": format!("https://api.github.com/repos/test/test/labels/{}", name)
            });
            Mock::given(method("POST"))
                .and(path("/repos/test-owner/test-repo/labels"))
                .respond_with(ResponseTemplate::new(201).set_body_json(response))
                .mount(&server)
                .await;
        }

        let client = make_client(&server);
        let result = ensure_labels(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.created.len(), RODGERS_REQUIRED_LABELS.len());

        // Verify canonical colors are defined correctly in the source.
        let colors: std::collections::HashMap<&str, &str> = RODGERS_REQUIRED_LABELS
            .iter()
            .map(|l| (l.name, l.color))
            .collect();
        assert_eq!(colors.get("bug"), Some(&"d73a4a"));
        assert_eq!(colors.get("feature"), Some(&"a2eeef"));
        assert_eq!(colors.get("question"), Some(&"d876e3"));
        assert_eq!(colors.get("needs-information"), Some(&"PaleGreen"));
        assert_eq!(colors.get("needs-documentation"), Some(&"DBAB79"));
        assert_eq!(colors.get("ready-for-review"), Some(&"fbca04"));
        assert_eq!(colors.get("will-not-do"), Some(&"ff4444"));
        assert_eq!(colors.get("ready-for-work"), Some(&"238636"));
        assert_eq!(colors.get("in-progress"), Some(&"1a7f37"));
    }

    /// Test: report is printed correctly for created labels.
    #[tokio::test]
    async fn test_print_fix_report_created() {
        let result = FixResult {
            created: vec!["bug".to_string(), "feature".to_string()],
            skipped: vec![],
        };
        // Just verify it doesn't panic.
        print_fix_report(&result);
    }

    /// Test: report shows both created and skipped.
    #[tokio::test]
    async fn test_print_fix_report_mixed() {
        let result = FixResult {
            created: vec!["question".to_string()],
            skipped: vec!["bug".to_string(), "feature".to_string()],
        };
        print_fix_report(&result);
    }

    /// Test: verify CreateLabelRequest matches label definition.
    #[tokio::test]
    async fn test_create_label_request_matches_definition() {
        let label_def = &RODGERS_REQUIRED_LABELS[0];
        let request = CreateLabelRequest {
            name: label_def.name.to_string(),
            color: label_def.color.to_string(),
            description: Some(label_def.description.to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains(&format!("\"name\":\"{}\"", label_def.name)));
        assert!(json.contains(&format!("\"color\":\"{}\"", label_def.color)));
        assert!(json.contains(&format!("\"description\":\"{}\"", label_def.description)));
    }

    /// Test: label definition count matches required labels.
    #[tokio::test]
    async fn test_label_definition_count() {
        // 3 triage + 2 routing + 4 workflow = 9
        assert_eq!(RODGERS_REQUIRED_LABELS.len(), 9);
    }

    /// Test: label definitions include all three categories.
    #[tokio::test]
    async fn test_label_definitions_includes_all_categories() {
        let names: Vec<&str> = RODGERS_REQUIRED_LABELS.iter().map(|l| l.name).collect();

        // Triage classification
        assert!(names.contains(&"bug"));
        assert!(names.contains(&"feature"));
        assert!(names.contains(&"question"));

        // Routing state
        assert!(names.contains(&"needs-information"));
        assert!(names.contains(&"needs-documentation"));

        // Workflow state
        assert!(names.contains(&"ready-for-review"));
        assert!(names.contains(&"will-not-do"));
        assert!(names.contains(&"ready-for-work"));
        assert!(names.contains(&"in-progress"));
    }

    /// Test: ensure_discussion_category creates when missing.
    #[tokio::test]
    async fn test_ensure_discussion_category_creates_when_missing() {
        let server = MockServer::start().await;

        // No categories exist.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 0,
                "categories": []
            })))
            .mount(&server)
            .await;

        // Create succeeds.
        Mock::given(method("POST"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 999,
                "name": "Release Proposals",
                "description": "Propose new releases",
                "emoji": "🚀",
                "emoji_name": "rocket",
                "color": "0075ca",
                "is_answerable": false,
                "created_at": "2024-01-01T00:00:00Z",
                "repository_id": 123,
                "slug": "release-proposals"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let result = ensure_discussion_category(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.created, vec!["Release Proposals".to_string()]);
        assert!(result.skipped.is_empty());
    }

    /// Test: ensure_discussion_category skips when already exists.
    #[tokio::test]
    async fn test_ensure_discussion_category_skips_when_exists() {
        let server = MockServer::start().await;

        // Category already exists.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "categories": [{
                    "id": 789,
                    "name": "Release Proposals",
                    "description": "Propose new releases",
                    "emoji": "🚀",
                    "emoji_name": "rocket",
                    "color": "0075ca",
                    "is_answerable": false,
                    "created_at": "2024-01-01T00:00:00Z",
                    "repository_id": 123,
                    "slug": "release-proposals"
                }]
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let result = ensure_discussion_category(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.created.len(), 0);
        assert_eq!(result.skipped, vec!["Release Proposals".to_string()]);
    }

    /// Test: ensure_discussion_category idempotent — second run same as first.
    #[tokio::test]
    async fn test_ensure_discussion_category_idempotent_second_run() {
        // First run: missing → created.
        let server1 = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 0,
                "categories": []
            })))
            .mount(&server1)
            .await;
        Mock::given(method("POST"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 999,
                "name": "Release Proposals",
                "description": "Propose new releases",
                "emoji": "🚀",
                "emoji_name": "rocket",
                "color": "0075ca",
                "is_answerable": false,
                "created_at": "2024-01-01T00:00:00Z",
                "repository_id": 123,
                "slug": "release-proposals"
            })))
            .mount(&server1)
            .await;

        let client1 = make_client(&server1);
        let result1 = ensure_discussion_category(&client1, "test-owner", "test-repo")
            .await
            .unwrap();
        assert_eq!(result1.created, vec!["Release Proposals".to_string()]);
        assert!(result1.skipped.is_empty());

        // Second run: exists → skipped.
        let server2 = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "categories": [{
                    "id": 999,
                    "name": "Release Proposals",
                    "description": "Propose new releases",
                    "emoji": "🚀",
                    "emoji_name": "rocket",
                    "color": "0075ca",
                    "is_answerable": false,
                    "created_at": "2024-01-01T00:00:00Z",
                    "repository_id": 123,
                    "slug": "release-proposals"
                }]
            })))
            .mount(&server2)
            .await;

        let client2 = make_client(&server2);
        let result2 = ensure_discussion_category(&client2, "test-owner", "test-repo")
            .await
            .unwrap();
        assert!(result2.created.is_empty());
        assert_eq!(result2.skipped, vec!["Release Proposals".to_string()]);
    }

    /// Test: print_category_fix_report for created category.
    #[test]
    fn test_print_category_fix_report_created() {
        let result = CategoryFixResult {
            created: vec!["Release Proposals".to_string()],
            skipped: vec![],
        };
        print_category_fix_report(&result);
    }

    /// Test: print_category_fix_report for skipped category.
    #[test]
    fn test_print_category_fix_report_skipped() {
        let result = CategoryFixResult {
            created: vec![],
            skipped: vec!["Release Proposals".to_string()],
        };
        print_category_fix_report(&result);
    }

    /// Test: print_category_fix_report for empty result.
    #[test]
    fn test_print_category_fix_report_empty() {
        let result = CategoryFixResult {
            created: vec![],
            skipped: vec![],
        };
        print_category_fix_report(&result);
    }

    /// Test: ensure_discussion_category continues on error.
    #[tokio::test]
    async fn test_ensure_discussion_category_continues_on_error() {
        let server = MockServer::start().await;

        // No categories.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 0,
                "categories": []
            })))
            .mount(&server)
            .await;

        // Create fails with 403.
        Mock::given(method("POST"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "message": "Requires admin to create categories"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let result = ensure_discussion_category(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(result.created.len(), 0);
        assert_eq!(result.skipped.len(), 0);
    }
}
