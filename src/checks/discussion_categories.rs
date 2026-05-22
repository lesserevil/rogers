//! Discussion Categories audit check.
//!
//! Verifies that the repository has a GitHub Discussion category Rodgers can use
//! for release proposals.
//!
//! - Warn if no `Release Proposals` category exists
//! - Info if category exists

use crate::checks::{CheckResult, Fixability, InitCheck, Severity};
use crate::error::Result;
use crate::github::GitHubClient;

/// Name of the discussion category Rodgers uses for release proposals.
pub const RELEASE_PROPOSALS_CATEGORY: &str = "Release Proposals";

/// Check for the Release Proposals discussion category.
pub struct DiscussionCategoriesCheck;

impl InitCheck for DiscussionCategoriesCheck {
    fn name(&self) -> &'static str {
        "discussion_categories"
    }

    async fn check(
        &self,
        github: &GitHubClient,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<CheckResult>> {
        let categories = github.list_discussion_categories(owner, repo).await?;

        let category_names: Vec<&str> = categories.iter().map(|c| c.name.as_str()).collect();

        if category_names.contains(&RELEASE_PROPOSALS_CATEGORY) {
            // Category exists → Info.
            Ok(vec![CheckResult {
                severity: Severity::Info,
                description: format!(
                    "Discussion category \"{}\" exists in {}/{}",
                    RELEASE_PROPOSALS_CATEGORY, owner, repo
                ),
                fixability: Fixability::NotApplicable,
                fix_instructions: None,
            }])
        } else {
            // Category missing → Warn.
            Ok(vec![CheckResult {
                severity: Severity::Warn,
                description: format!(
                    "Discussion category \"{}\" not found in {}/{}",
                    RELEASE_PROPOSALS_CATEGORY, owner, repo
                ),
                fixability: Fixability::Auto,
                fix_instructions: Some(format!(
                    "Running with `--fix` will create the \"{}\" discussion category via the \
                     GitHub API.\n\n\
                     This category is used for Rodgers release proposal discussions.\n\n\
                     Manual creation: \
                     https://github.com/{}/{}/discussions/categories/new",
                    RELEASE_PROPOSALS_CATEGORY, owner, repo
                )),
            }])
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

    const OWNER: &str = "test-owner";
    const REPO: &str = "test-repo";

    /// Test: category exists → Info.
    #[tokio::test]
    async fn test_category_exists_returns_info() {
        let server = MockServer::start().await;

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
        let check = DiscussionCategoriesCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("Release Proposals"));
        assert!(results[0].description.contains("exists"));
        assert_eq!(results[0].fixability, Fixability::NotApplicable);
        assert!(results[0].fix_instructions.is_none());
    }

    /// Test: category missing → Warn.
    #[tokio::test]
    async fn test_category_missing_returns_warn() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 0,
                "categories": []
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = DiscussionCategoriesCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("Release Proposals"));
        assert!(results[0].description.contains("not found"));
        assert_eq!(results[0].fixability, Fixability::Auto);
        assert!(results[0].fix_instructions.is_some());
    }

    /// Test: other categories exist but not Release Proposals → Warn.
    #[tokio::test]
    async fn test_other_categories_exist_but_not_release_proposals() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 2,
                "categories": [
                    {
                        "id": 1,
                        "name": "General",
                        "description": "General discussion",
                        "emoji": "💬",
                        "emoji_name": "speech_balloon",
                        "color": "ededed",
                        "is_answerable": false,
                        "created_at": "2024-01-01T00:00:00Z",
                        "repository_id": 123,
                        "slug": "general"
                    },
                    {
                        "id": 2,
                        "name": "Show and Tell",
                        "description": "Show off your work",
                        "emoji": "🎉",
                        "emoji_name": "tada",
                        "color": "ededed",
                        "is_answerable": false,
                        "created_at": "2024-01-01T00:00:00Z",
                        "repository_id": 123,
                        "slug": "show-and-tell"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = DiscussionCategoriesCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("Release Proposals"));
        assert!(results[0].description.contains("not found"));
        assert_eq!(results[0].fixability, Fixability::Auto);
    }

    /// Test: check name returns correct string.
    #[tokio::test]
    async fn test_check_name() {
        let check = DiscussionCategoriesCheck;
        assert_eq!(check.name(), "discussion_categories");
    }

    /// Test: case-sensitive category matching (Release Proposals != release proposals).
    #[tokio::test]
    async fn test_case_sensitive_matching() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "categories": [{
                    "id": 1,
                    "name": "release proposals",
                    "description": "Wrong case",
                    "emoji": "",
                    "emoji_name": "",
                    "color": "ededed",
                    "is_answerable": false,
                    "created_at": "2024-01-01T00:00:00Z",
                    "repository_id": 123,
                    "slug": "release-proposals"
                }]
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = DiscussionCategoriesCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        // "release proposals" != "Release Proposals" → should be Warn
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
    }

    /// Test: fix instructions mention --fix flag.
    #[tokio::test]
    async fn test_fix_instructions_mention_fix_flag() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/discussion-categories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 0,
                "categories": []
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = DiscussionCategoriesCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        let instructions = results[0].fix_instructions.as_ref().unwrap();
        assert!(instructions.contains("--fix"));
        assert!(instructions.contains("Release Proposals"));
    }
}
