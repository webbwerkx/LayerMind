//! LayerMind AI engine.
//!
//! Event-driven intelligence layer. Subscribes to the telemetry pipeline,
//! detects patterns and anomalies, and generates recommendations.
//!
//! Architecture:
//!   Telemetry → Event Detection → Interesting Event? → AI Analysis
//!     → Recommendation → User Feedback → Learning
//!
//! NOT a chatbot. The AI engine is a background service that observes
//! telemetry streams and produces structured recommendations.

use layermind_shared::error::Result;
use tokio::sync::broadcast;

mod detector;
mod recommender;

/// The AI engine — subscribes to envelopes and runs detection/recommendation.
pub struct AiEngine {
    rx: broadcast::Receiver<layermind_shared::event::Envelope>,
}

impl AiEngine {
    pub fn new(rx: broadcast::Receiver<layermind_shared::event::Envelope>) -> Self {
        Self { rx }
    }

    pub async fn run(mut self) -> Result<()> {
        tracing::info!("AI engine starting");
        loop {
            match self.rx.recv().await {
                Ok(envelope) => {
                    // TODO: Feed envelope into detection pipeline
                    tracing::debug!(event_id = %envelope.event_id, "AI engine received event");
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "AI engine lagging behind telemetry");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
        Ok(())
    }
}
