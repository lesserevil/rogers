//! Doc-gap bead filing logic.
//!
//! This module handles filing documentation gap beads ONLY after
//! exhausting both docs and code search.
//!
//! Flow (ONLY when both fail):
//! 1. Doc search: no answer in docs/
//! 2. Code search: no answer in code (or question not implementation-related)
//! 3. File chore bead (rodgers:type=docs)
//! 4. Post acknowledgment comment
//! 5. Label issue 'needs-documentation', remove 'question'

use crate::beads::client::{BeadClient, BeadCreateResponse, RODGERS_TAG_DOCS};
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Acknowledgment comment template for doc-gap beads.
/// 4. Post a comment on the GitHub issue:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGapAcknowledgment {
    /// The username of the person who asked the question.
    pub requestor: String,
    /// The GitHub issue number.
    pub issue_number: u64,
    /// The bead ID that was filed.
    pub bead_id: String,
}

/// Fields required to file a doc-gap bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGapBeadRequest {
    /// One-line restatement of the question for the bead title.
    pub question_summary: String,
    /// Full question text from the issue.
    pub full_question: String,
    /// Full issue body with all context.
    pub issue_body: String,
    /// Link to the originating GitHub issue.
    pub discovered_from_issue: String,
    /// GitHub issue number.
    pub issue_number: u64,
    /// Username of the person who asked the question.
    pub requestor: String,
}

/// Result of filing a doc-gap bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGapBeadResult {
    /// The created bead information.
    pub bead: BeadCreateResponse,
    /// The acknowledgment comment to post.
    pub acknowledgment: DocGapAcknowledgment,
    /// Labels to apply: ['needs-documentation']
    pub add_labels: Vec<String>,
    /// Labels to remove: ['question']
    pub remove_labels: Vec<String>,
}

/// DocGapBeadHandler handles the filing of doc-gap beads.
/// Only files after exhausting both docs AND code search.
pub struct DocGapBeadHandler {
    bead_client: BeadClient,
}

impl DocGapBeadHandler {
    /// Create a new DocGapBeadHandler.
    pub fn new() -> Self {
        Self {
            bead_client: BeadClient::new(),
        }
    }

    /// File a doc-gap bead after both doc and code search fail.
    ///
    /// This should ONLY be called when:
    /// 1. Doc search found no answer
    /// 2. Code search found no answer OR question not implementation-related
    ///
    /// # Arguments
    /// * `request` - The doc-gap bead request with question details
    ///
    /// # Returns
    /// The bead result with acknowledgment details and label changes.
    pub fn file_doc_gap_bead(&self, request: DocGapBeadRequest) -> Result<DocGapBeadResult> {
        // Build the bead title
        let title = format!("Answer question: {}", request.question_summary);

        // Build comprehensive description with full question + context
        let description = build_bead_description(&request);

        // Build acceptance criteria: new doc section answering question
        let acceptance = build_acceptance(&request);

        // File the chore bead with rodgers:type=docs
        let bead = self.bead_client.file_doc_gap_bead(
            &title,
            &description,
            &request.discovered_from_issue,
            &acceptance,
        )?;

        // Build acknowledgment
        let acknowledgment = DocGapAcknowledgment {
            requestor: request.requestor,
            issue_number: request.issue_number,
            bead_id: bead.id.clone(),
        };

        // Labels to update
        let add_labels = vec!["needs-documentation".to_string()];
        let remove_labels = vec!["question".to_string()];

        Ok(DocGapBeadResult {
            bead,
            acknowledgment,
            add_labels,
            remove_labels,
        })
    }

    /// Generate the acknowledgment comment body for posting to GitHub.
    ///
    /// Per the plan:
    /// > "Hi @[requestor], thanks for the question! We do not currently have
    /// > documentation that answers this. We have opened a task to add an answer
    /// > to our documentation — it will be linked here when complete."
    pub fn generate_acknowledgment_comment(&self, acknowledgment: &DocGapAcknowledgment) -> String {
        format!(
            "Hi @{requestor}, thanks for the question! We do not currently have \
             documentation that answers this. We have opened a task to add an answer \
             to our documentation — it will be linked here when complete.\n\n\
             Tracking: bead #{bead_id}",
            requestor = acknowledgment.requestor,
            bead_id = acknowledgment.bead_id
        )
    }
}

impl Default for DocGapBeadHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the comprehensive bead description.
///
/// Includes:
/// - The full question text from the issue
/// - The full issue body with all context
/// - Reference to the source issue (discovered-from)
/// - What needs to be documented
fn build_bead_description(request: &DocGapBeadRequest) -> String {
    format!(
        r#"## Question

{question}

## Full Issue Context

{issue_body}

## Source

{discovered_from_issue}

## What Needs Documentation

This issue was filed as a question but no documentation currently exists
that answers it. A new section should be added to the appropriate doc
file that answers the above question for future community members.

## Acceptance Criteria

A new section in the relevant documentation file that answers the question
above. The section must be linked from the GitHub issue when filed.
"#, 
        question = request.full_question,
        issue_body = request.issue_body,
        discovered_from_issue = request.discovered_from_issue
    )
}

/// Build the acceptance criteria for the bead.
///
/// Per the plan:
/// > Acceptance: a new section in the relevant doc that answers the question;
/// > the section must be linked from the issue when filed
fn build_acceptance(request: &DocGapBeadRequest) -> String {
    format!(
        "Add a new section to the relevant documentation file that answers the question: '{}'. \
         The section must be linked from issue #{issue_number} when complete.",
        request.question_summary,
        issue_number = request.issue_number
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request() -> DocGapBeadRequest {
        DocGapBeadRequest {
            question_summary: "How does authentication work".to_string(),
            full_question: "Can you explain how authentication works in the system?".to_string(),
            issue_body: "I'm trying to understand how the login flow works.".to_string(),
            discovered_from_issue: "#42".to_string(),
            issue_number: 42,
            requestor: "testuser".to_string(),
        }
    }

    #[test]
    fn test_build_bead_description() {
        let request = create_test_request();
        let desc = build_bead_description(&request);
        
        assert!(desc.contains("How does authentication work"));
        assert!(desc.contains("#42"));
        assert!(desc.contains("What Needs Documentation"));
    }

    #[test]
    fn test_build_acceptance() {
        let request = create_test_request();
        let acceptance = build_acceptance(&request);
        
        assert!(acceptance.contains("#42"));
        assert!(acceptance.contains("authentication"));
    }

    #[test]
    fn test_doc_gap_handler_generates_acknowledgment() {
        let handler = DocGapBeadHandler::new();
        let ack = DocGapAcknowledgment {
            requestor: "testuser".to_string(),
            issue_number: 42,
            bead_id: "test-bead-123".to_string(),
        };
        
        let comment = handler.generate_acknowledgment_comment(&ack);
        
        assert!(comment.contains("@testuser"));
        assert!(comment.contains("test-bead-123"));
        assert!(comment.contains("does not currently have documentation"));
    }
}