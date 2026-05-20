//! Question Router for Rodgers.
//!
//! Handles question issues by searching documentation and posting warm
//! responses with relevant doc links. Implements Step 3a of the question
//! routing workflow.

use crate::error::{Result, RogersError};
use crate::llm::prompts;
use crate::llm::{LlmClient, LlmConversation};

pub mod doc_search;

/// Result of processing a question issue.
#[derive(Debug, Clone)]
pub struct QuestionRoutingResult {
    /// Whether a documentation answer was found.
    pub answer_found: bool,
    /// The documentation link used (if found).
    pub doc_link: Option<String>,
    /// The drafted comment (if successful).
    pub comment: Option<String>,
    /// Whether the issue should be closed.
    pub should_close: bool,
}

/// GitHub issue data for question routing.
#[derive(Debug, Clone)]
pub struct QuestionIssue {
    /// GitHub issue number.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Issue body (description).
    pub body: String,
    /// Username of the requestor (author).
    pub requestor: String,
    /// Existing labels on the issue.
    pub labels: Vec<String>,
    /// Existing comments on the issue.
    pub prior_comments: Vec<String>,
}

/// Router for handling question issues.
#[derive(Debug, Clone)]
pub struct QuestionRouter {
    docs_path: std::path::PathBuf,
    search_limit: usize,
}

impl QuestionRouter {
    /// Create a new QuestionRouter.
    ///
    /// # Arguments
    ///
    /// * `docs_path` - Path to the docs directory
    /// * `search_limit` - Maximum number of doc search results to consider
    pub fn new(docs_path: std::path::PathBuf, search_limit: usize) -> Self {
        Self {
            docs_path,
            search_limit,
        }
    }

    /// Route a question issue through the documentation search workflow.
    ///
    /// This implements the CRIT-1 acceptance criteria:
    /// When a question issue exists and docs exist that answer it,
    /// Rodgers posts a comment within one triage run with the correct
    /// doc link (docs/filename.md §section-title)
    pub fn route_question(&self, issue: &QuestionIssue) -> Result<QuestionRoutingResult> {
        // Step 1: Check if Rodgers has already commented (skip if so)
        if self.has_rodgers_commented(issue) {
            return Ok(QuestionRoutingResult {
                answer_found: false,
                doc_link: None,
                comment: None,
                should_close: false,
            });
        }

        // Step 2: Search docs for relevant content
        let search_query = self.build_search_query(issue);
        let matches = doc_search::search_docs(&self.docs_path, &search_query, self.search_limit)?;

        if matches.is_empty() {
            // No answer found in docs
            return Ok(QuestionRoutingResult {
                answer_found: false,
                doc_link: None,
                comment: None,
                should_close: false,
            });
        }

        // Step 3: Use the best match
        let best_match = &matches[0];
        let section_title = best_match.section_title.as_deref();
        let doc_link = doc_search::format_doc_link(&best_match.path, section_title);

        // Step 4: Draft the comment content
        let comment = self.draft_comment(issue, &doc_link, &best_match.snippet)?;

        Ok(QuestionRoutingResult {
            answer_found: true,
            doc_link: Some(doc_link),
            comment: Some(comment),
            should_close: true, // Doc answer found - close if complete
        })
    }

    /// Route a question issue using LLM for comment drafting.
    pub fn route_question_with_llm(
        &self,
        issue: &QuestionIssue,
        llm_client: &LlmClient,
    ) -> Result<QuestionRoutingResult> {
        // Step 1: Check if Rodgers has already commented
        if self.has_rodgers_commented(issue) {
            return Ok(QuestionRoutingResult {
                answer_found: false,
                doc_link: None,
                comment: None,
                should_close: false,
            });
        }

        // Step 2: Search docs for relevant content
        let search_query = self.build_search_query(issue);
        let matches = doc_search::search_docs(&self.docs_path, &search_query, self.search_limit)?;

        if matches.is_empty() {
            return Ok(QuestionRoutingResult {
                answer_found: false,
                doc_link: None,
                comment: None,
                should_close: false,
            });
        }

        // Step 3: Use the best match
        let best_match = &matches[0];
        let section_title = best_match.section_title.as_deref();
        let doc_link = doc_search::format_doc_link(&best_match.path, section_title);

        // Step 4: Get relevant doc content for LLM summary
        let doc_content = self.get_doc_content(&best_match.path)?;

        // Step 5: Draft comment using LLM
        let comment = self.draft_comment_with_llm(llm_client, issue, &doc_link, &doc_content)?;

        Ok(QuestionRoutingResult {
            answer_found: true,
            doc_link: Some(doc_link),
            comment: Some(comment),
            should_close: true,
        })
    }

    /// Check if Rodgers has already commented on this issue.
    fn has_rodgers_commented(&self, issue: &QuestionIssue) -> bool {
        // Check if any prior comments are from Rodgers (bot)
        // In production, this would check the GitHub API for comment authors
        issue
            .prior_comments
            .iter()
            .any(|c| c.contains("Rodgers") || c.contains("🤖"))
    }

    /// Build a search query from the issue title and body.
    fn build_search_query(&self, issue: &QuestionIssue) -> String {
        // Extract key terms from title and body for search
        let combined = format!("{} {}", issue.title, issue.body);

        // Take the first 200 chars or up to first newline
        let truncated = combined.chars().take(200).collect::<String>();
        truncated.lines().next().unwrap_or(&truncated).to_string()
    }

    /// Draft a comment using the fixed template.
    fn draft_comment(
        &self,
        issue: &QuestionIssue,
        doc_link: &str,
        doc_summary: &str,
    ) -> Result<String> {
        Ok(format!(
            "Hi @{requestor}, thanks for reaching out!\n\n\
            The answer to your question is covered in [{doc_link}]().\n\n\
            {summary}\n\n\
            If this doesn't fully answer your question, just let me know and I'll dig further.\n\
            Really appreciate you asking.",
            requestor = issue.requestor,
            doc_link = doc_link,
            summary = doc_summary
        ))
    }

    /// Draft a comment using the LLM for more natural language.
    fn draft_comment_with_llm(
        &self,
        llm_client: &LlmClient,
        issue: &QuestionIssue,
        doc_link: &str,
        doc_content: &str,
    ) -> Result<String> {
        let mut conv = LlmConversation::with_system(prompts::DOC_ANSWER_SYSTEM_PROMPT);

        let user_prompt = prompts::draft_doc_answer_prompt(
            &issue.requestor,
            &issue.title,
            &issue.body,
            doc_link,
            &self.summarize_doc_for_prompt(doc_content),
        );
        conv.add_user(&user_prompt);

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| RogersError::Config(format!("Failed to create runtime: {}", e)))?;

        let comment = runtime.block_on(llm_client.complete(conv.messages()))?;

        Ok(comment.trim().to_string())
    }

    /// Summarize doc content for the LLM prompt (truncate to manageable size).
    fn summarize_doc_for_prompt(&self, content: &str) -> String {
        // Take first 500 chars as summary
        let truncated = content.chars().take(500).collect::<String>();
        if content.len() > 500 {
            format!("{}... (truncated)", truncated)
        } else {
            truncated
        }
    }

    /// Get the content of a documentation file.
    fn get_doc_content(&self, relative_path: &str) -> Result<String> {
        let full_path = self.docs_path.join(relative_path);
        Ok(std::fs::read_to_string(&full_path).map_err(|_| {
            RogersError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Could not read doc file: {}", full_path.display()),
            ))
        })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_docs(temp_dir: &TempDir, files: &[(&str, &str)]) -> std::path::PathBuf {
        let docs_dir = temp_dir.path().join("docs");
        std::fs::create_dir_all(&docs_dir).unwrap();

        for (filename, content) in files {
            let path = docs_dir.join(filename);
            std::fs::write(&path, content).unwrap();
        }

        docs_dir
    }

    fn create_test_issue() -> QuestionIssue {
        QuestionIssue {
            number: 1,
            title: "How do I install the software?".to_string(),
            body: "I want to install the package but I'm not sure how.".to_string(),
            requestor: "username".to_string(),
            labels: vec!["question".to_string()],
            prior_comments: vec![],
        }
    }

    #[test]
    fn test_route_question_finds_answer() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[(
                "getting-started.md",
                "# Installing\n\nInstallation is done via `cargo install` command.\n",
            )],
        );

        let router = QuestionRouter::new(docs_dir, 10);
        let issue = create_test_issue();

        let result = router.route_question(&issue).unwrap();

        assert!(result.answer_found);
        assert!(result.doc_link.is_some());
        assert!(result.comment.is_some());
        assert!(result.should_close);
    }

    #[test]
    fn test_route_question_no_answer() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[(
                "getting-started.md",
                "# Getting Started\n\nThis is about setup.\n",
            )],
        );

        let router = QuestionRouter::new(docs_dir, 10);
        let mut issue = create_test_issue();
        issue.title = "What is the meaning of life?".to_string();

        let result = router.route_question(&issue).unwrap();

        assert!(!result.answer_found);
        assert!(result.doc_link.is_none());
    }

    #[test]
    fn test_route_question_skips_if_already_commented() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(
            &temp_dir,
            &[(
                "getting-started.md",
                "# Installing\n\nInstallation via cargo.\n",
            )],
        );

        let router = QuestionRouter::new(docs_dir, 10);
        let mut issue = create_test_issue();
        issue.prior_comments = vec!["🤖 Rodgers has answered this.".to_string()];

        let result = router.route_question(&issue).unwrap();

        assert!(!result.answer_found);
    }

    #[test]
    fn test_build_search_query() {
        let temp_dir = TempDir::new().unwrap();
        let docs_dir = create_test_docs(&temp_dir, &[]);

        let router = QuestionRouter::new(docs_dir, 10);
        let issue = create_test_issue();

        let query = router.build_search_query(&issue);

        assert!(query.contains("install"));
        assert!(query.contains("software"));
    }

    #[test]
    fn test_doc_link_format() {
        let link = doc_search::format_doc_link("docs/getting-started.md", Some("Installation"));

        assert_eq!(link, "docs/getting-started.md §Installation");
    }
}
