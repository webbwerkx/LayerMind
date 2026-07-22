//! Telemetry pipeline: buffer → enrich → route → sink.

use layermind_config::TelemetryConfig;
use layermind_shared::error::Result;
use layermind_shared::event::Envelope;
use tokio::sync::mpsc;

pub async fn run(mut rx: mpsc::Receiver<Envelope>, config: &TelemetryConfig) -> Result<()> {
    use tokio::time::{Duration, interval};

    let mut buffer = Vec::with_capacity(config.buffer_size);
    let mut flush_timer = interval(Duration::from_secs_f64(config.flush_interval_secs));

    loop {
        tokio::select! {
            Some(envelope) = rx.recv() => {
                tracing::debug!(
                    event_id = %envelope.event_id,
                    printer_id = %envelope.printer_id,
                    "telemetry event received"
                );
                buffer.push(envelope);

                if buffer.len() >= config.buffer_size {
                    flush_batch(&buffer).await;
                    buffer.clear();
                }
            }
            _ = flush_timer.tick() => {
                if !buffer.is_empty() {
                    flush_batch(&buffer).await;
                    buffer.clear();
                }
            }
            else => break,
        }
    }

    // Final flush on shutdown
    if !buffer.is_empty() {
        flush_batch(&buffer).await;
    }

    tracing::info!("telemetry pipeline shut down");
    Ok(())
}

async fn flush_batch(batch: &[Envelope]) {
    tracing::debug!(count = batch.len(), "flushing telemetry batch");
    // TODO: Write to database, file, or AI pipeline
}
