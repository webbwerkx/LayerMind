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
            Ok(Arc::new(
                providers::OpenAiCompatibleProvider::new(&endpoint, &key, &config.model)
                    .with_name("openai"),
            ))
        }
        "openrouter" => {
            let key = resolve_key(config, "OPENROUTER_API_KEY")?;
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "https://openrouter.ai/api".into());
            Ok(Arc::new(
                providers::OpenAiCompatibleProvider::new(&endpoint, &key, &config.model)
                    .with_name("openrouter"),
            ))
        }
        "ollama" => {
            let endpoint = config
                .endpoint
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".into());
            Ok(Arc::new(
                providers::OpenAiCompatibleProvider::new(&endpoint, "", &config.model)
                    .with_name("ollama"),
            ))
        }
        "custom" => {
            let endpoint = config.endpoint.as_deref().ok_or_else(|| {
                ProviderCreationError::InvalidConfig(
                    "custom provider requires an endpoint URL".into(),
                )
            })?;
            let key = config.api_key.clone().unwrap_or_default();
            Ok(Arc::new(
                providers::OpenAiCompatibleProvider::new(endpoint, &key, &config.model)
                    .with_name("custom"),
            ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_config::ProviderConfig;

    fn config_for(provider: &str) -> ProviderConfig {
        ProviderConfig {
            provider: provider.into(),
            model: "test-model".into(),
            api_key: Some("test-key".into()),
            ..Default::default()
        }
    }

    #[test]
    fn create_openai_with_key() {
        let cfg = config_for("openai");
        let result = create_provider(&cfg);
        assert!(result.is_ok());
        let p = result.unwrap();
        assert_eq!(p.name(), "openai");
        assert_eq!(p.model(), "test-model");
    }

    #[test]
    fn create_openai_missing_key() {
        let cfg = ProviderConfig {
            provider: "openai".into(),
            model: "m".into(),
            api_key: None,
            ..Default::default()
        };
        let result = create_provider(&cfg);
        match result {
            Ok(_) => {} // env var was set, test passes
            Err(e) => assert!(matches!(e, ProviderCreationError::MissingApiKey(_))),
        }
    }

    #[test]
    fn create_openrouter_returns_correct_name() {
        let cfg = config_for("openrouter");
        let result = create_provider(&cfg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "openrouter");
    }

    #[test]
    fn create_ollama_no_auth() {
        let cfg = ProviderConfig {
            provider: "ollama".into(),
            model: "llama3.3".into(),
            ..Default::default()
        };
        let result = create_provider(&cfg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "ollama");
    }

    #[test]
    fn create_anthropic_with_key() {
        let cfg = config_for("anthropic");
        let result = create_provider(&cfg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "anthropic");
    }

    #[test]
    fn create_gemini_with_key() {
        let cfg = config_for("gemini");
        let result = create_provider(&cfg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "gemini");
    }

    #[test]
    fn create_custom_requires_endpoint() {
        let cfg = ProviderConfig {
            provider: "custom".into(),
            model: "m".into(),
            endpoint: None,
            ..Default::default()
        };
        let result = create_provider(&cfg);
        match result {
            Err(ProviderCreationError::InvalidConfig(_)) => {} // expected
            other => panic!("expected InvalidConfig, got {:?}", other.err()),
        }
    }

    #[test]
    fn create_custom_with_endpoint() {
        let cfg = ProviderConfig {
            provider: "custom".into(),
            model: "m".into(),
            endpoint: Some("http://localhost:9999".into()),
            api_key: Some("k".into()),
        };
        let result = create_provider(&cfg);
        match result {
            Ok(p) => assert_eq!(p.name(), "custom"),
            Err(e) => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn create_unknown_provider_errors() {
        let cfg = ProviderConfig {
            provider: "nonexistent".into(),
            model: "m".into(),
            ..Default::default()
        };
        let result = create_provider(&cfg);
        match result {
            Err(ProviderCreationError::UnknownProvider(_)) => {} // expected
            other => panic!("expected UnknownProvider, got {:?}", other.err()),
        }
    }

    #[test]
    fn all_known_providers_produce_valid_providers() {
        for name in &[
            "openai",
            "openrouter",
            "ollama",
            "anthropic",
            "gemini",
            "custom",
        ] {
            let cfg = ProviderConfig {
                provider: name.to_string(),
                model: "m".into(),
                endpoint: if *name == "custom" {
                    Some("http://localhost:9999".into())
                } else {
                    None
                },
                api_key: Some("k".into()),
            };
            let result = create_provider(&cfg);
            match result {
                Ok(p) => assert_eq!(p.model(), "m"),
                Err(e) => panic!("failed for provider '{}': {}", name, e),
            }
        }
    }
}
