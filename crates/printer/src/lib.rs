//! Printer abstraction layer.
//!
//! Normalizes raw integration events into canonical `layermind_shared::event::Event`
//! types. Maintains a high-level printer state machine. This crate consumes
//! raw protocol messages from integration crates (moonraker, etc.) and produces
//! typed, timestamped envelopes for the telemetry pipeline.

use layermind_shared::event::{Envelope, Event};
use layermind_shared::printer::PrinterState;
use tokio::sync::broadcast;
use uuid::Uuid;

mod moonraker_normalizer;

/// A logical printer instance that normalizes raw events into canonical form.
pub struct Printer {
    id: String,
    name: String,
    state: PrinterState,
    tx: broadcast::Sender<Envelope>,
}

impl Printer {
    pub fn new(id: String, name: String) -> (Self, broadcast::Receiver<Envelope>) {
        let (tx, rx) = broadcast::channel(256);
        (
            Self {
                id,
                name,
                state: PrinterState::Unknown,
                tx,
            },
            rx,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn state(&self) -> PrinterState {
        self.state
    }

    fn emit(&self, payload: Event) {
        let envelope = Envelope {
            event_id: Uuid::now_v7(),
            printer_id: self.id.clone(),
            timestamp: chrono::Utc::now(),
            payload,
        };
        let _ = self.tx.send(envelope);
    }
}
