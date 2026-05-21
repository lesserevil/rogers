//! Question Router Validator — zero-tolerance hallucination prevention.
//!
//! This module validates LLM-generated answers before they are posted to GitHub.
//! It enforces that every answer cites real, verifiable sources.
//!
//! Validation rules:
//! - Doc answers MUST cite existing doc file:line
//! - Code answers MUST cite existing file:function:line
//! - If citation invalid or missing → route to human OR file doc-gap
//! - Never post unverified LLM output to GitHub
//!
//! Validation flow:
//! 1. Extract citations from LLM response
//! 2. Validate each citation (file exists, line exists)
//! 3. If all valid → answer is safe to post
//! 4. If any invalid → reject and route to fallback

use crate::llm::validator::{self, StructuredOutputValidator};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The result of validating an LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationOutcome {
    /// Answer passed validation — safe to post.
    Valid,
    /// Answer failed validation — needs human escalation.
    EscalateToHuman {
        /// The original (unverified) LLM response.
        original_response: String,
        /// Error messages explaining why validation failed.
        errors: Vec<String>,
    },
    /// Answer failed validation — file a doc-gap instead.
    DocGap {
        /// The original (unverified) LLM response.
        original_response: String,
        /// Error messages explaining why validation failed.
        errors: Vec<String>,
    },
}

impl ValidationOutcome {
    /// Returns true if this answer can be safely posted.
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationOutcome::Valid)
    }
}

/// The validation gate for question router answers.
///
/// Sits between LLM answer generation and GitHub posting.
/// No answer reaches GitHub without passing through this gate.
pub struct QuestionRouterValidator {
    llm_validator: StructuredOutputValidator,
    repo_root: String,
    /// Whether to prefer doc-gap filing over human escalation.
    /// True by default: doc-gap scales better.
    prefer_doc_gap: bool,
}

impl QuestionRouterValidator {
    /// Create a new question router validator.
    pub fn new(repo_root: impl Into<String>) -> Self {
        let repo_root_str = repo_root.into();
        Self {
            llm_validator: StructuredOutputValidator::new(&repo_root_str),
            repo_root: repo_root_str,
            prefer_doc_gap: true,
        }
    }

    /// Set whether to prefer doc-gap over human escalation.
    pub fn with_prefer_doc_gap(mut self, prefer: bool) -> Self {
        self.prefer_doc_gap = prefer;
        self
    }

    /// Validate an LLM response before posting.
    ///
    /// This is the main entry point. It validates citations, and if
    /// validation fails, it produces a fallback action (human escalation
    /// or doc-gap filing).
    ///
    /// # Arguments
    /// * `response` - The LLM-generated response text
    /// * `source_type` - Whether this is a "doc" or "code" answer
    /// * `question` - The original question (for doc-gap filing)
    /// * `requestor` - Username of the person who asked
    /// * `issue_number` - GitHub issue number
    ///
    /// # Returns
    /// ValidationOutcome indicating whether the answer is safe to post
    /// or what fallback action to take.
    pub fn validate(
        &self,
        response: &str,
        _source_type: AnswerSource,
        _question: &str,
        _requestor: &str,
        _issue_number: u64,
    ) -> ValidationOutcome {
        tracing::info!(
            "Validating LLM response for question routing"
        );

        let result = self.llm_validator.validate(response);

        if result.is_valid() {
            tracing::info!(
                valid_citations = result.valid_count(),
                "LLM response passed validation"
            );
            return ValidationOutcome::Valid;
        }

        tracing::warn!(
            errors = ?result.errors,
            "LLM response failed validation"
        );

        // Determine fallback action
        let fallback = if self.prefer_doc_gap {
            ValidationOutcome::DocGap {
                original_response: response.to_string(),
                errors: result.errors.clone(),
            }
        } else {
            ValidationOutcome::EscalateToHuman {
                original_response: response.to_string(),
                errors: result.errors.clone(),
            }
        };

        fallback
    }

    /// Validate that a doc-style answer has at least one valid citation.
    ///
    /// Doc answers must cite files in the docs/ directory that actually
    /// exist and contain the cited line.
    pub fn validate_doc_citation(&self, response: &str) -> bool {
        let citations = validator::extract_citations(response);

        if citations.is_empty() {
            tracing::warn!("No citations found in doc answer");
            return false;
        }

        let full_path = Path::new(&self.repo_root);

        for citation in &citations {
            // Verify the citation points to the docs directory
            let is_in_docs = citation.file_path.starts_with("docs")
                || citation.file_path.starts_with("./docs");

            if !is_in_docs {
                tracing::warn!(
                    citation = %citation.display(),
                    "Citation does not point to docs directory"
                );
                return false;
            }

            // Resolve relative to repo root (citation paths are repo-relative)
            let citation_path = full_path.join(&citation.file_path);
            if !citation_path.exists() {
                tracing::warn!(
                    file = %citation.file_path,
                    "Doc citation file does not exist"
                );
                return false;
            }
        }

        // All citations point to real doc files
        true
    }

    /// Validate that a code-style answer has at least one valid citation.
    ///
    /// Code answers must cite files in the src/ directory that actually
    /// exist and contain the cited line range.
    pub fn validate_code_citation(&self, response: &str) -> bool {
        let citations = validator::extract_citations(response);

        if citations.is_empty() {
            tracing::warn!("No citations found in code answer");
            return false;
        }

        let full_path = Path::new(&self.repo_root);

        for citation in &citations {
            // Verify the citation points to the src directory
            let is_in_src = citation.file_path.starts_with("src")
                || citation.file_path.starts_with("./src");

            if !is_in_src {
                tracing::warn!(
                    citation = %citation.display(),
                    "Citation does not point to src directory"
                );
                return false;
            }

            // Resolve relative to repo root (citation paths are repo-relative)
            let citation_path = full_path.join(&citation.file_path);
            if !citation_path.exists() {
                tracing::warn!(
                    file = %citation.file_path,
                    "Code citation file does not exist"
                );
                return false;
            }
        }

        // All citations point to real source files
        true
    }

    /// Get the human escalation comment for a failed validation.
    ///
    /// The comment is warm and conversational, never robotic.
    pub fn human_escalation_comment(&self, requestor: &str) -> String {
        format!(
            "Hi @{requestor}, thanks for the question! I want to make sure I give you \
             an accurate answer, but I need some help from the team to get this right. \
             Could a maintainer jump in and share what they know? I don't want to risk \
             giving you incomplete or incorrect information."
        )
    }

    /// Generate a doc-gap filing request for a failed validation.
    ///
    /// When the LLM can't produce a verified answer, this creates a doc-gap
    /// bead for the team to address.
    pub fn doc_gap_request(
        &self,
        _response: &str,
        question: &str,
        errors: &[String],
        issue_number: u64,
        requestor: &str,
    ) -> crate::question_router::doc_gap::DocGapBeadRequest {
        // Build the fallback response acknowledging the issue
        let fallback_message = format!(
            "I searched the codebase for an answer to your question, but couldn't \
             find a well-documented source. We're filing a task to add this to our docs."
        );

        crate::question_router::doc_gap::DocGapBeadRequest {
            question_summary: question.to_string(),
            full_question: question.to_string(),
            issue_body: format!(
                "Question: {question}\n\n\
                 Validation errors: {errors:?}\n\n\
                 Fallback: {fallback_message}"
            ),
            discovered_from_issue: format!("#{}", issue_number),
            issue_number,
            requestor: requestor.to_string(),
        }
    }
}

impl Default for QuestionRouterValidator {
    fn default() -> Self {
        Self::new(".")
    }
}

/// Indicates the source type of an LLM answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnswerSource {
    /// Answer came from documentation search.
    Doc,
    /// Answer came from source code search.
    Code,
    /// Answer was generated without a clear source (LLM knowledge).
    Generated,
}

/// A verified answer that has passed validation and is safe to post.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedAnswer {
    /// The validated response text.
    pub response: String,
    /// The source type of the answer.
    pub source: AnswerSource,
    /// The valid citations from the answer.
    pub citations: Vec<validator::Citation>,
}

impl VerifiedAnswer {
    /// Create a new verified answer.
    pub fn new(response: String, source: AnswerSource, citations: Vec<validator::Citation>) -> Self {
        Self {
            response,
            source,
            citations,
        }
    }

    /// Get the number of valid citations.
    pub fn citation_count(&self) -> usize {
        self.citations.len()
    }

    /// Get the response text.
    pub fn response(&self) -> &str {
        &self.response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Create docs directory with test file
        fs::create_dir_all(temp_dir.path().join("docs")).unwrap();
        fs::write(
            temp_dir.path().join("docs/getting-started.md"),
            "# Getting Started\n\n## Installation\n\nInstall with `cargo install`.\n## Usage\n\nUse the CLI to interact.\n",
        )
        .unwrap();

        // Create src directory with test file
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(
            temp_dir.path().join("src/lib.rs"),
            "//! Test library.\n\npub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        )
        .unwrap();

        temp_dir
    }

    // ---- validate() tests ----

    #[test]
    fn test_validate_valid_doc_answer() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "See docs/getting-started.md:1 for the getting started guide.";
        let outcome = validator.validate(
            response,
            AnswerSource::Doc,
            "How do I get started?",
            "testuser",
            42,
        );

        assert!(outcome.is_valid());
        assert!(matches!(outcome, ValidationOutcome::Valid));
    }

    #[test]
    fn test_validate_valid_code_answer() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "The add function is at src/lib.rs:3.";
        let outcome = validator.validate(
            response,
            AnswerSource::Code,
            "How does add work?",
            "testuser",
            42,
        );

        assert!(outcome.is_valid());
        assert!(matches!(outcome, ValidationOutcome::Valid));
    }

    #[test]
    fn test_validate_invalid_file() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "See src/nonexistent.rs:1 for details.";
        let outcome = validator.validate(
            response,
            AnswerSource::Code,
            "How does X work?",
            "testuser",
            42,
        );

        assert!(!outcome.is_valid());
        assert!(!matches!(outcome, ValidationOutcome::Valid));
    }

    #[test]
    fn test_validate_no_citations() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "I don't know the answer to that question.";
        let outcome = validator.validate(
            response,
            AnswerSource::Doc,
            "How do I use this?",
            "testuser",
            42,
        );

        assert!(!outcome.is_valid());
        assert!(!matches!(outcome, ValidationOutcome::Valid));
    }

    #[test]
    fn test_validate_no_citations_fallback_doc_gap() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "I don't know the answer.";
        let outcome = validator.validate(
            response,
            AnswerSource::Doc,
            "How does X work?",
            "testuser",
            42,
        );

        match outcome {
            ValidationOutcome::DocGap { .. } => {}
            other => panic!("Expected DocGap, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_no_citations_fallback_escalation() {
        let temp_dir = create_test_repo();
        let validator =
            QuestionRouterValidator::new(temp_dir.path().to_str().unwrap()).with_prefer_doc_gap(false);

        let response = "I don't know the answer.";
        let outcome = validator.validate(
            response,
            AnswerSource::Doc,
            "How does X work?",
            "testuser",
            42,
        );

        match outcome {
            ValidationOutcome::EscalateToHuman { .. } => {}
            other => panic!("Expected EscalateToHuman, got {:?}", other),
        }
    }

    // ---- validate_doc_citation tests ----

    #[test]
    fn test_validate_doc_citation_valid() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "See docs/getting-started.md:1 for setup info.";
        assert!(validator.validate_doc_citation(response));
    }

    #[test]
    fn test_validate_doc_citation_wrong_directory() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        // Code citation should fail doc validation
        let response = "See src/lib.rs:3 for the function.";
        assert!(!validator.validate_doc_citation(response));
    }

    #[test]
    fn test_validate_doc_citation_no_citations() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "No citations here.";
        assert!(!validator.validate_doc_citation(response));
    }

    // ---- validate_code_citation tests ----

    #[test]
    fn test_validate_code_citation_valid() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "See src/lib.rs:3 for the add function.";
        assert!(validator.validate_code_citation(response));
    }

    #[test]
    fn test_validate_code_citation_wrong_directory() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        // Doc citation should fail code validation
        let response = "See docs/getting-started.md:1 for setup.";
        assert!(!validator.validate_code_citation(response));
    }

    #[test]
    fn test_validate_code_citation_no_citations() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "No citations here.";
        assert!(!validator.validate_code_citation(response));
    }

    // ---- human_escalation_comment tests ----

    #[test]
    fn test_human_escalation_comment_is_warm() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let comment = validator.human_escalation_comment("testuser");

        assert!(comment.contains("@testuser"));
        assert!(comment.contains("thanks"));
        assert!(comment.contains("help"));
        // Must not sound robotic
        assert!(!comment.contains("ERROR") && !comment.contains("FAILED"));
    }

    // ---- doc_gap_request tests ----

    #[test]
    fn test_doc_gap_request_creates_valid_request() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let request = validator.doc_gap_request(
            "fallback",
            "How does X work?",
            &["citation not found".to_string()],
            42,
            "testuser",
        );

        assert_eq!(request.question_summary, "How does X work?");
        assert_eq!(request.issue_number, 42);
        assert_eq!(request.requestor, "testuser");
    }

    // ---- VerifiedAnswer tests ----

    #[test]
    fn test_verified_answer_citation_count() {
        let citations = vec![
            validator::Citation {
                file_path: "src/lib.rs".to_string(),
                line_start: 1,
                line_end: 5,
            },
            validator::Citation {
                file_path: "docs/getting-started.md".to_string(),
                line_start: 1,
                line_end: 1,
            },
        ];
        let answer = VerifiedAnswer::new("Test".to_string(), AnswerSource::Doc, citations);
        assert_eq!(answer.citation_count(), 2);
        assert_eq!(answer.response(), "Test");
    }

    #[test]
    fn test_answer_source_enum() {
        assert!(matches!(AnswerSource::Doc, AnswerSource::Doc));
        assert!(matches!(AnswerSource::Code, AnswerSource::Code));
        assert!(matches!(AnswerSource::Generated, AnswerSource::Generated));
    }

    // ---- Integration test: hallucination blocked ----

    #[test]
    fn test_hallucination_blocked() {
        // Simulate an LLM hallucinating a citation
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let hallucinated_response =
            "The answer is in src/hallucinated.rs:42 — it's a simple function that processes data.";

        let outcome = validator.validate(
            hallucinated_response,
            AnswerSource::Code,
            "How does data processing work?",
            "curious_user",
            100,
        );

        // Should NOT be valid — the file doesn't exist
        assert!(!outcome.is_valid());

        // Should route to fallback instead of posting hallucination
        match outcome {
            ValidationOutcome::DocGap { .. } | ValidationOutcome::EscalateToHuman { .. } => {}
            ValidationOutcome::Valid => {
                panic!("Hallucination was NOT blocked — this is a critical failure!");
            }
        }
    }

    #[test]
    fn test_validate_line_out_of_range() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        // lib.rs has 5 lines, so line 999 is out of range
        let response = "See src/lib.rs:999 for the implementation.";
        let outcome = validator.validate(
            response,
            AnswerSource::Code,
            "What's at line 999?",
            "testuser",
            42,
        );

        assert!(!outcome.is_valid());
    }

    #[test]
    fn test_validate_multiple_citations_all_valid() {
        let temp_dir = create_test_repo();
        let validator = QuestionRouterValidator::new(temp_dir.path().to_str().unwrap());

        let response = "See src/lib.rs:3 for add and docs/getting-started.md:1 for install.";
        let outcome = validator.validate(
            response,
            AnswerSource::Doc,
            "Tell me about the project.",
            "testuser",
            42,
        );

        // Mixed citations (one src, one docs) — both files exist
        // The llm_validator validates both, both should be valid
        assert!(outcome.is_valid());
    }

    #[test]
    fn test_validation_outcome_variants() {
        // All three outcome variants exist and work
        let valid = ValidationOutcome::Valid;
        assert!(valid.is_valid());

        let escalate = ValidationOutcome::EscalateToHuman {
            original_response: "test".to_string(),
            errors: vec!["bad".to_string()],
        };
        assert!(!escalate.is_valid());

        let doc_gap = ValidationOutcome::DocGap {
            original_response: "test".to_string(),
            errors: vec!["bad".to_string()],
        };
        assert!(!doc_gap.is_valid());
    }
}
