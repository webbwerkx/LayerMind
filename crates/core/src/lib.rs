//! LayerMind core — service orchestration and lifecycle management.
//!
//! The core crate ties all LayerMind services together. It is responsible
//! for:
//! - Loading configuration.
//! - Initializing logging.
//! - Starting and supervising all sub-services.
//! - Graceful shutdown.
//!
//! This is the entry point for the LayerMind daemon.

use layermind_config::Config;
use layermind_shared::error::Result;
use tokio::signal;
use tracing::Instrument;

mod service;

/// Run the LayerMind daemon.
pub async fn run() -> Result<()> {
    let config = Config::load()?;
    layermind_logging::init(&config.logging);

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "LayerMind starting");

    let manager = ServiceManager::new(config);
    manager.run().await
}

/// Manages the lifecycle of all LayerMind services.
pub struct ServiceManager {
    config: Config,
}

impl ServiceManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("initializing services");

        // TODO: Wire up actual service graph:
        //   1. Connect database (optional, graceful degradation)
        //   2. Start telemetry engine
        //   3. Create printer instance
        //   4. Connect Moonraker client
        //   5. Normalize via printer
        //   6. Route to telemetry
        //   7. Start AI engine (subscribes to telemetry)
        //   8. Wait for shutdown signal

        tracing::info!("all services initialized, waiting for shutdown signal");
        shutdown_signal().await;

        self.graceful_shutdown().await
    }

    async fn graceful_shutdown(&self) -> Result<()> {
        tracing::info!("shutting down LayerMind");
        Ok(())
    }
}

/// Wait for SIGINT or SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("received SIGINT, shutting down");
        }
        () = terminate => {
            tracing::info!("received SIGTERM, shutting down");
        }
    }
}
