//! LayerMind core — service orchestration and lifecycle management.
//!
//! The core crate ties all LayerMind services together. It is responsible
//! for:
//! - Loading configuration.
//! - Initializing logging.
//! - Starting and supervising all sub-services.
//! - Graceful shutdown.
//!
//! Pipeline: Moonraker → Printer → Telemetry → (future: Database, AI)

use layermind_config::Config;
use tokio::signal;
use tokio::sync::{broadcast, watch};

/// Run the LayerMind daemon.
pub async fn run() -> layermind_shared::error::Result<()> {
    let config = Config::load()?;
    layermind_logging::init(&config.logging);

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "LayerMind starting");

    run_pipeline(&config).await
}

/// Build and run the full observation pipeline.
async fn run_pipeline(config: &Config) -> layermind_shared::error::Result<()> {
    // ── Shutdown signal ──────────────────────────────────────────
    let (shutdown_tx, _shutdown_rx) = watch::channel(());

    // ── Telemetry engine ─────────────────────────────────────────
    let telemetry_config = config.telemetry.clone();
    let (telemetry, telemetry_rx) =
        layermind_telemetry::TelemetryEngine::new(telemetry_config.clone());
    let telemetry_tx = telemetry.sender();

    tracing::info!("telemetry engine ready");

    // ── Printer (normalization layer) ────────────────────────────
    let (printer, printer_rx) = layermind_printer::Printer::new(
        config.moonraker.url.clone(), // printer ID is the Moonraker URL for now
        "Default Printer".into(),
    );
    let _printer_tx = printer.sender();

    tracing::info!(printer_id = %printer.id(), "printer instance created");

    // ── Moonraker client ─────────────────────────────────────────
    let moonraker_config = config.moonraker.clone();
    let (moonraker, moonraker_rx) =
        layermind_moonraker::MoonrakerClient::new(moonraker_config.clone());

    tracing::info!(url = %moonraker_config.url, "Moonraker client ready");

    // ── Wire telemetry subscriber ────────────────────────────────
    // Telemetry receives the printer's canonical envelopes.
    let telemetry_task = {
        tokio::spawn(async move {
            if let Err(e) = telemetry.run(telemetry_rx).await {
                tracing::error!(error = %e, "telemetry engine failed");
            }
        })
    };

    // ── Wire printer → telemetry bridge ──────────────────────────
    // Forward all printer envelopes to telemetry.
    // TODO: In the future, tee to AI engine and database here.
    let mut printer_forward_rx = printer_rx;
    let bridge_task = {
        let telemetry_tx = telemetry_tx.clone();
        tokio::spawn(async move {
            loop {
                match printer_forward_rx.recv().await {
                    Ok(envelope) => {
                        if let Err(e) = telemetry_tx.send(envelope).await {
                            tracing::warn!(error = %e, "telemetry channel full, dropping event");
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "bridge lagging");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            tracing::info!("printer → telemetry bridge stopped");
        })
    };

    // ── Wire printer → Moonraker normalizer ──────────────────────
    let printer_task = {
        tokio::spawn(async move {
            printer.run_from_moonraker(moonraker_rx).await;
        })
    };

    // ── Moonraker connection ─────────────────────────────────────
    let moonraker_shutdown2 = shutdown_tx.subscribe();
    let moonraker_task = {
        tokio::spawn(async move {
            if let Err(e) = moonraker.run(moonraker_shutdown2).await {
                tracing::error!(error = %e, "Moonraker client failed");
            }
        })
    };

    tracing::info!("all services started — pipeline active");
    tracing::info!("Moonraker → Printer → Telemetry → (sink)");

    // ── Wait for shutdown ────────────────────────────────────────
    shutdown_signal().await;

    tracing::info!("shutdown initiated, stopping services");

    // Signal all services to stop.
    let _ = shutdown_tx.send(());

    // Wait for tasks with a timeout.
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        futures_util::future::join_all([moonraker_task, printer_task, bridge_task, telemetry_task]),
    )
    .await;

    tracing::info!("LayerMind shut down cleanly");
    Ok(())
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
