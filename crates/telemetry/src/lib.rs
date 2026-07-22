//! LayerMind telemetry pipeline.
//!
//! The central nervous system of LayerMind. Consumes canonical Envelopes from
//! printer instances, buffers them, enriches with metadata, and routes them
//! to configured sinks (database, file, future AI engine).
//!
//! Design principles:
//! - Never drop events. Buffer and retry.
//! - Timestamp at ingress, not egress.
//! - Events are immutable once emitted.

use layermind_config::TelemetryConfig;
use layermind_shared::error::Result;
use layermind_shared::event::Envelope;
use tokio::sync::mpsc;

mod buffer;
mod pipeline;
mod sink;

/// The telemetry engine. Accepts envelopes and manages the pipeline.
pub struct TelemetryEngine {
    config: TelemetryConfig,
    tx: mpsc::Sender<Envelope>,
}

impl TelemetryEngine {
    pub fn new(config: TelemetryConfig) -> (Self, mpsc::Receiver<Envelope>) {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        (Self { config, tx }, rx)
    }

    pub fn config(&self) -> &TelemetryConfig {
        &self.config
    }

    pub fn sender(&self) -> mpsc::Sender<Envelope> {
        self.tx.clone()
    }

    pub async fn run(self, rx: mpsc::Receiver<Envelope>) -> Result<()> {
        tracing::info!("telemetry engine starting");
        pipeline::run(rx, &self.config).await
    }
}
