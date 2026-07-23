//! Google Gemini provider — native generateContent API.
//!
//! Uses the Gemini API: `POST /v1beta/models/{model}:generateContent`
//! with API key passed as a query parameter.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use layermind_reasoning::provider::{AiError, AiProvider, AiRequest, AiResponse, TokenUsage};

/// Native Gemini provider using the generateContent API.
#[derive(Debug)]
pub struct GeminiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    fn generate_url(&self) -> String {
        format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        )
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AiError> {
        let body = GeminiRequest {
            system_instruction: Some(GeminiContent {
                parts: vec![GeminiPart {
                    text: request.system_prompt,
                }],
            }),
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: request.user_prompt,
                }],
            }],
            generation_config: Some(GenerationConfig {
                max_output_tokens: request.max_tokens,
                temperature: request.temperature as f64,
            }),
        };

        let resp = self
            .client
            .post(&self.generate_url())
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

        let gemini_resp: GeminiResponse = resp
            .json()
            .await
            .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

        let content = gemini_resp
            .candidates
            .into_iter()
            .flat_map(|c| c.content.parts)
            .map(|p| p.text)
            .collect::<Vec<_>>()
            .join("\n");

        let usage = gemini_resp.usage_metadata.map_or(
            TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            |u| TokenUsage {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
            },
        );

        Ok(AiResponse {
            content,
            usage,
            model: gemini_resp
                .model_version
                .unwrap_or_else(|| self.model.clone()),
        })
    }

    fn name(&self) -> &str {
        "gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// ── Gemini API Types ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    max_output_tokens: u32,
    temperature: f64,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsage>,
    model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: GeminiContentResponse,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Debug, Deserialize)]
struct GeminiPartResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiUsage {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
}
