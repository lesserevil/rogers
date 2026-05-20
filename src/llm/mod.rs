//! LLM client module.
//!
//! Provides an OpenAI-compatible API client for LLM inference.
//! Handles authentication, structured output, and error handling.

pub mod client;
pub mod prompts;
pub mod validator;

pub use client::{ChatMessage, ChatRequest, LlmClient, ResponseFormat};
pub use prompts::{ClassificationPrompt, IssueMetadata};
pub use validator::{ClassificationOutput, OutputValidator, ValidationError, ValidationResult};
