//! LLM (Language Model) interface for Rodgers.
//!
//! Provides a thin wrapper around OpenAI-compatible API endpoints for
//! inference, with structured output validation.

use crate::error::{Result, RogersError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod prompts;

/// Configuration for an LLM client.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Base URL for the OpenAI-compatible API.
    pub base_url: String,
    /// Model name (e.g., "gpt-4o-mini").
    pub model: String,
    /// API key for authentication.
    pub api_key: String,
}

/// Message in an LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// Role: "system", "user", or "assistant".
    pub role: String,
    /// Message content.
    pub content: String,
}

/// Request/response types for the LLM API.
#[derive(Debug, Serialize)]
struct LlmRequest<'a> {
    model: &'a str,
    messages: &'a [LlmMessage],
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

/// LLM client for Rodgers.
#[derive(Debug, Clone)]
pub struct LlmClient {
    client: reqwest::Client,
    config: LlmConfig,
}

impl LlmClient {
    /// Create a new LLM client from configuration.
    pub fn new(config: LlmConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    /// Send a prompt to the LLM and get a text response.
    pub async fn complete(&self, messages: &[LlmMessage]) -> Result<String> {
        let request = LlmRequest {
            model: &self.config.model,
            messages,
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RogersError::GitHubStatus {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let response_json: Value = response.json().await?;

        let content = response_json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RogersError::Config("Invalid LLM response format".to_string()))?
            .to_string();

        Ok(content)
    }

    /// Send a prompt to the LLM and parse JSON response.
    pub async fn complete_json<T: serde::de::DeserializeOwned>(
        &self,
        messages: &[LlmMessage],
    ) -> Result<T> {
        let text = self.complete(messages).await?;
        let parsed: T = serde_json::from_str(&text)
            .map_err(|e| RogersError::Config(format!("Failed to parse JSON: {}", e)))?;
        Ok(parsed)
    }
}

/// Builder for constructing LLM conversation messages.
#[derive(Debug, Default)]
pub struct LlmConversation {
    messages: Vec<LlmMessage>,
}

impl LlmConversation {
    /// Create a new conversation with a system prompt.
    pub fn with_system(system_prompt: &str) -> Self {
        Self {
            messages: vec![LlmMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            }],
        }
    }

    /// Add a user message to the conversation.
    pub fn add_user(&mut self, content: &str) -> &mut Self {
        self.messages.push(LlmMessage {
            role: "user".to_string(),
            content: content.to_string(),
        });
        self
    }

    /// Add an assistant message to the conversation.
    pub fn add_assistant(&mut self, content: &str) -> &mut Self {
        self.messages.push(LlmMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
        });
        self
    }

    /// Get the messages for sending to the LLM.
    pub fn messages(&self) -> &[LlmMessage] {
        &self.messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_builder() {
        let mut conv = LlmConversation::with_system("You are helpful.");
        conv.add_user("Hello, how are you?");

        assert_eq!(conv.messages().len(), 2);
        assert_eq!(conv.messages()[0].role, "system");
        assert_eq!(conv.messages()[0].content, "You are helpful.");
        assert_eq!(conv.messages()[1].role, "user");
        assert_eq!(conv.messages()[1].content, "Hello, how are you?");
    }

    #[test]
    fn test_llm_config() {
        let config = LlmConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: "test-key".to_string(),
        };

        assert_eq!(config.model, "gpt-4o-mini");
    }
}
