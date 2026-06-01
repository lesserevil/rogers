//! GitHub API response models.
//!
//! This module contains all the types needed to deserialize GitHub REST API and GraphQL responses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// GitHub User/Actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: i64,
    #[serde(rename = "node_id")]
    pub node_id: Option<String>,
    #[serde(rename = "avatar_url")]
    pub avatar_url: Option<String>,
    #[serde(rename = "html_url")]
    pub html_url: Option<String>,
    #[serde(rename = "type")]
    pub user_type: Option<String>,
}

/// GitHub Label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    #[serde(default)]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    #[serde(rename = "node_id")]
    pub node_id: Option<String>,
}

/// Issue comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    #[serde(rename = "node_id")]
    pub node_id: Option<String>,
    pub body: String,
    pub user: User,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "author_association")]
    pub author_association: Option<String>,
}

/// GitHub Issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub number: i32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: User,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub milestone: Option<Milestone>,
    pub comments: i32,
    #[serde(rename = "closed_at")]
    pub closed_at: Option<DateTime<Utc>>,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "pull_request")]
    pub pull_request: Option<PullRequestInfo>,
    #[serde(rename = "node_id")]
    pub node_id: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "html_url")]
    pub html_url: Option<String>,
}

impl Issue {
    /// Returns true if this is actually a pull request
    pub fn is_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }
}

/// Minimal PR info (present on PR issues)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub url: Option<String>,
    #[serde(rename = "html_url")]
    pub html_url: Option<String>,
    pub merged_at: Option<DateTime<Utc>>,
    pub merged_by: Option<User>,
}

/// GitHub Milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub number: i32,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    #[serde(rename = "open_issues")]
    pub open_issues: i32,
    #[serde(rename = "closed_issues")]
    pub closed_issues: i32,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "closed_at")]
    pub closed_at: Option<DateTime<Utc>>,
    pub url: Option<String>,
}

/// GitHub Pull Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: i32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: User,
    pub labels: Vec<Label>,
    pub assignees: Vec<User>,
    pub milestone: Option<Milestone>,
    pub comments: i32,
    #[serde(rename = "commits")]
    pub commits: i32,
    #[serde(rename = "additions")]
    pub additions: i32,
    #[serde(rename = "deletions")]
    pub deletions: i32,
    #[serde(rename = "changed_files")]
    pub changed_files: i32,
    #[serde(rename = "closed_at")]
    pub closed_at: Option<DateTime<Utc>>,
    #[serde(rename = "merged_at")]
    pub merged_at: Option<DateTime<Utc>>,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "merge_commit_sha")]
    pub merge_commit_sha: Option<String>,
    #[serde(rename = "head")]
    pub head: RepoRef,
    #[serde(rename = "base")]
    pub base: RepoRef,
    #[serde(rename = "node_id")]
    pub node_id: Option<String>,
    pub url: Option<String>,
    #[serde(rename = "html_url")]
    pub html_url: Option<String>,
    pub draft: bool,
    #[serde(rename = "mergeable")]
    pub mergeable: Option<bool>,
}

/// Branch reference in a PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRef {
    pub ref_name: String,
    pub sha: String,
    pub repo: Repository,
}

/// Minimal repository info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub name: String,
    #[serde(rename = "node_id")]
    pub node_id: Option<String>,
    pub full_name: String,
    pub private: bool,
    #[serde(rename = "html_url")]
    pub html_url: Option<String>,
    pub description: Option<String>,
}

/// GitHub Release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: i64,
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "published_at")]
    pub published_at: Option<DateTime<Utc>>,
    pub author: User,
    pub url: Option<String>,
    #[serde(rename = "html_url")]
    pub html_url: Option<String>,
    #[serde(rename = "upload_url")]
    pub upload_url: Option<String>,
    pub target_commitish: Option<String>,
    #[serde(rename = "node_id")]
    pub node_id: Option<String>,
}

/// Rate limit response from GitHub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResource {
    pub limit: i32,
    pub remaining: i32,
    pub reset: i64,
    #[serde(rename = "used")]
    pub used: i32,
    #[serde(rename = "resource")]
    pub resource: Option<String>,
}

/// Rate limit response from GitHub API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResponse {
    #[serde(rename = "resources")]
    pub resources: Resources,
    #[serde(rename = "rate")]
    pub rate: RateLimitResource,
}

/// Resources section of rate limit response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resources {
    #[serde(rename = "core")]
    pub core: RateLimitResource,
    #[serde(rename = "search")]
    pub search: RateLimitResource,
    #[serde(rename = "graphql")]
    pub graphql: RateLimitResource,
}

// ---------------------------------------------------------------------------
// GraphQL types for Discussions
// ---------------------------------------------------------------------------

/// GraphQL response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

/// GraphQL error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Option<Vec<ErrorLocation>>,
    pub path: Option<Vec<serde_json::Value>>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub extensions: Option<ErrorExtensions>,
}

/// Error location in GraphQL query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLocation {
    pub line: i32,
    pub column: i32,
}

/// GraphQL error extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorExtensions {
    pub code: Option<String>,
    #[serde(rename = "typeName")]
    pub type_name: Option<String>,
    #[serde(rename = "fieldName")]
    pub field_name: Option<String>,
}

/// Discussion category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionCategory {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    #[serde(rename = "isAnswerable")]
    pub is_answerable: bool,
}

/// Discussion answer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionAnswer {
    pub id: String,
    pub body: String,
    #[serde(rename = "body_html")]
    pub body_html: Option<String>,
    pub author: User,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    pub url: String,
    #[serde(rename = "comment_count")]
    pub comment_count: i32,
}

/// Single discussion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discussion {
    pub id: String,
    pub number: i32,
    pub title: String,
    pub body: String,
    #[serde(rename = "body_html")]
    pub body_html: Option<String>,
    #[serde(rename = "category")]
    pub category: DiscussionCategory,
    pub author: User,
    pub answer: Option<DiscussionAnswer>,
    #[serde(rename = "answer_chosen_at")]
    pub answer_chosen_at: Option<DateTime<Utc>>,
    #[serde(rename = "answer_chosen_by")]
    pub answer_chosen_by: Option<User>,
    pub labels: LabelConnection,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    pub url: String,
    #[serde(rename = "viewer_subscription")]
    pub viewer_subscription: Option<String>,
    #[serde(rename = "viewer_has_handed_of_previous_version")]
    pub viewer_has_heard_of_previous_version: Option<bool>,
    #[serde(rename = "Comments")]
    pub comments: Option<CommentConnection>,
}

/// Discussion comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionComment {
    pub id: String,
    pub body: String,
    #[serde(rename = "body_html")]
    pub body_html: Option<String>,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updated_at")]
    pub updated_at: DateTime<Utc>,
    pub author: Option<User>,
    pub url: String,
    #[serde(rename = "comment_count")]
    pub comment_count: i32,
    #[serde(rename = "Replies")]
    pub replies: Option<CommentConnection>,
}

/// Connection type for labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelConnection {
    pub total_count: i32,
    pub nodes: Vec<Label>,
}

/// Connection type for comments/discussions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentConnection {
    pub total_count: i32,
    pub page_info: PageInfo,
    pub nodes: Vec<DiscussionComment>,
}

/// Page information for cursor pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    #[serde(rename = "start_cursor")]
    pub start_cursor: Option<String>,
    #[serde(rename = "end_cursor")]
    pub end_cursor: Option<String>,
    #[serde(rename = "has_next_page")]
    pub has_next_page: bool,
    #[serde(rename = "has_previous_page")]
    pub has_previous_page: bool,
}

// ---------------------------------------------------------------------------
// List response wrappers
// ---------------------------------------------------------------------------

/// Paginated response wrapper for REST API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    #[serde(rename = "total_count")]
    pub total_count: Option<i32>,
    #[serde(rename = "incomplete_results")]
    pub incomplete_results: Option<bool>,
    #[serde(rename = "data")]
    #[serde(default)]
    pub data: Vec<T>,
}

impl<T> ListResponse<T> {
    /// Returns the items in this list response
    pub fn into_items(self) -> Vec<T> {
        self.data
    }
}

impl<T> Default for ListResponse<T> {
    fn default() -> Self {
        Self {
            total_count: None,
            incomplete_results: None,
            data: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

/// Issue creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub body: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
    pub milestone: Option<i32>,
}

/// Issue update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub state: Option<String>,
    pub labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
    pub milestone: Option<i32>,
}

/// Discussion creation request (GraphQL input)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscussionInput {
    pub repository_id: String,
    pub category_id: String,
    pub title: String,
    pub body: Option<String>,
}

/// Discussion update input (GraphQL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDiscussionInput {
    pub id: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub category_id: Option<String>,
    pub state: Option<String>,
}

/// Webhook event payload for rate limit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitWebhook {
    #[serde(rename = "resources")]
    pub resources: Resources,
    #[serde(rename = "rate")]
    pub rate: RateLimitResource,
}

// ---------------------------------------------------------------------------
// Branches & Commits
// ---------------------------------------------------------------------------

/// GitHub repository branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    #[serde(rename = "commit")]
    pub commit: CommitRef,
    #[serde(rename = "protected")]
    pub r#protected: bool,
}

/// Minimal commit reference (used in branches, parents, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRef {
    pub sha: String,
    pub url: Option<String>,
}

/// Full GitHub commit object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    #[serde(rename = "commit")]
    pub commit_inner: CommitDetail,
    pub html_url: String,
    pub parents: Vec<CommitRef>,
}

/// Commit author/committer detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetail {
    pub message: String,
    pub author: CommitAuthor,
    #[serde(rename = "committer")]
    pub committer: CommitAuthor,
}

impl CommitDetail {
    /// First line of the commit message.
    pub fn message_short(&self) -> String {
        self.message
            .lines()
            .next()
            .unwrap_or(&self.message)
            .trim()
            .to_string()
    }
}

/// Commit author record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: Option<String>,
    pub date: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Compare
// ---------------------------------------------------------------------------

/// GitHub compare API response (two-dot diff between commits/refs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareResult {
    pub status: String,
    #[serde(rename = "ahead_by")]
    pub ahead_by: u32,
    #[serde(rename = "behind_by")]
    pub behind_by: u32,
    #[serde(rename = "total_commits")]
    pub total_commits: u32,
    pub commits: Vec<CompareCommit>,
    pub files: Option<Vec<String>>,
    pub diff: Option<String>,
}

/// Commit in a compare result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareCommit {
    pub sha: String,
    pub commit: CommitDetail,
    pub html_url: String,
}

// ---------------------------------------------------------------------------
// Reactions
// ---------------------------------------------------------------------------

/// A reaction on a discussion or comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub id: i64,
    pub content: String,
    #[serde(rename = "created_at")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "viewer_has_reacted")]
    pub viewer_has_reacted: bool,
    pub user: Option<User>,
}

/// Well-known reaction content strings.
pub mod reaction_content {
    /// Thumbs up emoji.
    pub const THUMBS_UP: &str = "+1";
    /// Thumbs down emoji.
    pub const THUMBS_DOWN: &str = "-1";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_issue() {
        let json = r#"{
            "number": 123,
            "title": "Test issue",
            "body": "Issue body",
            "state": "open",
            "user": {"login": "testuser", "id": 1},
            "labels": [{"id": 1, "name": "bug", "description": null, "color": null, "node_id": null}],
            "assignees": [],
            "milestone": null,
            "comments": 0,
            "closed_at": null,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "pull_request": null,
            "node_id": "I_kwDOA",
            "url": "https://api.github.com/repos/test/issue/123",
            "html_url": "https://github.com/test/issue/123"
        }"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 123);
        assert_eq!(issue.title, "Test issue");
        assert!(!issue.is_pull_request());
    }

    #[test]
    fn test_parse_pr() {
        let json = r#"{
            "number": 456,
            "title": "Test PR",
            "body": "PR body",
            "state": "open",
            "user": {"login": "testuser", "id": 1},
            "labels": [],
            "assignees": [],
            "milestone": null,
            "comments": 0,
            "commits": 1,
            "additions": 10,
            "deletions": 5,
            "changed_files": 2,
            "closed_at": null,
            "merged_at": null,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "merge_commit_sha": "abc123",
            "head": {"ref_name": "feature", "sha": "abc", "repo": {"id": 1, "name": "test", "node_id": null, "full_name": "test/repo", "private": false, "html_url": null, "description": null}},
            "base": {"ref_name": "main", "sha": "def", "repo": {"id": 1, "name": "test", "node_id": null, "full_name": "test/repo", "private": false, "html_url": null, "description": null}},
            "node_id": "PR_kwDOA",
            "url": "https://api.github.com/repos/test/pulls/456",
            "html_url": "https://github.com/test/pull/456",
            "draft": false,
            "mergeable": true
        }"#;
        let pr: PullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 456);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.commits, 1);
        assert!(pr.draft == false);
    }

    #[test]
    fn test_parse_release() {
        let json = r#"{
            "id": 1,
            "tag_name": "v1.0.0",
            "name": "Version 1.0.0",
            "body": "Release notes",
            "draft": false,
            "prerelease": false,
            "created_at": "2024-01-01T00:00:00Z",
            "published_at": "2024-01-01T00:00:00Z",
            "author": {"login": "testuser", "id": 1},
            "url": "https://api.github.com/repos/test/releases/1",
            "html_url": "https://github.com/test/releases/tag/v1.0.0",
            "upload_url": "https://uploads.github.com/releases/1",
            "target_commitish": "main",
            "node_id": "RE_kwDOA"
        }"#;
        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert!(release.draft == false);
    }

    #[test]
    fn test_parse_rate_limit() {
        let json = r#"{
            "resources": {
                "core": {"limit": 5000, "remaining": 4999, "reset": 1234567890, "used": 1, "resource": "core"},
                "search": {"limit": 30, "remaining": 30, "reset": 1234567890, "used": 0, "resource": "search"},
                "graphql": {"limit": 5000, "remaining": 4999, "reset": 1234567890, "used": 1, "resource": "graphql"}
            },
            "rate": {"limit": 5000, "remaining": 4999, "reset": 1234567890, "used": 1, "resource": null}
        }"#;
        let rate_limit: RateLimitResponse = serde_json::from_str(json).unwrap();
        assert_eq!(rate_limit.resources.core.limit, 5000);
        assert_eq!(rate_limit.resources.core.remaining, 4999);
    }

    #[test]
    fn test_list_response_default() {
        let list: ListResponse<String> = ListResponse::default();
        assert!(list.data.is_empty());
    }

    #[test]
    fn test_list_response_into_items() {
        let list = ListResponse {
            total_count: Some(2),
            incomplete_results: Some(false),
            data: vec!["a".to_string(), "b".to_string()],
        };
        let items = list.into_items();
        assert_eq!(items.len(), 2);
    }
}
