//! AI provider abstraction.
//!
//! The `AiProvider` trait decouples the reasoning pipeline from any
//! specific model or API. The first implementation is OpenAI-compatible,
//! which covers the entire ecosystem: OpenAI, OpenRouter, Anthropic
//! (via compatible gateway), local llama.cpp, Ollama.

pub mod openai;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Request / Response ──────────────────────────────────────────────

/// A request to an AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// A response from an AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub usage: TokenUsage,
    pub model: String,
}

/// Token usage for one completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── Provider Trait ───────────────────────────────────────────────────

/// Abstraction over AI model providers.
///
/// Implementations should be `Send + Sync` so they can be shared
/// across concurrent diagnostic requests.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Send a completion request and return the raw AI response.
    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AiError>;

    /// Human-readable provider name (e.g. "openai", "openrouter").
    fn name(&self) -> &str;

    /// Model identifier (e.g. "gpt-4o", "deepseek-chat").
    fn model(&self) -> &str;

    /// Whether this provider supports structured JSON output mode.
    fn supports_structured_output(&self) -> bool {
        false
    }
}

// ── Errors ───────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("provider HTTP error: {0}")]
    Http(String),
    #[error("provider returned error status {status}: {body}")]
    ApiError { status: u16, body: String },
    #[error("rate limited — retry after {retry_after:?}")]
    RateLimited { retry_after: Option<String> },
    #[error("authentication failed: {0}")]
    Unauthorized(String),
    #[error("invalid response format: {0}")]
    InvalidResponse(String),
    #[error("provider not configured: {0}")]
    NotConfigured(String),
}

// ── Mock Provider (for testing) ──────────────────────────────────────

/// A mock AI provider that returns pre-configured responses.
/// Used in tests — no network calls, no API key needed.
pub struct MockProvider {
    name: String,
    model: String,
    response: String,
}

impl MockProvider {
    pub fn new(name: &str, model: &str, response: &str) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            response: response.into(),
        }
    }
}

#[async_trait]
impl AiProvider for MockProvider {
    async fn complete(&self, _request: AiRequest) -> Result<AiResponse, AiError> {
        Ok(AiResponse {
            content: self.response.clone(),
            usage: TokenUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            },
            model: self.model.clone(),
        })
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_structured_output(&self) -> bool {
        true
    }
}
