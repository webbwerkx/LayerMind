//! Anthropic provider — native Messages API implementation.
//!
//! Uses the Anthropic Messages API (`/v1/messages`) with `x-api-key`
//! authentication and `anthropic-version` header.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use layermind_reasoning::provider::{AiError, AiProvider, AiRequest, AiResponse, TokenUsage};

/// Native Anthropic provider using the Messages API.
#[derive(Debug)]
pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AiError> {
        let body = MessagesRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature as f64,
            system: request.system_prompt,
            messages: vec![Message {
                role: "user".into(),
                content: request.user_prompt,
            }],
        };

        let resp = self
            .client
            .post(&self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return match status.as_u16() {
                401 | 403 => Err(AiError::Unauthorized(body)),
                429 => Err(AiError::RateLimited { retry_after: None }),
                _ => Err(AiError::ApiError {
                    status: status.as_u16(),
                    body,
                }),
            };
        }

        let msg_resp: MessagesResponse = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let content = msg_resp
            .content
            .into_iter()
            .filter_map(|block| {
                #[allow(unreachable_patterns)]
                match block {
                    ContentBlock::Text { text } => Some(text),
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let usage = TokenUsage {
            prompt_tokens: msg_resp.usage.input_tokens,
            completion_tokens: msg_resp.usage.output_tokens,
            total_tokens: msg_resp.usage.input_tokens + msg_resp.usage.output_tokens,
        };

        Ok(AiResponse {
            content,
            usage,
            model: msg_resp.model,
        })
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// ── Anthropic API Types ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    temperature: f64,
    system: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    model: String,
    content: Vec<ContentBlock>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}
