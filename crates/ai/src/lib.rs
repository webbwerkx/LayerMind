//! LayerMind AI provider implementations.
//!
//! This crate contains provider implementations for every supported
//! AI backend. The `AiProvider` trait is defined in `layermind-reasoning`;
//! this crate provides the real implementations.
//!
//! Supported providers:
//!   - `OpenAiCompatible` — OpenAI, OpenRouter, Ollama (v0.1.24+),
//!     LM Studio, vLLM, LocalAI, llama.cpp, and any `/v1/chat/completions`
//!     endpoint.
//!   - `AnthropicProvider` — Anthropic Messages API.
//!   - `GeminiProvider` — Google Gemini generateContent API.
//!
//! Use `create_provider()` to instantiate from `ProviderConfig`.

use std::sync::Arc;

use layermind_config::ProviderConfig;
use layermind_reasoning::provider::AiProvider;

mod providers;
mod retry;
mod streaming;

pub use providers::{AnthropicProvider, GeminiProvider, OpenAiCompatibleProvider};
pub use retry::RetryingProvider;
pub use streaming::StreamingAiProvider;

/// Errors specific to provider instantiation and configuration.
#[derive(Debug, thiserror::Error)]
pub enum ProviderCreationError {
    #[error("unknown provider '{0}'. Known: openai, openrouter, anthropic, gemini, ollama, custom")]
    UnknownProvider(String),
    #[error("provider '{0}' requires an API key")]
    MissingApiKey(String),
    #[error("invalid provider configuration: {0}")]
    InvalidConfig(String),
}

/// Create an AI provider from configuration.
///
/// Detects the appropriate backend from `config.provider` and returns
/// a fully initialized `Arc<dyn AiProvider>` ready for use with
/// `PrintDoctor`.
///
/// API key resolution order (per provider):
///   1. `config.api_key` (explicit)
///   2. Provider-specific environment variable
///
/// # Examples
///
/// ```ignore
/// let config = ProviderConfig {
///     provider: "openrouter".into(),
///     model: "deepseek/deepseek-chat".into(),
///     api_key: Some("sk-...".into()),
///     ..Default::default()
/// };
/// let provider = create_provider(&config)?;
/// let doctor = PrintDoctor::new(provider);
/// ```
pub fn create_provider(
    config: &ProviderConfig,
) -> Result<Arc<dyn AiProvider>, ProviderCreationError> {
    match config.provider.as_str() {
        "openai" => {
            let key = resolve_key(config, "OPENAI_API_KEY")?;
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://api.openai.com".into());
            Ok(Arc::new(providers::OpenAiCompatibleProvider::new(
                &endpoint,
                &key,
                &config.model,
            )))
        }
        "openrouter" => {
            let key = resolve_key(config, "OPENROUTER_API_KEY")?;
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://openrouter.ai/api".into());
            Ok(Arc::new(providers::OpenAiCompatibleProvider::new(
                &endpoint,
                &key,
                &config.model,
            )))
        }
        "ollama" => {
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".into());
            // Ollama doesn't require auth by default.
            Ok(Arc::new(providers::OpenAiCompatibleProvider::new(
                &endpoint,
                "",
                &config.model,
            )))
        }
        "custom" => {
            let endpoint = config.endpoint.as_deref().ok_or_else(|| {
                ProviderCreationError::InvalidConfig(
                    "custom provider requires an endpoint URL".into(),
                )
            })?;
            let key = config.api_key.clone().unwrap_or_default();
            Ok(Arc::new(providers::OpenAiCompatibleProvider::new(
                endpoint,
                &key,
                &config.model,
            )))
        }
        "anthropic" => {
            let key = resolve_key(config, "ANTHROPIC_API_KEY")?;
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://api.anthropic.com".into());
            Ok(Arc::new(providers::AnthropicProvider::new(
                &endpoint,
                &key,
                &config.model,
            )))
        }
        "gemini" => {
            let key = resolve_key(config, "GEMINI_API_KEY")?;
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com".into());
            Ok(Arc::new(providers::GeminiProvider::new(
                &endpoint,
                &key,
                &config.model,
            )))
        }
        other => Err(ProviderCreationError::UnknownProvider(other.into())),
    }
}

fn resolve_key(config: &ProviderConfig, env_var: &str) -> Result<String, ProviderCreationError> {
    if let Some(ref key) = config.api_key {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }
    std::env::var(env_var).map_err(|_| ProviderCreationError::MissingApiKey(env_var.into()))
}
