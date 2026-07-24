//! LayerMind configuration management.
//!
//! Loads configuration from standard XDG paths, environment variables,
//! and optional config files. Provides typed, validated access to all
//! LayerMind settings.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub moonraker: MoonrakerConfig,
    pub telemetry: TelemetryConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub provider: ProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoonrakerConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub reconnect_interval_secs: f64,
    pub heartbeat_interval_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub buffer_size: usize,
    pub flush_interval_secs: f64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub run_migrations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub json_output: bool,
}

/// AI provider configuration — selects which backend to use and how to reach it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider identifier: "openai", "openrouter", "anthropic", "gemini",
    /// "ollama", or "custom".
    #[serde(default = "default_provider")]
    pub provider: String,
    /// API endpoint base URL. When set, overrides the provider default.
    /// Examples: `<https://api.openai.com>`, `<http://localhost:11434>`,
    /// `<https://openrouter.ai/api>`.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// Model name: "gpt-4o", "claude-opus-4-20250514", "gemini-2.5-pro",
    /// "llama3.3", "deepseek-chat", etc.
    #[serde(default = "default_model")]
    pub model: String,
    /// API key. Falls back to provider-specific env vars if not set.
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_provider() -> String {
    "custom".into()
}

fn default_model() -> String {
    "gpt-4o".into()
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            endpoint: None,
            model: default_model(),
            api_key: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            moonraker: MoonrakerConfig {
                url: "ws://localhost:7125/websocket".into(),
                api_key: None,
                reconnect_interval_secs: 5.0,
                heartbeat_interval_secs: 30.0,
            },
            telemetry: TelemetryConfig {
                buffer_size: 4096,
                flush_interval_secs: 1.0,
                max_retries: 3,
            },
            database: DatabaseConfig {
                url: "postgres://localhost:5432/layermind".into(),
                max_connections: 5,
                run_migrations: true,
            },
            logging: LoggingConfig {
                level: "info".into(),
                json_output: false,
            },
            provider: ProviderConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Environment variables override defaults:
    /// - `LAYERMIND_MOONRAKER_URL`
    /// - `LAYERMIND_DATABASE_URL`
    /// - `LAYERMIND_LOG_LEVEL`
    ///
    /// Future: load from XDG config file as base, env as overrides.
    pub fn load() -> layermind_shared::error::Result<Self> {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("LAYERMIND_MOONRAKER_URL") {
            config.moonraker.url = url;
        }
        if let Ok(key) = std::env::var("LAYERMIND_MOONRAKER_API_KEY") {
            config.moonraker.api_key = Some(key);
        }
        if let Ok(url) = std::env::var("LAYERMIND_DATABASE_URL") {
            config.database.url = url;
        }
        if let Ok(level) = std::env::var("LAYERMIND_LOG_LEVEL") {
            config.logging.level = level;
        }
        if std::env::var("LAYERMIND_LOG_JSON").is_ok() {
            config.logging.json_output = true;
        }
        if let Ok(provider) = std::env::var("LAYERMIND_PROVIDER") {
            config.provider.provider = provider;
        }
        if let Ok(model) = std::env::var("LAYERMIND_MODEL") {
            config.provider.model = model;
        }
        if let Ok(endpoint) = std::env::var("LAYERMIND_PROVIDER_ENDPOINT") {
            config.provider.endpoint = Some(endpoint);
        }

        Ok(config)
    }

    pub fn config_dir() -> std::path::PathBuf {
        directories::ProjectDirs::from("com", "layermind", "LayerMind")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(".layermind"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_config_defaults() {
        let cfg = ProviderConfig::default();
        assert_eq!(cfg.provider, "custom");
        assert_eq!(cfg.model, "gpt-4o");
        assert!(cfg.endpoint.is_none());
        assert!(cfg.api_key.is_none());
    }

    #[test]
    fn provider_config_serialization_roundtrip() {
        let cfg = ProviderConfig {
            provider: "openrouter".into(),
            endpoint: Some("https://openrouter.ai/api".into()),
            model: "deepseek/deepseek-chat".into(),
            api_key: Some("sk-test".into()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: ProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.provider, "openrouter");
        assert_eq!(deserialized.model, "deepseek/deepseek-chat");
    }

    #[test]
    fn config_backwards_compatible_minimal() {
        // Config without provider field should deserialize with defaults.
        let json = r#"{
            "moonraker": {"url": "ws://localhost:7125/websocket", "api_key": null, "reconnect_interval_secs": 5.0, "heartbeat_interval_secs": 30.0},
            "telemetry": {"buffer_size": 4096, "flush_interval_secs": 1.0, "max_retries": 3},
            "database": {"url": "postgres://localhost:5432/layermind", "max_connections": 5, "run_migrations": true},
            "logging": {"level": "info", "json_output": false}
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.provider.provider, "custom"); // default
    }

    #[test]
    fn config_with_provider_field() {
        let json = r#"{
            "moonraker": {"url": "ws://localhost:7125/websocket", "api_key": null, "reconnect_interval_secs": 5.0, "heartbeat_interval_secs": 30.0},
            "telemetry": {"buffer_size": 4096, "flush_interval_secs": 1.0, "max_retries": 3},
            "database": {"url": "postgres://localhost:5432/layermind", "max_connections": 5, "run_migrations": true},
            "logging": {"level": "info", "json_output": false},
            "provider": {"provider": "ollama", "model": "llama3.3"}
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.provider.provider, "ollama");
        assert_eq!(cfg.provider.model, "llama3.3");
    }
}
