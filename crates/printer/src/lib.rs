//! Printer abstraction layer.
//!
//! Normalizes raw integration events into canonical `layermind_shared::event::Event`
//! types. Maintains a high-level printer state machine. This crate consumes
//! raw protocol messages from integration crates (moonraker, etc.) and produces
//! typed, timestamped envelopes for the telemetry pipeline.

use layermind_moonraker::protocol::RpcMessage;
use layermind_shared::event::{Envelope, Event};
use layermind_shared::printer::PrinterState;
use tokio::sync::broadcast;
use uuid::Uuid;

pub mod moonraker_normalizer;

/// A logical printer instance that normalizes raw events into canonical form.
pub struct Printer {
    id: String,
    name: String,
    state: PrinterState,
    normalizer_state: moonraker_normalizer::NormalizerState,
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
                normalizer_state: moonraker_normalizer::NormalizerState::new(),
                tx,
            },
            rx,
        )
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn state(&self) -> PrinterState {
        self.state
    }

    /// Process a raw Moonraker RPC message and emit any resulting events.
    pub fn process_raw_message(&mut self, msg: &RpcMessage) {
        let events = moonraker_normalizer::normalize(msg, &mut self.normalizer_state);

        // Track state transitions.
        for event in &events {
            if let Event::StateChanged { state } = event {
                self.state = *state;
            }
        }

        for event in events {
            self.emit(event);
        }
    }

    /// Run a loop processing raw messages from a Moonraker broadcast receiver.
    /// Blocks until the sender is dropped.
    pub async fn run_from_moonraker(mut self, mut rx: broadcast::Receiver<RpcMessage>) {
        tracing::info!(printer_id = %self.id, "printer starting event loop");

        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if msg.is_status_update() {
                        self.process_raw_message(&msg);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        printer_id = %self.id,
                        skipped = n,
                        "printer lagging behind Moonraker messages"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!(printer_id = %self.id, "Moonraker channel closed, printer stopping");
                    break;
                }
            }
        }

        self.emit(Event::Disconnected {
            reason: "Moonraker connection ended".into(),
        });
    }

    /// Clone the sender for additional subscribers (telemetry, AI engine).
    pub fn sender(&self) -> broadcast::Sender<Envelope> {
        self.tx.clone()
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
