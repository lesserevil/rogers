//! Issue Templates audit check.
//!
//! Verifies that `.github/ISSUE_TEMPLATE/` directory exists and contains
//! at least one template file (.yml, .yaml, .md).

use crate::checks::{CheckResult, Fixability, InitCheck, Severity};
use crate::error::{Result, RogersError};
use crate::github::GitHubClient;

/// Check for issue templates in `.github/ISSUE_TEMPLATE/`.
pub struct IssueTemplatesCheck;

// Template file extensions accepted by GitHub.
const TEMPLATE_EXTENSIONS: &[&str] = &["yml", "yaml", "md"];

impl InitCheck for IssueTemplatesCheck {
    fn name(&self) -> &'static str {
        "issue_templates"
    }

    async fn check(
        &self,
        github: &GitHubClient,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<CheckResult>> {
        let directory_path = ".github/ISSUE_TEMPLATE";

        // Try to list the directory contents via the GitHub Contents API.
        // If the API returns 404, the directory doesn't exist.
        match github.list_directory(owner, repo, directory_path).await {
            Err(e) => {
                // If the error is a 404, the directory doesn't exist → Blocker.
                let is_404 = matches!(&e, RogersError::GitHubStatus { code, .. } if *code == 404);
                if is_404 {
                    return Ok(vec![CheckResult {
                        severity: Severity::Blocker,
                        description: format!(
                            "`.github/ISSUE_TEMPLATE/` directory not found in {}/{}",
                            owner, repo
                        ),
                        fixability: Fixability::Manual,
                        fix_instructions: Some(format!(
                            "Create the directory and add template files at \
                             https://github.com/{}/{}/new/main/.github/ISSUE_TEMPLATE/",
                            owner, repo
                        )),
                    }]);
                }
                // For other errors, propagate them.
                Err(e)
            }
            Ok(items) => {
                // Filter items that are files with template extensions.
                let template_files: Vec<&serde_json::Value> = items
                    .iter()
                    .filter(|item| {
                        // Must be a file (not a directory).
                        item.get("type")
                            .and_then(|t| t.as_str())
                            == Some("file")
                            // Extension must be one of our accepted types.
                            && item
                                .get("name")
                                .and_then(|n| n.as_str())
                                .is_some_and(|name| {
                                    let ext = name.rsplit('.').next().unwrap_or("");
                                    TEMPLATE_EXTENSIONS.contains(&ext)
                                })
                    })
                    .collect();

                if template_files.is_empty() {
                    // Directory exists but has no template files → Warn.
                    return Ok(vec![CheckResult {
                        severity: Severity::Warn,
                        description: format!(
                            "`.github/ISSUE_TEMPLATE/` directory exists in {}/{} \
                             but contains no `.yml`, `.yaml`, or `.md` template files",
                            owner, repo
                        ),
                        fixability: Fixability::Manual,
                        fix_instructions: Some(format!(
                            "Add issue template files (`.yml`, `.yaml`, or `.md`) to \
                             https://github.com/{}/{}/new/main/.github/ISSUE_TEMPLATE/",
                            owner, repo
                        )),
                    }]);
                }

                // Templates found → Info.
                let file_names: Vec<String> = template_files
                    .iter()
                    .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
                    .map(String::from)
                    .collect();
                Ok(vec![CheckResult {
                    severity: Severity::Info,
                    description: format!(
                        "Found {} issue template(s) in {}/{}: {}",
                        file_names.len(),
                        owner,
                        repo,
                        file_names.join(", ")
                    ),
                    fixability: Fixability::NotApplicable,
                    fix_instructions: None,
                }])
            }
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

    /// Test: no directory returns blocker.
    #[tokio::test]
    async fn test_no_directory_returns_blocker() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Not Found",
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Blocker);
        assert!(results[0].description.contains("not found"));
        assert_eq!(results[0].fixability, Fixability::Manual);
        assert!(results[0]
            .fix_instructions
            .as_deref()
            .unwrap()
            .contains("github.com/test-owner/test-repo"));
    }

    #[tokio::test]
    async fn test_empty_directory_returns_warn() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("no `.yml`"));
        assert_eq!(results[0].fixability, Fixability::Manual);
        assert!(results[0].fix_instructions.is_some());
    }

    #[tokio::test]
    async fn test_directory_with_only_subdirs_returns_warn() {
        let server = MockServer::start().await;

        // Directory has config.yml (a template) and a screenshots dir
        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "name": "config.yml",
                    "path": ".github/ISSUE_TEMPLATE/config.yml",
                    "sha": "abc123",
                    "type": "file",
                    "size": 128
                },
                {
                    "name": "screenshots",
                    "path": ".github/ISSUE_TEMPLATE/screenshots",
                    "sha": "def456",
                    "type": "dir",
                    "size": 0
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        // config.yml is a .yml file, so it should find templates → Info
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("1 issue template"));
    }

    #[tokio::test]
    async fn test_directory_with_dirs_and_non_template_files_returns_warn() {
        let server = MockServer::start().await;

        // Directory has subdirectories and non-template files — no .yml/.yaml/.md
        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "name": "screenshots",
                    "path": ".github/ISSUE_TEMPLATE/screenshots",
                    "sha": "def456",
                    "type": "dir",
                    "size": 0
                },
                {
                    "name": "readme.txt",
                    "path": ".github/ISSUE_TEMPLATE/readme.txt",
                    "sha": "ghi789",
                    "type": "file",
                    "size": 64
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("no `.yml`"));
    }

    #[tokio::test]
    async fn test_has_templates_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "name": "bug_report.yml",
                    "path": ".github/ISSUE_TEMPLATE/bug_report.yml",
                    "sha": "abc123",
                    "type": "file",
                    "size": 512
                },
                {
                    "name": "feature_request.md",
                    "path": ".github/ISSUE_TEMPLATE/feature_request.md",
                    "sha": "def456",
                    "type": "file",
                    "size": 256
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("2 issue template(s)"));
        assert!(results[0].description.contains("bug_report.yml"));
        assert!(results[0].description.contains("feature_request.md"));
        assert_eq!(results[0].fixability, Fixability::NotApplicable);
    }

    #[tokio::test]
    async fn test_yaml_extension_accepted() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "name": "template.yaml",
                    "path": ".github/ISSUE_TEMPLATE/template.yaml",
                    "sha": "abc123",
                    "type": "file",
                    "size": 256
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("template.yaml"));
    }

    #[tokio::test]
    async fn test_non_template_extensions_ignored() {
        let server = MockServer::start().await;

        // Directory has .txt and .json files, but no template files
        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "name": "readme.txt",
                    "path": ".github/ISSUE_TEMPLATE/readme.txt",
                    "sha": "abc123",
                    "type": "file",
                    "size": 128
                },
                {
                    "name": "schema.json",
                    "path": ".github/ISSUE_TEMPLATE/schema.json",
                    "sha": "def456",
                    "type": "file",
                    "size": 256
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("no `.yml`"));
    }

    #[tokio::test]
    async fn test_mixed_entries_with_templates() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "repos/test-owner/test-repo/contents/.github/ISSUE_TEMPLATE",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "name": ".markdownlint.json",
                    "path": ".github/ISSUE_TEMPLATE/.markdownlint.json",
                    "sha": "000",
                    "type": "file",
                    "size": 64
                },
                {
                    "name": "bug_report.yml",
                    "path": ".github/ISSUE_TEMPLATE/bug_report.yml",
                    "sha": "abc123",
                    "type": "file",
                    "size": 512
                },
                {
                    "name": "screenshots",
                    "path": ".github/ISSUE_TEMPLATE/screenshots",
                    "sha": "def456",
                    "type": "dir",
                    "size": 0
                },
                {
                    "name": "feature_request.md",
                    "path": ".github/ISSUE_TEMPLATE/feature_request.md",
                    "sha": "789xyz",
                    "type": "file",
                    "size": 256
                }
            ])))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = IssueTemplatesCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("2 issue template(s)"));
        // Should NOT include .markdownlint.json or screenshots directory
        assert!(results[0].description.contains("bug_report.yml"));
        assert!(results[0].description.contains("feature_request.md"));
    }
}
