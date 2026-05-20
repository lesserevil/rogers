//! LLM integration for Rodgers.
//!
//! This module provides the interface for interacting with the LLM
//! for question routing and code explanation.

pub mod prompts;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for LLM interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Model to use (e.g., "gpt-4", "claude-3-opus")
    pub model: String,
    /// API endpoint for the LLM
    pub api_endpoint: String,
    /// API key (should come from environment)
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    /// Maximum tokens in response
    pub max_tokens: usize,
    /// Temperature for generation (0.0-1.0)
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: None,
            max_tokens: 1024,
            temperature: 0.3, // Low temperature for factual responses
        }
    }
}

/// A message in an LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "system", "user", or "assistant"
    pub role: String,
    /// Message content
    pub content: String,
}

impl Message {
    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create a new system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// A conversation with the LLM.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Messages in the conversation.
    pub messages: Vec<Message>,
}

impl Conversation {
    /// Create a new empty conversation.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Add a system message.
    pub fn with_system(content: impl Into<String>) -> Self {
        let mut conv = Self::new();
        conv.add_system(content);
        conv
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Add a user message.
    pub fn add_user(&mut self, content: impl Into<String>) {
        self.add_message(Message::user(content));
    }

    /// Add a system message.
    pub fn add_system(&mut self, content: impl Into<String>) {
        self.add_message(Message::system(content));
    }

    /// Add an assistant message.
    pub fn add_assistant(&mut self, content: impl Into<String>) {
        self.add_message(Message::assistant(content));
    }

    /// Get a reference to all messages.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Build request body for LLM API.
    pub fn build_request_body(&self, config: &LlmConfig) -> serde_json::Value {
        let messages: Vec<HashMap<String, String>> = self
            .messages
            .iter()
            .map(|m| {
                let mut map = HashMap::new();
                map.insert("role".to_string(), m.role.clone());
                map.insert("content".to_string(), m.content.clone());
                map
            })
            .collect();

        let body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
        });

        body
    }
}

/// Response from the LLM API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// The generated text.
    pub content: String,
    /// Number of tokens used.
    pub usage: TokenUsage,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens in the prompt.
    pub prompt_tokens: usize,
    /// Tokens in the completion.
    pub completion_tokens: usize,
    /// Total tokens.
    pub total_tokens: usize,
}

/// Parses an LLM response from API JSON.
pub fn parse_llm_response(body: &serde_json::Value) -> Option<LlmResponse> {
    let choices = body.get("choices")?;
    let choices_arr = choices.as_array()?;
    if choices_arr.is_empty() {
        return None;
    }
    let first_choice = choices_arr.first()?;
    let message = first_choice.get("message")?;
    let content = message.get("content")?.as_str()?.to_string();

    let usage = body
        .get("usage")
        .map(|u| TokenUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        })
        .unwrap_or(TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

    Some(LlmResponse { content, usage })
}

/// Determines if a question text contains code search trigger keywords.
pub fn contains_code_search_trigger(question: &str) -> bool {
    let question_lower = question.to_lowercase();
    prompts::CODE_SEARCH_TRIGGERS
        .iter()
        .any(|trigger| question_lower.contains(&trigger.to_lowercase()))
}

/// Extracts potential code element names from a question.
/// Looks for PascalCase/camelCase identifiers that might be function or type names.
pub fn extract_code_elements(question: &str) -> Vec<String> {
    let mut elements = Vec::new();

    // Pattern to match CamelCase or PascalCase identifiers
    // This catches things like "QuestionRouter", "codeSearch", "TriageEngine"
    let pattern = regex::Regex::new(r"[A-Z][a-z]+(?:[A-Z][a-z]+)+").ok();

    if let Some(re) = pattern {
        for cap in re.find_iter(question) {
            let element = cap.as_str().to_string();
            // Filter out common English words that happen to be PascalCase
            if !prompts::STOPWORDS.contains(&element.to_lowercase().as_str()) {
                elements.push(element);
            }
        }
    }

    // Also look for file-like patterns (word/word or word/word.rs)
    let file_pattern = regex::Regex::new(r"\w+/\w+(?:\.\w+)?").ok();
    if let Some(re) = file_pattern {
        for cap in re.find_iter(question) {
            elements.push(cap.as_str().to_string());
        }
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_build_request() {
        let mut conv = Conversation::new();
        conv.add_user("Hello");
        conv.add_assistant("Hi there!");

        let config = LlmConfig::default();
        let body = conv.build_request_body(&config);

        assert!(body.get("messages").is_some());
        let messages = body.get("messages").unwrap().as_array().unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_contains_code_search_trigger() {
        assert!(contains_code_search_trigger("How does X work?"));
        assert!(contains_code_search_trigger("What function handles Y?"));
        assert!(contains_code_search_trigger(
            "Walk me through the flow of Z"
        ));
        assert!(contains_code_search_trigger("Show me the internals"));
        assert!(!contains_code_search_trigger("How do I install this?"));
        assert!(!contains_code_search_trigger("What is the weather today?"));
    }

    #[test]
    fn test_extract_code_elements() {
        let elements = extract_code_elements("How does QuestionRouter work?");
        assert!(elements.contains(&"QuestionRouter".to_string()));

        let elements2 = extract_code_elements("Tell me about the TriageEngine internals");
        assert!(elements2.contains(&"TriageEngine".to_string()));
    }

    #[test]
    fn test_parse_llm_response() {
        let json = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "CODE_SEARCH_WARRANTED: TriageEngine, process_issue"
                }
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            }
        });

        let response = parse_llm_response(&json).unwrap();
        assert!(response.content.contains("CODE_SEARCH_WARRANTED"));
        assert_eq!(response.usage.total_tokens, 150);
    }
}
