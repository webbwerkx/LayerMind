//! LayerMind structured logging.
//!
//! Initializes the tracing subscriber based on configuration. Supports
//! human-readable and JSON output modes for integration with log aggregators.

use layermind_config::LoggingConfig;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init(config: &LoggingConfig) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    let registry = tracing_subscriber::registry().with(env_filter);

    if config.json_output {
        registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .init();
    }

    tracing::info!("logging initialized at level {}", config.level);
}
