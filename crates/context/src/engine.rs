//! Context Engine — subscribes to Knowledge stream and updates the
//! shared ContextStore.
//!
//! The engine owns only the event receiver and ingestion loop. All
//! cached data lives in the `ContextStore`, which is shared via `Arc`
//! with consumers (PrintDoctor, CLI, REST API, web UI).

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing;

use crate::store::ContextStore;

/// Ingestion engine. Receives Knowledge records and updates the store.
pub struct ContextEngine {
    rx: broadcast::Receiver<layermind_shared::knowledge::Knowledge>,
    store: Arc<ContextStore>,
}

impl ContextEngine {
    pub fn new(
        rx: broadcast::Receiver<layermind_shared::knowledge::Knowledge>,
        store: Arc<ContextStore>,
    ) -> Self {
        Self { rx, store }
    }

    /// Run the ingestion loop until the broadcast sender is dropped.
    pub async fn run(mut self) {
        tracing::info!("context engine starting");

        loop {
            match self.rx.recv().await {
                Ok(knowledge) => {
                    self.store.update(knowledge);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "context engine lagging");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        tracing::info!("context engine stopped");
    }
}
