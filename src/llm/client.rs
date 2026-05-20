//! LLM client for OpenAI-compatible API.
//!
//! Provides a client for LLM inference with structured output support.
//! All reasoning, classification, and decision-making flows through this client.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::LlmConfig;
use crate::error::{Result, RogersError};

/// LLM client for OpenAI-compatible API.
#[derive(Debug, Clone)]
pub struct LlmClient {
    /// HTTP client for making requests.
    client: Client,
    /// Base URL for the API.
    base_url: String,
    /// API key for authentication.
    api_key: String,
    /// Model name.
    model: String,
}

impl LlmClient {
    /// Create a new LLM client from configuration.
    pub fn new(config: &LlmConfig) -> Self {
        let client = Client::builder()
            .user_agent("Rodgers/0.1.0 (GitHub-native community relations agent)")
            .build()
            .expect("valid reqwest client");

        Self {
            client,
            base_url: config
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
        }
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Build the chat completions URL.
    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    /// Send a chat completion request and parse the response.
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let request = self
            .client
            .post(&self.chat_url())
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key),
            )
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&request);

        let response = request.send().await?;

        if response.status().is_success() {
            response
                .json::<ChatResponse>()
                .await
                .map_err(RogersError::from)
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message: error_body,
            })
        }
    }

    /// Send a chat completion request with structured output.
    pub async fn chat_structured<T: for<'de> Deserialize<'de>>(
        &self,
        request: ChatRequest,
    ) -> Result<T> {
        let request = self
            .client
            .post(&self.chat_url())
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key),
            )
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&request);

        let response = request.send().await?;

        if response.status().is_success() {
            response
                .json::<ChatResponse>()
                .await
                .map_err(RogersError::from)
                .and_then(|r| {
                    serde_json::from_str::<T>(&r.choices[0].message.content)
                        .map_err(RogersError::from)
                })
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message: error_body,
            })
        }
    }
}

// ─── Request/Response types ─────────────────────────────────────────────────

/// Chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

/// Chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            name: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            name: None,
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
        }
    }
}

/// Response format for structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

/// Response choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ResponseMessage,
    pub finish_reason: String,
}

/// Response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    pub content: String,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("You are a helpful assistant.");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are a helpful assistant.");
    }

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("Hello, world!");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, world!");
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("I am an assistant.");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "I am an assistant.");
    }

    #[test]
    fn test_response_format_json() {
        let format = ResponseFormat {
            format_type: "json_object".to_string(),
            schema: None,
        };
        let json = serde_json::to_string(&format).unwrap();
        assert!(json.contains("json_object"));
    }
}
