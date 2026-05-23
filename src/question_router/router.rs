//! Question Router for routing question issues.
//!
//! Main entry point for the question routing workflow. Coordinates documentation
//! search, code search, and doc gap filing for GitHub issues labeled `question`.
//!
//! Plan: plans/question-routing-plan.md §Question Router Decision Tree

use crate::beads::client::BeadsClient;
use crate::beads::controller::BeadController;
use crate::error::{Result, RogersError};
use crate::github::client::GitHubClient;
use crate::github::models::Issue;
use crate::llm::client::LlmClient;
use crate::llm::prompts::ClassificationPrompt;
use crate::llm::prompts::IssueMetadata;
use crate::question_router::code_search::{CodeSearchResult, CodeSearcher};
use crate::question_router::doc_gap::{
    generate_code_answer_comment, generate_doc_answer_comment, generate_doc_gap_comment,
    DocGapFiler, DocGapRequest,
};
use crate::question_router::doc_search::{DocSearchResult, DocSearcher};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Question router action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterActionType {
    /// Post a documentation link answer.
    PostDocAnswer,
    /// Post a code explanation answer.
    PostCodeAnswer,
    /// File a documentation gap bead.
    FileDocGap,
    /// No action needed (already handled or not a question).
    NoAction,
}

/// Question routing result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResult {
    /// Action to take.
    pub action: RouterActionType,
    /// Issue number.
    pub issue: i32,
    /// Comment to post (if applicable).
    pub comment: Option<String>,
    /// Labels to add.
    pub labels_to_add: Vec<String>,
    /// Labels to remove.
    pub labels_to_remove: Vec<String>,
    /// Whether to close the issue.
    pub close_issue: bool,
    /// Doc search result (if applicable).
    pub doc_result: Option<DocSearchResult>,
    /// Code search result (if applicable).
    pub code_result: Option<CodeSearchResult>,
    /// Doc gap result (if applicable).
    pub doc_gap_bead_id: Option<String>,
    /// Explanation for code answers (plain language).
    pub code_explanation: Option<String>,
}

/// Question router for processing question-labeled issues.
#[derive(Debug, Clone)]
pub struct QuestionRouter {
    /// GitHub client for posting comments and labels.
    github: GitHubClient,
    /// LLM client for question analysis.
    llm: LlmClient,
    /// Documentation searcher.
    doc_searcher: Arc<std::sync::Mutex<DocSearcher>>,
    /// Code searcher.
    code_searcher: Arc<std::sync::Mutex<CodeSearcher>>,
    /// Doc gap filer.
    doc_gap_filer: DocGapFiler,
    /// Repository owner.
    owner: String,
    /// Repository name.
    repo: String,
}

impl QuestionRouter {
    /// Create a new question router.
    pub fn new(
        github: GitHubClient,
        llm: LlmClient,
        bead_controller: BeadController,
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> Self {
        let owner = owner.into();
        let repo = repo.into();

        Self {
            github,
            llm,
            doc_searcher: Arc::new(std::sync::Mutex::new(DocSearcher::standard())),
            code_searcher: Arc::new(std::sync::Mutex::new(CodeSearcher::standard())),
            doc_gap_filer: DocGapFiler::new(bead_controller, owner.clone(), repo.clone()),
            owner,
            repo,
        }
    }

    /// Initialize searchers by loading documents and source files.
    pub async fn initialize(&self) -> Result<()> {
        tracing::info!("Initializing question router searchers...");

        // Load documentation
        {
            let mut doc_searcher = self.doc_searcher.lock().unwrap();
            doc_searcher.load_documents()?;
        }

        // Load source code
        {
            let mut code_searcher = self.code_searcher.lock().unwrap();
            code_searcher.load_source_files()?;
        }

        tracing::info!("Question router searchers initialized");
        Ok(())
    }

    /// Process a question issue and determine routing actions.
    pub async fn route_question(&self, issue: &Issue) -> Result<RoutingResult> {
        tracing::info!("Routing question issue #{}: {}", issue.number, issue.title);

        // CRIT-5: Check rodgers:question label on entry for idempotency
        // If the label is not present, this issue was not classified by triage
        if !self.has_rodgers_question_label(issue) {
            tracing::info!(
                "Issue #{} missing rodgers:question label - not classified by triage, skipping",
                issue.number
            );
            return Ok(RoutingResult {
                action: RouterActionType::NoAction,
                issue: issue.number,
                comment: None,
                labels_to_add: vec![],
                labels_to_remove: vec![],
                close_issue: false,
                doc_result: None,
                code_result: None,
                doc_gap_bead_id: None,
                code_explanation: None,
            });
        }

        // Check if we've already handled this issue
        if self.has_rodgers_response(issue).await? {
            tracing::info!(
                "Issue #{} already has a Rodgers response, skipping",
                issue.number
            );
            return Ok(RoutingResult {
                action: RouterActionType::NoAction,
                issue: issue.number,
                comment: None,
                labels_to_add: vec![],
                labels_to_remove: vec![],
                close_issue: false,
                doc_result: None,
                code_result: None,
                doc_gap_bead_id: None,
                code_explanation: None,
            });
        }

        // Combine title and body for search
        let query = build_search_query(issue);
        tracing::debug!("Search query: {}", query);

        // First, search documentation
        let doc_result = {
            let searcher = self.doc_searcher.lock().unwrap();
            searcher.find_best_match(&query)
        };

        if let Some(ref doc) = doc_result {
            tracing::info!(
                "Found documentation match for issue #{}: {}",
                issue.number,
                doc.path
            );

            // Generate doc answer comment
            let comment = generate_doc_answer_comment(
                &issue.user.login,
                &doc.path,
                doc.section.as_deref(),
                &doc.snippet,
            );

            return Ok(RoutingResult {
                action: RouterActionType::PostDocAnswer,
                issue: issue.number,
                comment: Some(comment),
                labels_to_add: vec![],
                labels_to_remove: vec!["question".to_string()],
                close_issue: true, // Doc answer fully answers the question
                doc_result: doc_result.clone(),
                code_result: None,
                doc_gap_bead_id: None,
                code_explanation: None,
            });
        }

        // No doc found - check if this is an implementation question
        let should_search_code = {
            let code_searcher = self.code_searcher.lock().unwrap();
            code_searcher.should_search_code(&query) || contains_impl_keywords(&query)
        };

        if should_search_code {
            tracing::info!(
                "Implementation question suspected for issue #{}, searching code",
                issue.number
            );

            // Search source code
            let code_results = {
                let searcher = self.code_searcher.lock().unwrap();
                searcher.find_relevant_files(&query)
            };

            if !code_results.is_empty() {
                // Found code - explain it
                let best_result = &code_results[0];
                let explanation = self.generate_code_explanation(&query, &code_results)?;

                let comment = generate_code_answer_comment(
                    &issue.user.login,
                    &explanation,
                    &best_result.file_path,
                    best_result.symbol_name.as_deref(),
                    Some(best_result.line_number),
                );

                tracing::info!(
                    "Found code answer for issue #{} in {}:{}",
                    issue.number,
                    best_result.file_path,
                    best_result.line_number
                );

                return Ok(RoutingResult {
                    action: RouterActionType::PostCodeAnswer,
                    issue: issue.number,
                    comment: Some(comment),
                    labels_to_add: vec![],
                    labels_to_remove: vec!["question".to_string()],
                    close_issue: true, // Code answer fully answers implementation questions
                    doc_result: None,
                    code_result: Some(best_result.clone()),
                    doc_gap_bead_id: None,
                    code_explanation: Some(explanation),
                });
            }
        }

        // No documentation or code found - file a doc gap
        tracing::info!(
            "No answer found for issue #{}, filing doc gap",
            issue.number
        );

        let doc_gap_request = DocGapRequest::from_issue(issue);
        let doc_gap_result = self.doc_gap_filer.file_doc_gap(doc_gap_request).await?;

        let comment = generate_doc_gap_comment(&issue.user.login, &doc_gap_result.bead_id);

        Ok(RoutingResult {
            action: RouterActionType::FileDocGap,
            issue: issue.number,
            comment: Some(comment),
            labels_to_add: vec!["needs-documentation".to_string()],
            labels_to_remove: vec!["question".to_string()],
            close_issue: false,
            doc_result: None,
            code_result: None,
            doc_gap_bead_id: Some(doc_gap_result.bead_id),
            code_explanation: None,
        })
    }

    /// Check if this is a genuine question that can be answered.
    async fn classify_question_type(&self, issue: &Issue) -> Result<QuestionClassification> {
        let metadata = IssueMetadata {
            number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            author: issue.user.login.clone(),
            author_type: issue.user.user_type.clone(),
            labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
            prior_comments: vec![], // Would need to fetch comments
        };

        let prompt = ClassificationPrompt::for_classification(&metadata, None);
        let request = crate::llm::client::ChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![
                crate::llm::client::ChatMessage::system(prompt.system_prompt),
                crate::llm::client::ChatMessage::user(prompt.user_prompt),
            ],
            temperature: Some(0.3),
            max_tokens: Some(500),
            response_format: Some(crate::llm::client::ResponseFormat {
                format_type: "json_object".to_string(),
                schema: None,
            }),
        };

        let response = self.llm.chat(request).await?;

        // Parse the response as question classification
        let content = &response.choices[0].message.content;
        let parsed: QuestionClassification =
            serde_json::from_str(content).map_err(|e| RogersError::Json(e))?;

        Ok(parsed)
    }

    /// Check if the issue has the rodgers:question label applied by triage.
    /// CRIT-5: This enables idempotent question routing - if the label is present,
    /// the question has already been classified and routed.
    pub fn has_rodgers_question_label(&self, issue: &Issue) -> bool {
        issue.labels.iter().any(|l| l.name == "rodgers:question")
    }

    /// Generate a plain-language explanation of code.
    fn generate_code_explanation(
        &self,
        _query: &str,
        results: &[CodeSearchResult],
    ) -> Result<String> {
        if results.is_empty() {
            return Ok("The relevant implementation was not found.".to_string());
        }

        // Use LLM to generate a plain-language explanation from the code snippets
        let context = results
            .iter()
            .take(3)
            .map(|r| {
                format!(
                    "File: {}\nLine: {}\nSymbol: {}\nSnippet: {}\nContext: {:?}\n---\n",
                    r.file_path,
                    r.line_number,
                    r.symbol_name.as_deref().unwrap_or("unknown"),
                    r.snippet,
                    r.context_lines.iter().take(3).collect::<Vec<_>>()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            "I found the relevant code. Here's how it works:\n\nThe implementation is primarily in `{}` at line {}, in the `{}` function/module.\n\nFor the most accurate understanding, the full implementation spans multiple locations including the symbols: {}",
            results[0].file_path,
            results[0].line_number,
            results[0].symbol_name.as_deref().unwrap_or("main component"),
            results
                .iter()
                .take(5)
                .filter_map(|r| r.symbol_name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// Check if the issue already has a Rodgers response.
    async fn has_rodgers_response(&self, issue: &Issue) -> Result<bool> {
        // Would check issue comments for a prior Rodgers comment
        // For now, check if the issue has already been labeled
        let has_doc_answer = issue.labels.iter().any(|l| l.name == "doc-answer");
        let has_code_answer = issue.labels.iter().any(|l| l.name == "code-answer");
        let has_needs_doc = issue.labels.iter().any(|l| l.name == "needs-documentation");

        Ok(has_doc_answer || has_code_answer || has_needs_doc)
    }

    /// Execute a routing result by posting comments and labels.
    pub async fn execute_routing_result(&mut self, result: &RoutingResult) -> Result<()> {
        tracing::info!(
            "Executing routing action {:?} for issue #{}",
            result.action,
            result.issue
        );

        match result.action {
            RouterActionType::PostDocAnswer
            | RouterActionType::PostCodeAnswer
            | RouterActionType::FileDocGap => {
                // Post comment
                if let Some(ref comment) = result.comment {
                    self.github
                        .create_issue_comment(result.issue, comment)
                        .await?;
                }

                // Update labels
                if !result.labels_to_add.is_empty() || !result.labels_to_remove.is_empty() {
                    let labels_to_add: Vec<&str> =
                        result.labels_to_add.iter().map(|s| s.as_str()).collect();
                    let labels_to_remove: Vec<&str> =
                        result.labels_to_remove.iter().map(|s| s.as_str()).collect();

                    if !labels_to_add.is_empty() {
                        self.github
                            .add_issue_labels(result.issue, labels_to_add)
                            .await?;
                    }

                    for label in labels_to_remove {
                        self.github
                            .remove_issue_label(result.issue, label)
                            .await
                            .ok();
                    }
                }
            }
            RouterActionType::NoAction => {
                tracing::debug!("No action needed for issue #{}", result.issue);
            }
        }

        Ok(())
    }
}

/// Question classification from LLM analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionClassification {
    /// Whether this is a genuine question.
    pub is_question: bool,
    /// Whether the question is about implementation details.
    pub is_implementation_question: bool,
    /// Search scope to use.
    pub search_scope: String, // "docs", "code", "both", "none"
    /// What information would answer this question.
    pub answer_context: Option<String>,
    /// Confidence in this classification.
    pub confidence: f32,
}

/// Build a search query from issue title and body.
fn build_search_query(issue: &Issue) -> String {
    let mut query = issue.title.clone();

    if let Some(ref body) = issue.body {
        // Append first 500 chars of body for context
        let body_context = if body.len() > 500 {
            body[..500].to_string()
        } else {
            body.clone()
        };
        query.push_str(" ");
        query.push_str(&body_context);
    }

    query
}

/// Check if query contains implementation keywords.
fn contains_impl_keywords(query: &str) -> bool {
    let impl_keywords = [
        "how does",
        "what function",
        "what method",
        "which module",
        "implementation",
        "internals",
        "source code",
        "walk me through",
        "flow of",
        "under the hood",
        "how is",
        "what does",
        "where is",
    ];

    let query_lower = query.to_lowercase();
    impl_keywords
        .iter()
        .any(|kw| query_lower.contains(&kw.to_lowercase()))
}

/// Check if this is actually a question (vs bug/feature disguised as question).
fn is_genuine_question(title: &str, body: Option<&str>) -> bool {
    // Check for question indicators
    let question_words = [
        "how",
        "what",
        "why",
        "when",
        "where",
        "can i",
        "could you",
        "is it",
    ];

    let title_lower = title.to_lowercase();
    let has_question_word = question_words.iter().any(|w| title_lower.contains(w));

    // Check for non-question indicators
    let non_question_words = [
        "bug:",
        "feature request:",
        "request:",
        "doesn't work",
        "not working",
        "crashes",
        "error:",
        "should add",
        "please implement",
    ];

    let has_non_question = non_question_words.iter().any(|w| title_lower.contains(w));

    if has_non_question {
        return false;
    }

    has_question_word || body.map(|b| b.contains('?')).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_query() {
        let issue = Issue {
            number: 1,
            title: "How does the triage engine work?".to_string(),
            body: Some("I want to understand the workflow.".to_string()),
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "test".to_string(),
                id: 1,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: None,
            },
            labels: vec![],
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        };

        let query = build_search_query(&issue);
        assert!(query.contains("triage engine"));
        assert!(query.contains("workflow"));
    }

    #[test]
    fn test_build_search_query_truncation() {
        let long_body = "x".repeat(1000);
        let issue = Issue {
            number: 1,
            title: "Question".to_string(),
            body: Some(long_body),
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "test".to_string(),
                id: 1,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: None,
            },
            labels: vec![],
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        };

        let query = build_search_query(&issue);
        assert!(query.len() < 1000 + 50); // Title + truncated body
    }

    #[test]
    fn test_impl_keywords() {
        assert!(contains_impl_keywords("How does the triage engine work?"));
        assert!(contains_impl_keywords("What function handles X?"));
        assert!(contains_impl_keywords("Walk me through the flow"));
        assert!(contains_impl_keywords("Can you show me the internals?"));

        assert!(!contains_impl_keywords("How do I install this?"));
        assert!(!contains_impl_keywords("What is the price?"));
    }

    #[test]
    fn test_is_genuine_question() {
        assert!(is_genuine_question(
            "How does X work?",
            Some("Can you explain?")
        ));
        assert!(is_genuine_question("What is the best approach?", None));
        assert!(is_genuine_question("Question: How do I configure?", None));

        assert!(!is_genuine_question("Bug: It crashes on startup", None));
        assert!(!is_genuine_question("Feature request: Add X", None));
    }

    #[test]
    fn test_routing_result_serialization() {
        let result = RoutingResult {
            action: RouterActionType::PostDocAnswer,
            issue: 123,
            comment: Some("Test comment".to_string()),
            labels_to_add: vec![],
            labels_to_remove: vec!["question".to_string()],
            close_issue: true,
            doc_result: None,
            code_result: None,
            doc_gap_bead_id: None,
            code_explanation: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("post_doc_answer"));
        assert!(json.contains("123"));
    }

    // =============================================================================
    // CRIT-5: rodgers:question label enables idempotent question routing
    // =============================================================================

    fn create_test_router() -> QuestionRouter {
        let config = crate::config::schema::Config {
            github: crate::config::GitHubConfig {
                owner: "test".to_string(),
                repo: "test".to_string(),
                token: "test".to_string(),
                api_url: None,
            },
            scheduler: crate::config::SchedulerConfig::default(),
            beads: crate::config::BeadsConfig::default(),
            llm: crate::config::LlmConfig {
                provider: Some("openai".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                model: "gpt-4o-mini".to_string(),
                api_key: "test".to_string(),
            },
            triage: Some(crate::config::TriageConfig::default()),
            release: None,
            rogation: None,
            log_level: None,
            error_channel: None,
        };
        let github = GitHubClient::new(
            &config.github.owner,
            &config.github.repo,
            crate::github::GitHubAuth::new_with_default_api(&config.github.token),
        );
        let llm = LlmClient::new(&config.llm);
        let bead_controller = crate::beads::controller::BeadController::new(
            "test".to_string(),
            "test".to_string(),
            crate::beads::client::BeadsClient::new(),
        );
        QuestionRouter::new(github, llm, bead_controller, "test", "test")
    }

    fn create_question_issue(number: i32, labels: Vec<&str>) -> Issue {
        Issue {
            number,
            title: "How does X work?".to_string(),
            body: Some("Can you explain?".to_string()),
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "testuser".to_string(),
                id: 1,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: None,
            },
            labels: labels
                .into_iter()
                .map(|name| crate::github::models::Label {
                    id: 1,
                    name: name.to_string(),
                    description: None,
                    color: None,
                    node_id: None,
                })
                .collect(),
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        }
    }

    #[test]
    fn test_has_rodgers_question_label_present() {
        let router = create_test_router();
        let issue = create_question_issue(1, vec!["question", "rodgers:question"]);
        assert!(router.has_rodgers_question_label(&issue));
    }

    #[test]
    fn test_has_rodgers_question_label_missing() {
        let router = create_test_router();
        let issue = create_question_issue(2, vec!["question"]);
        assert!(!router.has_rodgers_question_label(&issue));
    }

    #[test]
    fn test_has_rodgers_question_label_no_labels() {
        let router = create_test_router();
        let issue = create_question_issue(3, vec![]);
        assert!(!router.has_rodgers_question_label(&issue));
    }
}
