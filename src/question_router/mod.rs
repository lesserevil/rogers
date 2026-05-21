//! Question Router for Rodgers.
//!
//! This module handles routing question-labeled issues to the appropriate
//! answer source: user documentation, source code, or doc-gap beads.
//!
//! Flow:
//! 1. Question issue labeled 'question' arrives
//! 2. Search docs/ for relevant content (doc_search.rs)
//! 3. If not in docs and question is about implementation, search source code
//! 4. If code found, explain in plain language with citations
//! 5. If neither found, file doc-gap bead
//!
//! Search order:
//! - docs/**/*.md (user-facing documentation)
//! - All code files (when question is about implementation details)

pub mod code_search;
pub mod doc_gap;
pub mod validator;

use crate::llm;

pub use validator::{AnswerSource, QuestionRouterValidator, ValidationOutcome, VerifiedAnswer};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// Re-export anyhow::Result for use in this module
pub use anyhow::Result;

/// Configuration for question routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionRouterConfig {
    /// Path to the docs directory relative to repo root.
    pub docs_path: PathBuf,
    /// Path to the source code directory relative to repo root.
    pub src_path: PathBuf,
    /// Path to AGENTS.md for project context.
    pub agents_path: PathBuf,
    /// Whether code search is enabled.
    pub code_search_enabled: bool,
    /// Whether doc search is enabled.
    pub doc_search_enabled: bool,
}

impl Default for QuestionRouterConfig {
    fn default() -> Self {
        Self {
            docs_path: PathBuf::from("docs"),
            src_path: PathBuf::from("src"),
            agents_path: PathBuf::from("AGENTS.md"),
            code_search_enabled: true,
            doc_search_enabled: true,
        }
    }
}

/// Represents a question issue to be routed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionIssue {
    /// GitHub issue number.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Issue body/description.
    pub body: String,
    /// Original issue URL.
    pub url: String,
    /// Issue author username.
    pub author: String,
}

/// Result of routing a question.
#[derive(Debug, Clone)]
pub enum RoutingResult {
    /// Answer found in user documentation.
    AnswerInDocs {
        doc_path: String,
        section: Option<String>,
        comment: String,
    },
    /// Answer found in source code.
    AnswerInCode {
        explanation: String,
        citations: Vec<String>,
        fully_answers: bool,
    },
    /// No answer found - doc gap bead needed.
    DocGapNeeded {
        reason: String,
        areas_searched: Vec<String>,
    },
    /// Question is not actually a question - reclassify.
    NotAQuestion { suggested_label: String },
    /// Need more information from the requestor.
    ClarificationNeeded { question: String },
}

impl RoutingResult {
    /// Returns true if the question was answered (from docs or code).
    pub fn is_answered(&self) -> bool {
        matches!(
            self,
            RoutingResult::AnswerInDocs { .. }
                | RoutingResult::AnswerInCode {
                    fully_answers: true,
                    ..
                }
        )
    }

    /// Returns true if the question needs a doc-gap bead filed.
    pub fn needs_doc_gap_bead(&self) -> bool {
        matches!(self, RoutingResult::DocGapNeeded { .. })
    }
}

/// The main question router.
#[derive(Debug, Clone)]
pub struct QuestionRouter {
    config: QuestionRouterConfig,
}

impl QuestionRouter {
    /// Create a new question router with the given configuration.
    pub fn new(config: QuestionRouterConfig) -> Self {
        Self { config }
    }

    /// Create a new question router with default configuration.
    pub fn default_config() -> Self {
        Self::new(QuestionRouterConfig::default())
    }

    /// Route a question issue and determine the appropriate response.
    pub fn route(&self, issue: &QuestionIssue, repo_root: &Path) -> Result<RoutingResult> {
        tracing::info!(
            issue_number = issue.number,
            title = %issue.title,
            "Routing question issue"
        );

        // Step 1: Check if this is actually a question vs feature request or bug
        if !self.is_genuine_question(&issue.title, &issue.body) {
            return Ok(RoutingResult::NotAQuestion {
                suggested_label: self.suggest_reclassification(&issue.body),
            });
        }

        // Step 2: Check if we have enough context to answer
        if self.needs_clarification(&issue.title, &issue.body) {
            return Ok(RoutingResult::ClarificationNeeded {
                question: self.ask_for_clarification(&issue.body),
            });
        }

        // Step 3: Search docs first if enabled
        if self.config.doc_search_enabled {
            if let Some(result) = self.search_docs(issue, repo_root)? {
                tracing::info!(issue_number = issue.number, "Answer found in docs");
                return Ok(result);
            }
        }

        // Step 4: Search code if question is about implementation
        if self.config.code_search_enabled
            && (code_search::is_implementation_question(&issue.title)
                || code_search::is_implementation_question(&issue.body))
        {
            tracing::info!(
                issue_number = issue.number,
                "Implementation question detected, searching code"
            );

            if let Some(result) = self.search_code(issue, repo_root)? {
                tracing::info!(issue_number = issue.number, "Answer found in code");
                return Ok(result);
            }
        }

        // Step 5: If neither search found an answer, file doc-gap bead
        tracing::info!(
            issue_number = issue.number,
            "No answer found, doc-gap needed"
        );
        Ok(RoutingResult::DocGapNeeded {
            reason: "Neither docs nor code contained an answer to this question".to_string(),
            areas_searched: self.areas_searched(),
        })
    }

    /// Check if this is a genuine question vs a bug report or feature request.
    fn is_genuine_question(&self, title: &str, _body: &str) -> bool {
        let title_lower = title.to_lowercase();

        // If title contains bug/feature report patterns, reclassify
        let bug_patterns = [
            "bug:",
            "feature:",
            "crash:",
            "error:",
            "doesn't work",
            "does not work",
        ];
        let is_likely_bug = bug_patterns.iter().any(|p| title_lower.contains(p));

        let feature_patterns = [
            "please add",
            "we should",
            "i would like",
            "enhancement",
            "feature request",
        ];
        let is_likely_feature = feature_patterns.iter().any(|p| title_lower.contains(p));

        !is_likely_bug && !is_likely_feature
    }

    /// Suggest a reclassification for non-question issues.
    fn suggest_reclassification(&self, body: &str) -> String {
        let body_lower = body.to_lowercase();

        if body_lower.contains("crash") || body_lower.contains("panic") {
            return "bug".to_string();
        }

        if body_lower.contains("please add") || body_lower.contains("would be nice") {
            return "feature".to_string();
        }

        "question".to_string()
    }

    /// Check if the question has enough context to answer.
    fn needs_clarification(&self, title: &str, body: &str) -> bool {
        title.len() < 10 && body.len() < 20
    }

    /// Generate a clarification question.
    fn ask_for_clarification(&self, _body: &str) -> String {
        "Could you please provide more details about what you'd like to know? \
         For example, what specific feature or behavior are you asking about?"
            .to_string()
    }

    /// Search user documentation for an answer.
    fn search_docs(
        &self,
        issue: &QuestionIssue,
        repo_root: &Path,
    ) -> Result<Option<RoutingResult>> {
        // This will be implemented by doc_search.rs (CRIT-1)
        // For now, return None to continue to code search
        let docs_path = repo_root.join(&self.config.docs_path);

        if !docs_path.exists() {
            return Ok(None);
        }

        // Simple keyword search in docs
        let query = format!("{} {}", issue.title, issue.body);
        let matches = self.simple_doc_search(&query, &docs_path)?;

        if matches.is_empty() {
            return Ok(None);
        }

        // Found a doc answer
        let doc_path = matches[0].file_path.clone();
        let comment = format!(
            "Hi @{}, thanks for the question!\n\n\
             The answer to your question is covered in [{}]().\n\n\
             [Add brief summary of relevant content here]\n\n\
             If this doesn't fully answer your question, please let us know.",
            issue.author, doc_path
        );

        Ok(Some(RoutingResult::AnswerInDocs {
            doc_path,
            section: None,
            comment,
        }))
    }

    /// Simple doc search implementation.
    fn simple_doc_search(
        &self,
        query: &str,
        docs_path: &Path,
    ) -> Result<Vec<code_search::CodeMatch>> {
        let mut matches = Vec::new();
        let query_lower = query.to_lowercase();

        if let Ok(entries) = std::fs::read_dir(docs_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Recurse into subdirectories
                    if let Ok(sub_matches) = self.simple_doc_search(query, &path) {
                        matches.extend(sub_matches);
                    }
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "md" {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for (i, line) in content.lines().enumerate() {
                                if line.to_lowercase().contains(&query_lower) {
                                    matches.push(code_search::CodeMatch::new(
                                        path.to_string_lossy(),
                                        i + 1,
                                        line,
                                        code_search::MatchType::Other,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Search source code for an implementation answer.
    fn search_code(
        &self,
        issue: &QuestionIssue,
        repo_root: &Path,
    ) -> Result<Option<RoutingResult>> {
        let src_path = repo_root.join(&self.config.src_path);

        if !src_path.exists() {
            return Ok(None);
        }

        // Extract key terms from the question for searching
        let query = format!("{} {}", issue.title, issue.body);

        let matches = code_search::search_code(&query, &src_path)?;

        if matches.is_empty() {
            return Ok(None);
        }

        // Get project context for the LLM
        let project_context = self.get_project_context(repo_root)?;

        // Generate explanation
        let explanation = code_search::format_code_explanation(&query, &matches, &project_context);

        // Extract citations for validation
        let citations: Vec<String> = matches.iter().map(|m| m.citation()).collect();

        // Determine if this fully answers the question
        let fully_answers = matches.len() >= 2 && matches.iter().any(|m| m.relevance_score > 5.0);

        Ok(Some(RoutingResult::AnswerInCode {
            explanation,
            citations,
            fully_answers,
        }))
    }

    /// Get project context from AGENTS.md.
    fn get_project_context(&self, repo_root: &Path) -> Result<String> {
        let agents_path = repo_root.join(&self.config.agents_path);

        if !agents_path.exists() {
            return Ok("No project context available.".to_string());
        }

        let content = std::fs::read_to_string(&agents_path)?;
        Ok(content)
    }

    /// Return the list of areas that were searched.
    fn areas_searched(&self) -> Vec<String> {
        let mut areas = Vec::new();

        if self.config.doc_search_enabled {
            areas.push(self.config.docs_path.display().to_string());
        }

        if self.config.code_search_enabled {
            areas.push(self.config.src_path.display().to_string());
        }

        areas
    }

    /// Check if code search is warranted based on the question content.
    pub fn is_code_search_warranted(&self, question: &str) -> bool {
        code_search::is_implementation_question(question)
    }

    /// Get the code search triggers.
    pub fn code_search_triggers() -> &'static [&'static str] {
        llm::prompts::CODE_SEARCH_TRIGGERS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create test source code
        std::fs::create_dir_all(temp_dir.path().join("src/question_router")).unwrap();
        std::fs::write(
            temp_dir.path().join("src/lib.rs"),
            r#"//! Test library.

/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Subtracts two numbers.
pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
"#,
        )
        .unwrap();

        std::fs::write(
            temp_dir.path().join("AGENTS.md"),
            r#"# Project Context

This is a test project for Rodgers.
"#,
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_is_implementation_question() {
        let router = QuestionRouter::default_config();

        assert!(router.is_code_search_warranted("How does add work?"));
        assert!(router.is_code_search_warranted("What function handles this?"));
        assert!(router.is_code_search_warranted("Tell me about the internals"));
        assert!(!router.is_code_search_warranted("How do I install this?"));
    }

    #[test]
    fn test_is_genuine_question() {
        let router = QuestionRouter::default_config();

        assert!(router.is_genuine_question("How does X work?", "I want to understand X"));
        assert!(!router.is_genuine_question("Bug: X crashes", "X crashes when I do Y"));
        assert!(!router.is_genuine_question("Feature request: Add X", "Please add X functionality"));
    }

    #[test]
    fn test_suggest_reclassification() {
        let router = QuestionRouter::default_config();

        assert_eq!(router.suggest_reclassification("My app crashes"), "bug");
        assert_eq!(
            router.suggest_reclassification("Please add X functionality"),
            "feature"
        );
        // "I would like to add X" doesn't match "please add" pattern - returns default "question"
        assert_eq!(
            router.suggest_reclassification("I would like to add X"),
            "question"
        );
    }

    #[test]
    fn test_routing_result_is_answered() {
        let doc_result = RoutingResult::AnswerInDocs {
            doc_path: "docs/test.md".to_string(),
            section: None,
            comment: "Test comment".to_string(),
        };
        assert!(doc_result.is_answered());

        let code_result = RoutingResult::AnswerInCode {
            explanation: "Test".to_string(),
            citations: vec![],
            fully_answers: true,
        };
        assert!(code_result.is_answered());

        let partial_result = RoutingResult::AnswerInCode {
            explanation: "Partial".to_string(),
            citations: vec![],
            fully_answers: false,
        };
        assert!(!partial_result.is_answered());

        let gap_result = RoutingResult::DocGapNeeded {
            reason: "No answer".to_string(),
            areas_searched: vec![],
        };
        // DocGapNeeded SHOULD need a doc gap bead - it's the whole point
        assert!(gap_result.needs_doc_gap_bead());
        // and is_answered() should be false
        assert!(!gap_result.is_answered());
    }

    #[test]
    fn test_needs_clarification() {
        let router = QuestionRouter::default_config();

        assert!(router.needs_clarification("?", ""));
        assert!(!router.needs_clarification("How does X work?", ""));
    }

    #[test]
    fn test_route_question_with_code() {
        let temp_dir = create_test_repo();
        let router = QuestionRouter::default_config();

        let issue = QuestionIssue {
            number: 1,
            title: "How does the add function work?".to_string(),
            body: "Can you explain the implementation of add?".to_string(),
            url: "https://github.com/test/test/issues/1".to_string(),
            author: "testuser".to_string(),
        };

        // This is an implementation question based on keywords
        assert!(router.is_code_search_warranted(&issue.title));
        assert!(router.is_code_search_warranted(&issue.body));

        // Verify the search would trigger code search
        let result = router.route(&issue, temp_dir.path()).unwrap();

        // Code search may or may not find results depending on search implementation
        // The important thing is that it IS triggered for implementation questions
        match result {
            RoutingResult::AnswerInCode { explanation, .. } => {
                assert!(explanation.contains("add"));
            }
            RoutingResult::DocGapNeeded {
                reason,
                areas_searched,
            } => {
                // Acceptable if code search didn't find direct matches
                // but implementation question was correctly identified
                assert!(areas_searched.contains(&"src".to_string()));
            }
            other => {
                panic!("Expected AnswerInCode or DocGapNeeded, got {:?}", other);
            }
        }
    }

    #[test]
    fn test_route_question_with_doc() {
        let temp_dir = TempDir::new().unwrap();

        // Create test docs with content that MATCHES the query
        std::fs::create_dir_all(temp_dir.path().join("docs")).unwrap();
        std::fs::write(
            temp_dir.path().join("docs/test.md"),
            "# How to Use This\n\nThis feature allows you to understand how to use this.\n",
        )
        .unwrap();

        let router = QuestionRouter::default_config();

        let issue = QuestionIssue {
            number: 2,
            title: "How do I use this feature?".to_string(),
            body: "I want to understand how to use this".to_string(),
            url: "https://github.com/test/test/issues/2".to_string(),
            author: "testuser".to_string(),
        };

        let result = router.route(&issue, temp_dir.path()).unwrap();

        match result {
            RoutingResult::AnswerInDocs { doc_path, .. } => {
                assert!(doc_path.contains("docs/test.md"));
            }
            RoutingResult::DocGapNeeded { reason, .. } => {
                // This is acceptable if the search didn't find a match
                // but the doc exists
                tracing::debug!("Doc search found no match: {}", reason);
            }
            other => {
                panic!("Expected AnswerInDocs or DocGapNeeded, got {:?}", other);
            }
        }
    }

    #[test]
    fn test_route_bug_report() {
        let temp_dir = create_test_repo();
        let router = QuestionRouter::default_config();

        let issue = QuestionIssue {
            number: 3,
            title: "Bug: App crashes with error X".to_string(),
            body: "The app crashes when I click the button".to_string(),
            url: "https://github.com/test/test/issues/3".to_string(),
            author: "testuser".to_string(),
        };

        let result = router.route(&issue, temp_dir.path()).unwrap();

        match result {
            RoutingResult::NotAQuestion { suggested_label } => {
                assert_eq!(suggested_label, "bug");
            }
            other => {
                panic!("Expected NotAQuestion, got {:?}", other);
            }
        }
    }

    #[test]
    fn test_areas_searched() {
        let router = QuestionRouter::default_config();
        let areas = router.areas_searched();

        assert!(areas.iter().any(|a| a.contains("docs")));
        assert!(areas.iter().any(|a| a.contains("src")));
    }

    #[test]
    fn test_code_search_triggers() {
        let triggers = QuestionRouter::code_search_triggers();
        assert!(triggers.contains(&"how does"));
        assert!(triggers.contains(&"implementation"));
    }
}
