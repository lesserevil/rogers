//! Repository Settings audit check.
//!
//! Verifies that the repository's settings meet Rodgers' operational
//! requirements:
//! - Blocker: Main branch has branch protection rules enabled
//! - Warn: Delete branches on merge (recommended on)
//! - Warn: Default branch is 'main'
//!
//! Note: "Allow issue developers to modify labels" is not exposed by
//! the GitHub REST API (only visible in the web UI), so it is skipped.

use crate::checks::{CheckResult, Fixability, InitCheck, Severity};
use crate::error::{Result, RogersError};
use crate::github::GitHubClient;

/// Check for repository settings compliance.
pub struct RepoSettingsCheck;

impl InitCheck for RepoSettingsCheck {
    fn name(&self) -> &'static str {
        "repo_settings"
    }

    async fn check(
        &self,
        github: &GitHubClient,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();

        // Fetch repository settings.
        let repository = github.get_repository(owner, repo).await?;

        // ─── Check 1: Default branch is 'main' ───
        if repository.default_branch != "main" {
            results.push(CheckResult {
                severity: Severity::Warn,
                description: format!(
                    "Default branch is '{}' (expected 'main')",
                    repository.default_branch
                ),
                fixability: Fixability::Manual,
                fix_instructions: Some(format!(
                    "Change the default branch to 'main' at \
                     https://github.com/{}/{}/settings",
                    owner, repo
                )),
            });
        }

        // ─── Check 2: Delete branches on merge ───
        // GitHub API returns `delete_branch_on_merge` in the repo response.
        if !repository.delete_branch_on_merge.unwrap_or(false) {
            results.push(CheckResult {
                severity: Severity::Warn,
                description: "Delete branches on merge is not enabled".to_string(),
                fixability: Fixability::Manual,
                fix_instructions: Some(format!(
                    "Enable 'Delete branches on merge' at \
                     https://github.com/{}/{}/settings",
                    owner, repo
                )),
            });
        }

        // ─── Check 3: Main branch has branch protection ───
        // Try to get branch protection for the default branch.
        // If it returns 404, branch protection is not enabled → Blocker.
        match github
            .get_branch_protection(owner, repo, &repository.default_branch)
            .await
        {
            Ok(_) => {
                // Branch protection is enabled → Info.
                results.push(CheckResult {
                    severity: Severity::Info,
                    description: format!(
                        "Branch protection enabled for '{}'",
                        repository.default_branch
                    ),
                    fixability: Fixability::NotApplicable,
                    fix_instructions: None,
                });
            }
            Err(e) => {
                let is_404 = matches!(&e, RogersError::GitHubStatus { code, .. } if *code == 404);
                if is_404 {
                    // No branch protection → Blocker.
                    results.push(CheckResult {
                        severity: Severity::Blocker,
                        description: format!(
                            "No branch protection rules for '{}'",
                            repository.default_branch
                        ),
                        fixability: Fixability::Manual,
                        fix_instructions: Some(format!(
                            "Enable branch protection at \
                             https://github.com/{}/{}/settings/branches",
                            owner, repo
                        )),
                    });
                } else {
                    // Some other error (e.g., 403 for permissions) — propagate.
                    return Err(e);
                }
            }
        }

        Ok(results)
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

    // ─── Helpers ─────────────────────────────────────────────────────────

    fn make_repo_json(default_branch: &str, delete_branch_on_merge: bool) -> serde_json::Value {
        serde_json::json!({
            "id": 1,
            "name": "test-repo",
            "full_name": "test-owner/test-repo",
            "html_url": "https://github.com/test-owner/test-repo",
            "default_branch": default_branch,
            "delete_branch_on_merge": delete_branch_on_merge,
            "private": false,
            "has_issues": true,
            "has_wiki": false,
            "has_discussions": true,
            "size": 1024,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-02T00:00:00Z",
            "pushed_at": "2024-01-03T00:00:00Z",
            "visibility": "public"
        })
    }

    // ─── Test: All settings good ───────────────────────────────────────

    #[tokio::test]
    async fn test_all_settings_good() {
        let server = MockServer::start().await;

        // Repo settings: default branch is main, delete on merge enabled.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(make_repo_json("main", true)))
            .mount(&server)
            .await;

        // Branch protection: enabled (not 404).
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/branches/main/protection"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://api.github.com/repos/test-owner/test-repo/branches/main/protection",
                "required_pull_request_reviews": {
                    "dismiss_stale_reviews": true,
                    "require_code_owner_reviews": false,
                    "required_approving_review_count": 1
                },
                "allow_force_pushes": false,
                "allow_deletions": false
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = RepoSettingsCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("main"));
        assert!(results[0].description.contains("protection"));
        assert_eq!(results[0].fixability, Fixability::NotApplicable);
    }

    // ─── Test: Missing branch protection (blocker) ─────────────────────

    #[tokio::test]
    async fn test_missing_branch_protection_is_blocker() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(make_repo_json("main", true)))
            .mount(&server)
            .await;

        // Branch protection returns 404.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/branches/main/protection"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({ "message": "Not Found" })),
            )
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = RepoSettingsCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Blocker);
        assert!(results[0].description.contains("No branch protection"));
        assert_eq!(results[0].fixability, Fixability::Manual);
        assert!(
            results[0]
                .fix_instructions
                .as_deref()
                .unwrap()
                .contains("https://github.com/test-owner/test-repo/settings/branches")
        );
    }

    // ─── Test: Default branch is not main (warn) ───────────────────────

    #[tokio::test]
    async fn test_default_branch_not_main() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(make_repo_json("develop", true)))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/branches/develop/protection",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://api.github.com/repos/test-owner/test-repo/branches/develop/protection"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = RepoSettingsCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 2);

        // First result should be the default branch warning.
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(
            results[0]
                .description
                .contains("develop")
                .then(|| true)
                .unwrap_or(false)
        );
        assert_eq!(results[0].fixability, Fixability::Manual);

        // Second result should be the branch protection info.
        assert_eq!(results[1].severity, Severity::Info);
        assert!(results[1].description.contains("protection"));
    }

    // ─── Test: Delete branches on merge disabled (warn) ────────────────

    #[tokio::test]
    async fn test_delete_branches_on_merge_disabled() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(make_repo_json("main", false)))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/branches/main/protection"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://api.github.com/repos/test-owner/test-repo/branches/main/protection"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = RepoSettingsCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 2);

        // First result: delete on merge warning.
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("Delete branches on merge"));
        assert_eq!(results[0].fixability, Fixability::Manual);

        // Second result: branch protection info.
        assert_eq!(results[1].severity, Severity::Info);
    }

    // ─── Test: All settings bad (blocker + 2 warns) ───────────────────

    #[tokio::test]
    async fn test_all_settings_bad() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(make_repo_json("develop", false)),
            )
            .mount(&server)
            .await;

        // Branch protection 404.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/branches/develop/protection",
            ))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({ "message": "Not Found" })),
            )
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = RepoSettingsCheck;
        let results = check
            .check(&client, "test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(results.len(), 3);

        // 1. Default branch warn.
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("develop"));

        // 2. Delete branches on merge warn.
        assert_eq!(results[1].severity, Severity::Warn);
        assert!(results[1].description.contains("Delete branches on merge"));

        // 3. No branch protection blocker.
        assert_eq!(results[2].severity, Severity::Blocker);
        assert!(results[2].description.contains("No branch protection"));
    }

    // ─── Test: Check name ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_check_name() {
        let check = RepoSettingsCheck;
        assert_eq!(check.name(), "repo_settings");
    }

    // ─── Test: Branch protection returns non-404 error ─────────────────

    #[tokio::test]
    async fn test_branch_protection_forbidden_propagates_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(make_repo_json("main", true)))
            .mount(&server)
            .await;

        // Branch protection returns 403 (permission denied).
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/branches/main/protection"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "message": "Requires admin to access branch protection"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = RepoSettingsCheck;
        let result = check.check(&client, "test-owner", "test-repo").await;

        // Should return an error, not a CheckResult.
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let RogersError::GitHubStatus { code, .. } = err {
            assert_eq!(code, 403);
        } else {
            panic!("Expected GitHubStatus 403 error, got {:?}", err);
        }
    }
}
