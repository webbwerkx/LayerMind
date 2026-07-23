//! OpenAI-compatible provider — the universal `/v1/chat/completions` backend.
//!
//! Covers: OpenAI, OpenRouter, Ollama (v0.1.24+), LM Studio, vLLM,
//! LocalAI, llama.cpp server, and any other endpoint that speaks the
//! OpenAI chat completions contract.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use layermind_reasoning::provider::{AiError, AiProvider, AiRequest, AiResponse, TokenUsage};

/// A provider for any OpenAI-compatible `/v1/chat/completions` endpoint.
///
/// Configurable via base URL, API key, and model name. Works with:
///   - OpenAI: base = `<https://api.openai.com>`
///   - OpenRouter: base = `<https://openrouter.ai/api>`
///   - Ollama: base = `<http://localhost:11434>`
///   - LM Studio: base = `<http://localhost:1234>`
///   - vLLM: base = `<http://localhost:8000>`
///   - LocalAI: base = `<http://localhost:8080>`
#[derive(Debug)]
pub struct OpenAiCompatibleProvider {
    name: String,
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            name: "openai_compatible".into(),
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    /// Set the display name (e.g. "openrouter", "ollama").
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    fn chat_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }
}

#[async_trait]
impl AiProvider for OpenAiCompatibleProvider {
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
        };

        let mut req = self
            .client
            .post(&self.chat_url())
            .header("Content-Type", "application/json")
            .json(&body);

        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let resp = req.send().await.map_err(|e| AiError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let body = resp.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 | 403 => Err(AiError::Unauthorized(body)),
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
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_structured_output(&self) -> bool {
        true
    }
}

// ── API Types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
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
