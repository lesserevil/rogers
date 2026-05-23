//! GitHub API client.
//!
//! A thin wrapper around reqwest for GitHub REST API and GraphQL operations.
//! Provides consistent authentication, rate limiting, and error handling.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{Result, RogersError};
use crate::github::auth::{AuthError, GitHubAuth};
use crate::github::models::*;
use crate::github::rate_limit::RateLimitHandler;

/// Figuratively "GitHub" but shorter for the module namespace.
type GhResult<T> = Result<T>;

/// GitHub API client for REST and GraphQL operations.
#[derive(Debug, Clone)]
pub struct GitHubClient {
    /// HTTP client for making requests.
    client: Client,
    /// Authentication configuration.
    auth: GitHubAuth,
    /// Rate limit handler.
    rate_limit: RateLimitHandler,
    /// Repository owner.
    owner: String,
    /// Repository name.
    repo: String,
}

impl GitHubClient {
    /// Create a new GitHubClient from configuration.
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, auth: GitHubAuth) -> Self {
        let client = Client::builder()
            .user_agent("Rodgers/0.1.0 (GitHub-native community relations agent)")
            .build()
            .expect("valid reqwest client");

        Self {
            client,
            auth,
            rate_limit: RateLimitHandler::new(),
            owner: owner.into(),
            repo: repo.into(),
        }
    }

    /// Get the rate limit handler.
    pub fn rate_limit_handler(&self) -> &RateLimitHandler {
        &self.rate_limit
    }

    /// Get a mutable reference to the rate limit handler.
    pub fn rate_limit_handler_mut(&mut self) -> &mut RateLimitHandler {
        &mut self.rate_limit
    }

    /// Get the configured owner.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Get the configured repo.
    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Get the GitHub API base URL.
    pub fn api_base(&self) -> &str {
        self.auth.api_url()
    }

    /// Get the authentication token, if set.
    pub fn token(&self) -> Option<&str> {
        Some(self.auth.token())
    }

    /// Get the underlying HTTP client for making raw requests.
    #[allow(dead_code)]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get the authentication configuration.
    pub fn auth(&self) -> &GitHubAuth {
        &self.auth
    }

    /// Build the base API URL for REST endpoints.
    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.auth.api_url(), path)
    }

    /// Build the GitHub GraphQL URL.
    fn graphql_url(&self) -> String {
        format!("{}{}", self.auth.api_url(), "/graphql")
    }

    /// Build the repository-specific API URL.
    fn repo_url(&self, path: &str) -> String {
        self.api_url(&format!("/repos/{}/{}{}", self.owner, self.repo, path))
    }

    /// Execute a request and handle rate limiting with retry.
    async fn execute<T: for<'de> Deserialize<'de>>(
        &mut self,
        request: reqwest::RequestBuilder,
    ) -> GhResult<T> {
        let mut attempts = 0u32;
        let max_retries = self.rate_limit.max_retries();

        loop {
            attempts += 1;

            let response = request.try_clone().unwrap().send().await?;
            let status = response.status();
            let headers = response.headers().clone();

            // Handle rate limiting
            if status.as_u16() == 429 {
                // Get retry-after header if present
                let retry_after = headers
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok());

                if self.rate_limit.should_retry(attempts) {
                    let delay = self.rate_limit.calculate_delay(attempts, retry_after);
                    tracing::warn!(
                        "Rate limited (429). Attempt {}/{}, waiting {:?} before retry.",
                        attempts,
                        max_retries,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                } else {
                    return Err(RogersError::GitHubStatus {
                        code: 429,
                        message: "Rate limit exceeded after max retries".to_string(),
                    });
                }
            }

            // Handle auth errors - fail fast
            if status.as_u16() == 401 || status.as_u16() == 403 {
                if let Some(warning) = self.rate_limit.get_warning_message() {
                    tracing::warn!("{}", warning);
                }
                return Err(RogersError::GitHubStatus {
                    code: status.as_u16(),
                    message: format!(
                        "Authorization failed: {}",
                        status.canonical_reason().unwrap_or("Unknown")
                    ),
                });
            }

            // Handle 404 - not found, return gracefully
            if status.as_u16() == 404 {
                return Err(RogersError::GitHubStatus {
                    code: 404,
                    message: "Resource not found".to_string(),
                });
            }

            // Update rate limit info from headers
            if let Some(remaining) = headers
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<i32>().ok())
            {
                if let Some(reset) = headers
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<i64>().ok())
                {
                    self.rate_limit.update_from_headers(remaining, reset);
                }
            }

            // Log warning if rate limit is low
            if let Some(warning) = self.rate_limit.get_warning_message() {
                tracing::warn!("{}", warning);
            }

            // Parse response or return error
            if status.is_success() {
                return response.json::<T>().await.map_err(RogersError::from);
            }

            // Other errors
            let error_body = response.text().await.unwrap_or_default();
            return Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message: error_body,
            });
        }
    }

    // ─── Issues ───────────────────────────────────────────────────────────────

    /// Get an issue by number.
    pub async fn get_issue(&mut self, number: i32) -> GhResult<Issue> {
        let url = self.repo_url(&format!("/issues/{}", number));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// List issues (REST API).
    pub async fn list_issues(
        &mut self,
        state: Option<&str>,
        labels: Option<Vec<&str>>,
        assignee: Option<&str>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> GhResult<Vec<Issue>> {
        let mut url = self.repo_url("/issues");
        let mut params = Vec::new();

        if let Some(s) = state {
            params.push(format!("state={}", s));
        }
        if let Some(l) = labels {
            for label in l {
                params.push(format!("labels={}", label));
            }
        }
        if let Some(a) = assignee {
            params.push(format!("assignee={}", a));
        }
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(pp) = per_page {
            params.push(format!("per_page={}", pp));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute::<Vec<Issue>>(request).await
    }

    /// Create a new issue.
    pub async fn create_issue(&mut self, req: CreateIssueRequest) -> GhResult<Issue> {
        let url = self.repo_url("/issues");
        let request = self
            .client
            .post(&url)
            .headers(self.auth.auth_headers())
            .json(&req);
        self.execute(request).await
    }

    /// Update an existing issue.
    pub async fn update_issue(&mut self, number: i32, req: UpdateIssueRequest) -> GhResult<Issue> {
        let url = self.repo_url(&format!("/issues/{}", number));
        let request = self
            .client
            .patch(&url)
            .headers(self.auth.auth_headers())
            .json(&req);
        self.execute(request).await
    }

    /// Get comments on an issue.
    pub async fn get_issue_comments(&mut self, number: i32) -> GhResult<Vec<Comment>> {
        let url = self.repo_url(&format!("/issues/{}/comments", number));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// Add comments to an issue.
    pub async fn create_issue_comment(&mut self, number: i32, body: &str) -> GhResult<Comment> {
        use serde_json::json;

        let url = self.repo_url(&format!("/issues/{}/comments", number));
        let request = self
            .client
            .post(&url)
            .headers(self.auth.auth_headers())
            .json(&json!({ "body": body }));
        self.execute(request).await
    }

    // ─── Pull Requests ────────────────────────────────────────────────────────

    /// Get a pull request by number.
    pub async fn get_pull_request(&mut self, number: i32) -> GhResult<PullRequest> {
        let url = self.repo_url(&format!("/pulls/{}", number));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// List pull requests.
    pub async fn list_pull_requests(
        &mut self,
        state: Option<&str>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> GhResult<Vec<PullRequest>> {
        let mut url = self.repo_url("/pulls");
        let mut params = Vec::new();

        if let Some(s) = state {
            params.push(format!("state={}", s));
        }
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(pp) = per_page {
            params.push(format!("per_page={}", pp));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute::<Vec<PullRequest>>(request).await
    }

    // ─── Labels ──────────────────────────────────────────────────────────────

    /// Get all labels for the repository.
    pub async fn list_labels(&mut self) -> GhResult<Vec<Label>> {
        let url = self.repo_url("/labels");
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// Get a label by name.
    pub async fn get_label(&mut self, name: &str) -> GhResult<Label> {
        let url = self.repo_url(&format!("/labels/{}", name));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// Add labels to an issue.
    pub async fn add_issue_labels(
        &mut self,
        issue_number: i32,
        labels: Vec<&str>,
    ) -> GhResult<Vec<Label>> {
        let url = self.repo_url(&format!("/issues/{}/labels", issue_number));
        let request = self
            .client
            .post(&url)
            .headers(self.auth.auth_headers())
            .json(&serde_json::json!({ "labels": labels }));
        self.execute(request).await
    }

    /// Remove a label from an issue.
    pub async fn remove_issue_label(
        &mut self,
        issue_number: i32,
        label_name: &str,
    ) -> GhResult<()> {
        let url = self.repo_url(&format!("/issues/{}/labels/{}", issue_number, label_name));
        let request = self.client.delete(&url).headers(self.auth.auth_headers());
        let _: serde_json::Value = self.execute(request).await?;
        Ok(())
    }

    /// Replace all labels on an issue.
    pub async fn replace_issue_labels(
        &mut self,
        issue_number: i32,
        labels: Vec<&str>,
    ) -> GhResult<Vec<Label>> {
        let url = self.repo_url(&format!("/issues/{}/labels", issue_number));
        let request = self
            .client
            .put(&url)
            .headers(self.auth.auth_headers())
            .json(&serde_json::json!({ "labels": labels }));
        self.execute(request).await
    }

    // ─── Releases ────────────────────────────────────────────────────────────

    /// Get a release by tag name or ID.
    pub async fn get_release(&mut self, tag: &str) -> GhResult<Release> {
        let url = self.repo_url(&format!("/releases/tags/{}", tag));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// List all releases.
    pub async fn list_releases(
        &mut self,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> GhResult<Vec<Release>> {
        let mut url = self.repo_url("/releases");
        let mut params = Vec::new();

        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(pp) = per_page {
            params.push(format!("per_page={}", pp));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute::<Vec<Release>>(request).await
    }

    /// Create a new release.
    pub async fn create_release(
        &mut self,
        tag_name: &str,
        target_commitish: Option<&str>,
        name: Option<&str>,
        body: Option<&str>,
        draft: bool,
        prerelease: bool,
    ) -> GhResult<Release> {
        use serde_json::json;

        let url = self.repo_url("/releases");
        let request = self
            .client
            .post(&url)
            .headers(self.auth.auth_headers())
            .json(&json!({
                "tag_name": tag_name,
                "target_commitish": target_commitish,
                "name": name,
                "body": body,
                "draft": draft,
                "prerelease": prerelease
            }));
        self.execute(request).await
    }

    // ─── Rate Limit ───────────────────────────────────────────────────────────

    /// Get current rate limit status.
    pub async fn get_rate_limit(&mut self) -> GhResult<RateLimitResponse> {
        let url = self.api_url("/rate_limit");
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        let response: RateLimitResponse = self.execute(request).await?;

        // Update our handler with the latest info
        self.rate_limit.update_from_response(&response);

        Ok(response)
    }

    // ─── GraphQL ─────────────────────────────────────────────────────────────

    /// Execute a GraphQL query.
    pub async fn graphql<Q: Serialize, D: for<'de> Deserialize<'de>>(
        &mut self,
        query: &str,
        variables: Option<Q>,
    ) -> GhResult<GraphQLResponse<D>> {
        #[derive(Serialize)]
        struct GraphQLRequest<'a, Q> {
            query: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            variables: Option<Q>,
        }

        let request_body = GraphQLRequest { query, variables };

        let request = self
            .client
            .post(&self.graphql_url())
            .headers(self.auth.auth_headers())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&request_body);

        self.execute(request).await
    }

    /// Get discussions for the repository (GraphQL).
    pub async fn get_discussions(
        &mut self,
        category_id: Option<&str>,
        first: Option<i32>,
        after: Option<&str>,
    ) -> GhResult<DiscussionsResponse> {
        let query = r#"
            query($owner: String!, $repo: String!, $categoryId: ID, $first: Int, $after: String) {
                repository(owner: $owner, name: $repo) {
                    discussions(first: $first, after: $after, categoryId: $categoryId) {
                        pageInfo {
                            hasNextPage
                            endCursor
                        }
                        nodes {
                            id
                            number
                            title
                            body
                            bodyHtml
                            category {
                                id
                                name
                                slug
                            }
                            author {
                                login
                                id
                            }
                            answer {
                                id
                                body
                                author {
                                    login
                                    id
                                }
                            }
                            createdAt
                            updatedAt
                            url
                            comments(first: 100) {
                                totalCount
                                nodes {
                                    id
                                    body
                                    author {
                                        login
                                        id
                                    }
                                }
                            }
                        }
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            owner: String,
            repo: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            category_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            first: Option<i32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            after: Option<String>,
        }

        let variables = Variables {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            category_id: category_id.map(String::from),
            first,
            after: after.map(String::from),
        };

        let response: GraphQLResponse<RepositoryDiscussions> =
            self.graphql(query, Some(variables)).await?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(RogersError::GitHubStatus {
                    code: 400,
                    message: errors
                        .iter()
                        .map(|e| e.message.clone())
                        .collect::<Vec<_>>()
                        .join("; "),
                });
            }
        }

        response
            .data
            .map(|d| {
                let conn = d.repository.discussions;
                DiscussionsResponse {
                    page_info: conn.page_info,
                    nodes: conn.nodes,
                }
            })
            .ok_or_else(|| RogersError::GitHubStatus {
                code: 200,
                message: "No data in GraphQL response".to_string(),
            })
    }

    /// Create a new discussion (GraphQL).
    pub async fn create_discussion(
        &mut self,
        category_id: &str,
        title: &str,
        body: Option<&str>,
    ) -> GhResult<Discussion> {
        let mutation = r#"
            mutation($repositoryId: ID!, $categoryId: ID!, $title: String!, $body: String!) {
                createDiscussion(input: {
                    repositoryId: $repositoryId,
                    categoryId: $categoryId,
                    title: $title,
                    body: $body
                }) {
                    discussion {
                        id
                        number
                        title
                        body
                        createdAt
                        url
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            repository_id: String,
            category_id: String,
            title: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            body: Option<String>,
        }

        // First we need to get the repository ID
        let repo_id = self.get_repository_id().await?;
        let variables = Variables {
            repository_id: repo_id,
            category_id: category_id.to_string(),
            title: title.to_string(),
            body: body.map(String::from),
        };

        #[derive(Deserialize)]
        struct CreateDiscussionResponse {
            #[serde(rename = "createDiscussion")]
            create_discussion: DiscussionCreated,
        }

        #[derive(Deserialize)]
        struct DiscussionCreated {
            discussion: Discussion,
        }

        let response: GraphQLResponse<CreateDiscussionResponse> =
            self.graphql(mutation, Some(variables)).await?;

        if let Some(errors) = response.errors {
            if !errors.is_empty() {
                return Err(RogersError::GitHubStatus {
                    code: 400,
                    message: errors
                        .iter()
                        .map(|e| e.message.clone())
                        .collect::<Vec<_>>()
                        .join("; "),
                });
            }
        }

        response
            .data
            .map(|d| d.create_discussion.discussion)
            .ok_or_else(|| RogersError::GitHubStatus {
                code: 200,
                message: "No data in GraphQL response".to_string(),
            })
    }

    /// Get the repository ID (needed for GraphQL mutations).
    async fn get_repository_id(&mut self) -> GhResult<String> {
        let query = r#"
            query($owner: String!, $repo: String!) {
                repository(owner: $owner, name: $repo) {
                    id
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            owner: String,
            repo: String,
        }

        #[derive(Deserialize)]
        struct RepoResponse {
            repository: RepoId,
        }

        #[derive(Deserialize)]
        struct RepoId {
            id: String,
        }

        let variables = Variables {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
        };

        let response: GraphQLResponse<RepoResponse> = self.graphql(query, Some(variables)).await?;

        response
            .data
            .map(|d| d.repository.id)
            .ok_or_else(|| RogersError::GitHubStatus {
                code: 200,
                message: "Could not retrieve repository ID".to_string(),
            })
    }

    // ─── Commits ────────────────────────────────────────────────────────────

    /// Get a commit by SHA.
    pub async fn get_commit(&mut self, sha: &str) -> GhResult<Commit> {
        let url = self.repo_url(&format!("/commits/{}", sha));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// List commits with optional branch/since filters.
    pub async fn list_commits(
        &mut self,
        sha: Option<&str>,
        since: Option<&str>,
        until: Option<&str>,
        per_page: Option<u32>,
    ) -> GhResult<Vec<Commit>> {
        let mut params = Vec::new();
        if let Some(s) = sha {
            params.push(format!("sha={}", s));
        }
        if let Some(s) = since {
            params.push(format!("since={}", s));
        }
        if let Some(u) = until {
            params.push(format!("until={}", u));
        }
        if let Some(pp) = per_page {
            params.push(format!("per_page={}", pp));
        }

        let url = if params.is_empty() {
            self.repo_url("/commits")
        } else {
            format!("{}?{}", self.repo_url("/commits"), params.join("&"))
        };

        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    // ─── Branches ───────────────────────────────────────────────────────────

    /// List all branches in the repository.
    pub async fn list_branches(&mut self, protected: Option<bool>) -> GhResult<Vec<Branch>> {
        let mut url = self.repo_url("/branches");
        if let Some(p) = protected {
            url = format!("{}?protected={}", url, p);
        }
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// Get a specific branch.
    pub async fn get_branch(&mut self, name: &str) -> GhResult<Branch> {
        let url = self.repo_url(&format!("/branches/{}", name));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// Compare two commits and return a diff.
    ///
    /// `base` and `head` can be branch names, commit SHAs, or tags.
    /// Returns a diff as text.
    pub async fn compare_commits(&mut self, base: &str, head: &str) -> GhResult<CompareResult> {
        let url = self.repo_url(&format!("/compare/{}...{}", base, head));
        let request = self.client.get(&url).headers(self.auth.auth_headers());
        self.execute(request).await
    }

    /// Get raw diff text for a specific file in a commit.
    pub async fn get_commit_file_diff(&mut self, sha: &str, path: &str) -> GhResult<String> {
        let url = self.repo_url(&format!("/commits/{}/{}", sha, path));
        let request = self.client.get(&url).headers(self.auth.auth_headers());

        #[derive(Deserialize)]
        struct FileContent {
            #[serde(rename = "contents")]
            contents: Option<FileContents>,
            pub sha: Option<String>,
        }

        #[derive(Deserialize)]
        struct FileContents {
            pub content: Option<String>,
        }

        let file_content: FileContent = self.execute(request).await?;
        Ok(file_content
            .contents
            .and_then(|c| c.content)
            .unwrap_or_default())
    }

    /// Get the names of files changed in a commit via compare API.
    pub async fn get_commit_files(&mut self, sha: &str) -> GhResult<Vec<String>> {
        // Use the commits list + a two-dot compare to get changed files
        let url = self.repo_url(&format!("/compare/HEAD...{}", sha));
        let request = self.client.get(&url).headers(self.auth.auth_headers());

        #[derive(Deserialize)]
        struct CompareFiles {
            files: Option<Vec<String>>,
        }

        let compare: CompareFiles = self.execute(request).await?;
        Ok(compare.files.unwrap_or_default())
    }

    // ─── Discussions (extended) ─────────────────────────────────────────────

    /// Get discussion categories for the repository.
    pub async fn get_discussion_categories(&mut self) -> GhResult<Vec<DiscussionCategory>> {
        let query = r#"
            query($owner: String!, $repo: String!) {
                repository(owner: $owner, name: $repo) {
                    discussionCategories(first: 20) {
                        nodes {
                            id
                            name
                            slug
                            description
                            isAnswerable
                        }
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            owner: String,
            repo: String,
        }

        #[derive(Deserialize)]
        struct RepoCats {
            repository: CatsData,
        }

        #[derive(Deserialize)]
        struct CatsData {
            #[serde(rename = "discussionCategories")]
            discussion_categories: CatConn,
        }

        #[derive(Deserialize)]
        struct CatConn {
            nodes: Vec<DiscussionCategory>,
        }

        let variables = Variables {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
        };

        let response: GraphQLResponse<RepoCats> = self.graphql(query, Some(variables)).await?;

        response
            .data
            .map(|d| d.repository.discussion_categories.nodes)
            .ok_or_else(|| RogersError::GitHubStatus {
                code: 200,
                message: "Could not retrieve discussion categories".to_string(),
            })
    }

    /// Add a reaction to a discussion or comment.
    pub async fn add_discussion_reaction(
        &mut self,
        subject_id: &str,
        content: &str,
    ) -> GhResult<Reaction> {
        let mutation = r#"
            mutation($subjectId: ID!, $content: String!) {
                addReaction(input: {subjectId: $subjectId, content: $content}) {
                    reaction {
                        id
                        content
                        createdAt
                        user {
                            login
                            id
                        }
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            subject_id: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct ReactionAdded {
            #[serde(rename = "addReaction")]
            add_reaction: ReactionData,
        }

        #[derive(Deserialize)]
        struct ReactionData {
            reaction: Reaction,
        }

        let variables = Variables {
            subject_id: subject_id.to_string(),
            content: content.to_string(),
        };

        let response: GraphQLResponse<ReactionAdded> =
            self.graphql(mutation, Some(variables)).await?;

        response
            .data
            .map(|d| d.add_reaction.reaction)
            .ok_or_else(|| RogersError::GitHubStatus {
                code: 200,
                message: "Could not add reaction".to_string(),
            })
    }

    /// Get reactions on a discussion by fetching it and reading reactions via GraphQL.
    pub async fn get_discussion_reactions(
        &mut self,
        discussion_number: i32,
    ) -> GhResult<Vec<Reaction>> {
        // Use GraphQL to get discussion with reactions
        let query = r#"
            query($owner: String!, $repo: String!, $number: Int!) {
                repository(owner: $owner, name: $repo) {
                    discussion(number: $number) {
                        id
                        reactions(first: 50) {
                            nodes {
                                id
                                content
                                createdAt
                                user {
                                    login
                                    id
                                }
                            }
                        }
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Variables {
            owner: String,
            repo: String,
            number: i32,
        }

        #[derive(Deserialize)]
        struct RepoDisc {
            repository: DiscRepo,
        }

        #[derive(Deserialize)]
        struct DiscRepo {
            discussion: Option<DiscWithReactions>,
        }

        #[derive(Deserialize)]
        struct DiscWithReactions {
            id: String,
            reactions: ReactionNodes,
        }

        #[derive(Deserialize)]
        struct ReactionNodes {
            nodes: Vec<Reaction>,
        }

        let variables = Variables {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            number: discussion_number,
        };

        let response: GraphQLResponse<RepoDisc> = self.graphql(query, Some(variables)).await?;

        response
            .data
            .and_then(|d| d.repository.discussion)
            .map(|d| d.reactions.nodes)
            .ok_or_else(|| RogersError::GitHubStatus {
                code: 200,
                message: "Could not retrieve discussion reactions".to_string(),
            })
    }

    // ─── Validation ─────────────────────────────────────────────────────────

    /// Validate the authentication by making a simple API call.
    pub async fn validate_auth(&mut self) -> GhResult<()> {
        // Try to get rate limit (which requires auth)
        match self.get_rate_limit().await {
            Ok(_) => {
                // Validate token format
                self.auth
                    .validate_token()
                    .map_err(|e| RogersError::Auth(e.to_string()))?;
                Ok(())
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

// GraphQL response structures
#[derive(Debug, Deserialize)]
struct RepositoryDiscussions {
    repository: DiscussionsData,
}

#[derive(Debug, Deserialize)]
struct DiscussionsData {
    discussions: DiscussionConnection,
}

#[derive(Debug, Deserialize)]
struct DiscussionConnection {
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    nodes: Vec<Discussion>,
}

impl From<DiscussionsData> for DiscussionsResponse {
    fn from(data: DiscussionsData) -> Self {
        Self {
            page_info: data.discussions.page_info,
            nodes: data.discussions.nodes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionsResponse {
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
    pub nodes: Vec<Discussion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TOKEN: &str = "ghp_test_token_1234567890";

    fn create_test_client() -> GitHubClient {
        let auth = GitHubAuth::new_with_default_api(TEST_TOKEN);
        GitHubClient::new("test-owner", "test-repo", auth)
    }

    #[test]
    fn test_new_client() {
        let client = create_test_client();
        assert_eq!(client.owner(), "test-owner");
        assert_eq!(client.repo(), "test-repo");
    }

    #[test]
    fn test_api_url() {
        let client = create_test_client();
        assert_eq!(client.api_url("/test"), "https://api.github.com/test");
    }

    #[test]
    fn test_graphql_url() {
        let client = create_test_client();
        assert_eq!(client.graphql_url(), "https://api.github.com/graphql");
    }

    #[test]
    fn test_repo_url() {
        let client = create_test_client();
        assert_eq!(
            client.repo_url("/issues"),
            "https://api.github.com/repos/test-owner/test-repo/issues"
        );
    }

    #[test]
    fn test_repo_url_with_params() {
        let client = create_test_client();
        assert_eq!(
            client.repo_url("/issues/123"),
            "https://api.github.com/repos/test-owner/test-repo/issues/123"
        );
    }

    #[tokio::test]
    async fn test_client_validates_auth_on_creation() {
        let auth = GitHubAuth::new_with_default_api(TEST_TOKEN);
        let client = GitHubClient::new("test-owner", "test-repo", auth);

        // Auth is valid at creation time
        assert!(client.auth.validate_token().is_ok());
    }
}
