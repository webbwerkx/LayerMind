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
        if let Ok(url) = std::env::var("LAYERMIND_DATABASE_URL") {
            config.database.url = url;
        }
        if let Ok(level) = std::env::var("LAYERMIND_LOG_LEVEL") {
            config.logging.level = level;
        }
        if std::env::var("LAYERMIND_LOG_JSON").is_ok() {
            config.logging.json_output = true;
        }

        Ok(config)
    }

    pub fn config_dir() -> std::path::PathBuf {
        directories::ProjectDirs::from("com", "layermind", "LayerMind")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(".layermind"))
    }
}
