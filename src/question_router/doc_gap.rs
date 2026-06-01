//! Documentation gap filing for question routing.
//!
//! Creates chore tasks to track documentation gaps identified from question issues.
//! Filed when neither documentation nor code search finds an answer.
//!
//! Plan: plans/question-routing-plan.md §Step 3b

use crate::backlog::controller::TaskController;
use crate::backlog::schema::task_type;
use crate::error::Result;
use crate::github::models::Issue;
use serde::{Deserialize, Serialize};

/// Doc gap filing result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGapResult {
    /// The task ID that was created.
    pub task_id: String,
    /// The linked GitHub issue URL.
    pub github_issue_url: String,
    /// Accepted doc link pattern (for sync verification).
    pub expected_doc_pattern: String,
}

/// Doc gap filer for creating documentation gap tasks.
#[derive(Debug, Clone)]
pub struct DocGapFiler {
    /// Task controller for creating tasks.
    task_controller: TaskController,
    /// Repository owner.
    owner: String,
    /// Repository name.
    repo: String,
}

impl DocGapFiler {
    /// Create a new doc gap filer.
    pub fn new(
        task_controller: TaskController,
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> Self {
        Self {
            task_controller,
            owner: owner.into(),
            repo: repo.into(),
        }
    }

    /// File a documentation gap task for a question issue.
    ///
    /// Creates a chore task with `rodgers:type=docs` metadata.
    pub async fn file_doc_gap(&self, request: DocGapRequest) -> Result<DocGapResult> {
        let github_issue_url = format!(
            "https://github.com/{}/{}/issues/{}",
            self.owner, self.repo, request.issue_number
        );

        // Format acceptance criteria as a doc link requirement
        let acceptance_criteria = format!(
            r#"- [ ] Document the answer to: {}

The documentation should:
1. Answer the question at: {}
2. Be placed in the most relevant existing doc file (or new section)
3. Link to this task from the GitHub issue when complete

File pattern: docs/**/*.md
Must contain a section that answers the question above."#,
            request.question_title, github_issue_url
        );

        // Create the task
        let create_request = crate::backlog::controller::CreateEpicRequest {
            title: format!("Answer question: {}", request.question_title),
            description: Some(format!(
                "{}\n\n---\n\n**Full Question from GitHub Issue #{}**\n\n{}\n\n---\n\n**Context from original issue:**\n{}",
                request.question_summary.unwrap_or_default(),
                request.issue_number,
                request.full_question.clone(),
                request.issue_body.clone().unwrap_or_default()
            )),
            task_type: Some(task_type::CHORE.to_string()),
            github_issue_url: Some(github_issue_url.clone()),
            rodgers_type: Some("docs".to_string()),
            rodgers_labels: Some("needs-documentation".to_string()),
            discovered_from: Some(format!("question:{}", request.issue_number)),
            acceptance_criteria: Some(acceptance_criteria),
            priority: Some(request.priority.unwrap_or(3)), // Default medium priority
        };

        let task = self.task_controller.file_epic(create_request).await?;

        tracing::info!(
            "Filed doc gap task {} for issue #{}: {}",
            task.id,
            request.issue_number,
            request.question_title
        );

        Ok(DocGapResult {
            task_id: task.id,
            github_issue_url,
            expected_doc_pattern: "docs/".to_string(),
        })
    }

    /// File a doc gap for multiple questions in one issue.
    ///
    /// Creates separate tasks for each question.
    pub async fn file_multiple_doc_gaps(
        &self,
        requests: Vec<DocGapRequest>,
    ) -> Result<Vec<DocGapResult>> {
        let mut results = Vec::new();
        let first_issue_number = requests.first().map(|r| r.issue_number).unwrap_or(0);

        for request in requests {
            match self.file_doc_gap(request).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::error!(
                        "Failed to file doc gap for issue #{}: {}",
                        first_issue_number,
                        e
                    );
                    // Continue with other tasks even if one fails
                }
            }
        }

        Ok(results)
    }

    /// Check if a doc gap task already exists for an issue.
    pub async fn doc_gap_exists_for_issue(&self, issue_number: i32) -> Result<bool> {
        let github_issue_url = format!(
            "https://github.com/{}/{}/issues/{}",
            self.owner, self.repo, issue_number
        );

        let epic = self
            .task_controller
            .get_epic_by_issue(&github_issue_url)
            .await?;
        Ok(epic.is_some())
    }

    /// Get pending doc gap tasks that need documentation.
    pub async fn get_pending_doc_gaps(&self) -> Result<Vec<PendingDocGap>> {
        // This would query for tasks with rodgers:type=docs and status=open
        // For now, return empty - this would be implemented with the actual DB query
        Ok(Vec::new())
    }
}

/// Request to file a documentation gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGapRequest {
    /// Issue number on GitHub.
    pub issue_number: i32,
    /// Title of the question (one-line restatement).
    pub question_title: String,
    /// Summary of the question.
    pub question_summary: Option<String>,
    /// Full question text from the issue.
    pub full_question: String,
    /// Original issue body (for context).
    pub issue_body: Option<String>,
    /// Priority (1=highest, 5=lowest). Defaults to 3.
    pub priority: Option<i32>,
}

impl DocGapRequest {
    /// Create a new doc gap request from an issue.
    pub fn from_issue(issue: &Issue) -> Self {
        Self {
            issue_number: issue.number,
            question_title: extract_question_title(&issue.title),
            question_summary: None,
            full_question: issue.body.clone().unwrap_or_else(|| issue.title.clone()),
            issue_body: issue.body.clone(),
            priority: Some(3),
        }
    }

    /// Create with a custom question title.
    pub fn with_title(mut self, title: String) -> Self {
        self.question_title = title;
        self
    }

    /// Create with a summary.
    pub fn with_summary(mut self, summary: Option<String>) -> Self {
        self.question_summary = summary;
        self
    }
}

/// Pending documentation gap information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDocGap {
    /// Task ID.
    pub task_id: String,
    /// Linked GitHub issue URL.
    pub github_issue_url: Option<String>,
    /// Task title.
    pub title: String,
    /// Status.
    pub status: String,
}

/// Extract a one-line question title from the issue title.
fn extract_question_title(title: &str) -> String {
    let trimmed = title.trim();

    // Remove common prefixes that add noise
    let clean = trimmed
        .trim_start_matches("Question: ")
        .trim_start_matches("Question about ")
        .trim_start_matches("[Question] ")
        .trim_start_matches("[question] ");

    // Truncate if too long
    if clean.len() > 80 {
        let truncated: String = clean.chars().take(77).collect();
        format!("{}...", truncated)
    } else {
        clean.to_string()
    }
}

/// Generate the acknowledgment comment for a doc gap filing.
pub fn generate_doc_gap_comment(requestor: &str, task_id: &str) -> String {
    format!(
        r#"Hi @{requestor}, thanks for the question!

We do not currently have documentation that answers this. We have opened internal task {task_id} to add an answer to our documentation — it will be linked here when complete.

If you have the answer or can help add documentation, we welcome contributions!"#
    )
}

/// Generate the doc answer comment with link.
pub fn generate_doc_answer_comment(
    requestor: &str,
    doc_path: &str,
    section: Option<&str>,
    snippet: &str,
) -> String {
    let section_link = section.map(|s| format!(" §{}", s)).unwrap_or_default();

    let doc_url = doc_path.replace(".md", "");

    format!(
        r#"Hi @{requestor}, thanks for the question!

The answer to your question is covered in [{doc_path}{section_link}]({doc_url}).

_{snippet}_

If this doesn't fully answer your question, please let us know and we will follow up."#
    )
}

/// Generate the code answer comment.
pub fn generate_code_answer_comment(
    requestor: &str,
    explanation: &str,
    file_path: &str,
    symbol_name: Option<&str>,
    line_number: Option<u32>,
) -> String {
    let symbol_ref = symbol_name
        .map(|s| format!(" in `{}`", s))
        .unwrap_or_default();

    let line_ref = line_number
        .map(|l| format!(" at line {}", l))
        .unwrap_or_default();

    format!(
        r#"Hi @{requestor}, thanks for this question! I took a look at the source code to find the answer.

{explanation}

Relevant source: `{file_path}`{symbol_ref}{line_ref}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_question_title() {
        assert_eq!(
            extract_question_title("Question: How does the triage engine work?"),
            "How does the triage engine work?"
        );
        assert_eq!(
            extract_question_title("What is the difference between X and Y?"),
            "What is the difference between X and Y?"
        );
        assert_eq!(
            extract_question_title("How do I configure the settings for the bot?"),
            "How do I configure the settings for the bot?"
        );
    }

    #[test]
    fn test_extract_question_title_truncation() {
        let long_title = "This is a very long question title that is definitely longer than eighty characters and should be truncated to fit the task title requirements";
        let extracted = extract_question_title(long_title);
        assert!(extracted.len() <= 80);
        assert!(extracted.ends_with("..."));
    }

    #[test]
    fn test_doc_gap_comment() {
        let comment = generate_doc_gap_comment("username", "123-456");
        assert!(comment.contains("@username"));
        assert!(comment.contains("123-456"));
        assert!(comment.contains("documentation"));
    }

    #[test]
    fn test_doc_answer_comment() {
        let comment = generate_doc_answer_comment(
            "testuser",
            "docs/configuration.md",
            Some("Authentication"),
            "Configuration is done via config.yaml",
        );
        assert!(comment.contains("@testuser"));
        assert!(comment.contains("configuration.md"));
        assert!(comment.contains("Authentication"));
    }

    #[test]
    fn test_code_answer_comment() {
        let comment = generate_code_answer_comment(
            "devel",
            "The function handles issue routing by checking the state machine.",
            "src/triage/engine.rs",
            Some("TriageEngine::new"),
            Some(42),
        );
        assert!(comment.contains("@devel"));
        assert!(comment.contains("engine.rs"));
        assert!(comment.contains("TriageEngine::new"));
        assert!(comment.contains("line 42"));
    }

    #[test]
    fn test_doc_gap_request_from_issue() {
        let issue = Issue {
            number: 123,
            title: "How does the question router work?".to_string(),
            body: Some("I want to understand the question routing flow.".to_string()),
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "user123".to_string(),
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

        let request = DocGapRequest::from_issue(&issue);
        assert_eq!(request.issue_number, 123);
        assert!(request.question_title.contains("question router"));
        assert!(request.full_question.contains("question routing flow"));
    }
}
