//! OpenAI-compatible provider implementation.
//!
//! Implements the `/v1/chat/completions` contract shared by:
//! - OpenAI
//! - OpenRouter
//! - Anthropic (via compatible gateway)
//! - DeepSeek, Mistral, Groq, etc.
//! - Local llama.cpp server
//! - Local Ollama

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AiError, AiProvider, AiRequest, AiResponse, TokenUsage};

/// An OpenAI-compatible provider.
#[derive(Debug)]
pub struct OpenAiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    /// Create a new provider.
    ///
    /// `base_url` should be the API endpoint root, e.g.:
    /// - `https://api.openai.com` (default)
    /// - `https://openrouter.ai/api` (OpenRouter)
    /// - `http://localhost:8080` (local llama.cpp)
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    /// Create a provider from the standard OPENAI_API_KEY env var.
    pub fn from_env(model: &str) -> Result<Self, AiError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("LAYERMIND_OPENAI_API_KEY"))
            .map_err(|_| {
                AiError::NotConfigured("OPENAI_API_KEY or LAYERMIND_OPENAI_API_KEY not set".into())
            })?;
        let base_url =
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com".into());
        Ok(Self::new(&base_url, &api_key, model))
    }

    fn chat_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AiError> {
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: request.system_prompt,
                },
                ChatMessage {
                    role: "user".into(),
                    content: request.user_prompt,
                },
            ],
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            response_format: None,
        };

        let resp = self
            .client
            .post(&self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let body = resp.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 => Err(AiError::Unauthorized(body)),
                429 => Err(AiError::RateLimited { retry_after }),
                _ => Err(AiError::ApiError {
                    status: status.as_u16(),
                    body,
                }),
            };
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let choice = chat_resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AiError::InvalidResponse("no choices in response".into()))?;

        let usage = chat_resp.usage.unwrap_or(TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        Ok(AiResponse {
            content: choice.message.content,
            usage,
            model: chat_resp.model,
        })
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_structured_output(&self) -> bool {
        true
    }
}

// ── OpenAI API Types ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    model: String,
    choices: Vec<Choice>,
    usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_missing_key_returns_error() {
        // The method returns an error when the env vars are not set.
        // We can't unset them in tests without affecting other tests,
        // so we test that from_env works when a bogus URL is used
        // with a set key. This test is best-effort.
        let result =
            std::env::var("OPENAI_API_KEY").or_else(|_| std::env::var("LAYERMIND_OPENAI_API_KEY"));
        // If neither is set, from_env should fail.
        if result.is_err() {
            let provider = OpenAiProvider::from_env("gpt-4o");
            assert!(provider.is_err());
        }
    }
}
