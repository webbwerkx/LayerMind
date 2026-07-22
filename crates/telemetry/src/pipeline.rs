//! Telemetry pipeline: buffer → enrich → route → sink.

use layermind_config::TelemetryConfig;
use layermind_shared::error::Result;
use layermind_shared::event::Envelope;
use layermind_shared::sink::Sink;
use tokio::sync::mpsc;

pub async fn run(
    mut rx: mpsc::Receiver<Envelope>,
    config: &TelemetryConfig,
    sink: &dyn Sink,
) -> Result<()> {
    use tokio::time::{Duration, interval};

    let mut buffer = Vec::with_capacity(config.buffer_size);
    let mut flush_timer = interval(Duration::from_secs_f64(config.flush_interval_secs));

    loop {
        tokio::select! {
            Some(envelope) = rx.recv() => {
                tracing::trace!(
                    event_id = %envelope.event_id,
                    printer_id = %envelope.printer_id,
                    "telemetry event received"
                );
                buffer.push(envelope);

                if buffer.len() >= config.buffer_size {
                    flush_batch(&buffer, sink).await;
                    buffer.clear();
                }
            }
            _ = flush_timer.tick() => {
                if !buffer.is_empty() {
                    flush_batch(&buffer, sink).await;
                    buffer.clear();
                }
            }
            else => break,
        }
    }

    if !buffer.is_empty() {
        flush_batch(&buffer, sink).await;
    }

    sink.flush().await?;
    tracing::info!("telemetry pipeline shut down");
    Ok(())
}

async fn flush_batch(batch: &[Envelope], sink: &dyn Sink) {
    tracing::debug!(count = batch.len(), "flushing telemetry batch");
    if let Err(e) = sink.write_batch(batch).await {
        tracing::error!(error = %e, count = batch.len(), "sink write failed, events lost");
    }
}
