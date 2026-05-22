//! LLM-based issue classifier.
//!
//! Uses the LLM to classify GitHub issues and determine completeness.

use rogers_core::error::{Result, RogersError};
use rogers_github::models::Issue;
use rogers_llm::client::{ChatMessage, ChatRequest, LlmClient};
use rogers_llm::prompts::{ClassificationPrompt, IssueMetadata};
use rogers_llm::validator::{ClassificationOutput, OutputValidator, ValidationResult};

/// Issue classifier using LLM.
#[derive(Debug, Clone)]
pub struct Classifier {
    /// LLM client.
    llm: LlmClient,
    /// Output validator.
    validator: OutputValidator,
    /// Model name.
    model: String,
}

impl Classifier {
    /// Create a new classifier from LLM config.
    pub fn new(llm: LlmClient) -> Self {
        Self {
            llm,
            validator: OutputValidator::new(),
            model: String::new(),
        }
    }

    /// Classify a GitHub issue.
    pub async fn classify(
        &self,
        issue: &Issue,
        domain_context: Option<&str>,
    ) -> Result<ClassificationResult> {
        // Convert issue to metadata
        let metadata = Self::issue_to_metadata(issue);

        // Build the classification prompt
        let prompt = ClassificationPrompt::for_classification(&metadata, domain_context);

        // Send to LLM
        let request = self.build_request(&prompt);
        let response = self.llm.chat(request).await?;

        // Parse and validate the response
        let content = &response.choices[0].message.content;
        self.validate_and_parse_classification(content)
    }

    /// Check completeness of an already-classified issue.
    pub async fn check_completeness(&self, issue: &Issue) -> Result<ClassificationResult> {
        let metadata = Self::issue_to_metadata(issue);
        let prompt = ClassificationPrompt::for_completeness_check(&metadata);

        let request = self.build_request(&prompt);
        let response = self.llm.chat(request).await?;

        let content = &response.choices[0].message.content;
        self.validate_and_parse_classification(content)
    }

    /// Build a chat request from a prompt.
    fn build_request(&self, prompt: &ClassificationPrompt) -> ChatRequest {
        ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage::system(&prompt.system_prompt),
                ChatMessage::user(&prompt.user_prompt),
            ],
            temperature: Some(0.3),
            max_tokens: Some(2048),
            response_format: Some(rogers_llm::ResponseFormat {
                format_type: "json_object".to_string(),
                schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "issue_type": {
                            "type": "string",
                            "enum": ["bug", "feature", "question", "docs", "chore", "unknown"]
                        },
                        "completeness": {
                            "type": "string",
                            "enum": ["complete", "incomplete"]
                        },
                        "missing_fields": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "severity": {
                            "type": "string",
                            "enum": ["critical", "high", "medium", "low", "none"]
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["critical", "high", "medium", "low"]
                        },
                        "response_draft": {"type": "string"},
                        "confidence": {"type": "number", "minimum": 0, "maximum": 1}
                    },
                    "required": ["issue_type", "completeness"]
                })),
            }),
        }
    }

    /// Convert a GitHub issue to metadata for classification.
    fn issue_to_metadata(issue: &Issue) -> IssueMetadata {
        let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
        let prior_comments: Vec<String> = vec![]; // Comments are fetched separately if needed

        IssueMetadata {
            number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            author: issue.user.login.clone(),
            author_type: issue.user.user_type.clone(),
            labels,
            prior_comments,
        }
    }

    /// Validate and parse LLM classification output.
    fn validate_and_parse_classification(&self, content: &str) -> Result<ClassificationResult> {
        // First, try to extract JSON from markdown code blocks if present
        let json_str = Self::extract_json(content);

        // Validate the output
        match self.validator.validate_classification(&json_str) {
            Ok(output) => Ok(ClassificationResult {
                output,
                raw_response: content.to_string(),
            }),
            Err(result) => {
                let errors = result
                    .errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                Err(RogersError::Config(format!(
                    "LLM output validation failed: {}",
                    errors
                )))
            }
        }
    }

    /// Extract JSON from content that might be wrapped in markdown code blocks.
    fn extract_json(content: &str) -> String {
        let trimmed = content.trim();

        // Check for markdown code block
        if trimmed.starts_with("```json") {
            // Find the end of the code block
            if let Some(end) = trimmed.find("```\n").or(Some(trimmed.len())) {
                let json_content = &trimmed[7..end];
                return json_content.trim().to_string();
            }
        } else if trimmed.starts_with("```") {
            if let Some(end) = trimmed.find("```\n").or(Some(trimmed.len())) {
                let json_content = &trimmed[3..end];
                return json_content.trim().to_string();
            }
        }

        trimmed.to_string()
    }

    /// Validate a response draft against warmth principles.
    pub fn validate_response_draft(&self, draft: &str) -> ValidationResult {
        self.validator.validate_response_draft(draft)
    }
}

/// Classification result with raw response for debugging.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Validated classification output.
    pub output: ClassificationOutput,
    /// Raw LLM response for debugging.
    pub raw_response: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_markdown() {
        let content = r#"```json
{"issue_type": "bug", "completeness": "complete"}
```"#;

        let json = Classifier::extract_json(content);
        assert!(json.contains("bug"));
        assert!(!json.starts_with("```"));
    }

    #[test]
    fn test_extract_json_from_plain() {
        let content = r#"{"issue_type": "feature", "completeness": "incomplete"}"#;

        let json = Classifier::extract_json(content);
        assert!(json.contains("feature"));
    }

    #[test]
    fn test_extract_json_with_extra_text() {
        let content = "Here's the analysis:\n```json\n{\"issue_type\": \"question\"}\n```\nDoes this look right?";

        let json = Classifier::extract_json(content);
        assert!(json.contains("question"));
    }

    #[test]
    fn test_issue_to_metadata() {
        let issue = Issue {
            number: 42,
            title: "Test Title".to_string(),
            body: Some("Test body content".to_string()),
            state: "open".to_string(),
            user: rogers_github::models::User {
                login: "testuser".to_string(),
                id: 123,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: Some("User".to_string()),
            },
            labels: vec![rogers_github::models::Label {
                id: 1,
                name: "bug".to_string(),
                description: None,
                color: None,
                node_id: None,
            }],
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

        let metadata = Classifier::issue_to_metadata(&issue);
        assert_eq!(metadata.number, 42);
        assert_eq!(metadata.title, "Test Title");
        assert_eq!(metadata.author, "testuser");
        assert_eq!(metadata.labels, vec!["bug"]);
    }

    #[test]
    fn test_issue_to_metadata_bot() {
        let issue = Issue {
            number: 43,
            title: "Bot Issue".to_string(),
            body: None,
            state: "open".to_string(),
            user: rogers_github::models::User {
                login: "snyk-bot".to_string(),
                id: 456,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: Some("Bot".to_string()),
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

        let metadata = Classifier::issue_to_metadata(&issue);
        assert_eq!(metadata.author_type, Some("Bot".to_string()));
    }
}
