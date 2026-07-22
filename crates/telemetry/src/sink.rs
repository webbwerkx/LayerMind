//! Telemetry sinks — destinations for processed events.

use layermind_shared::error::Result;
use layermind_shared::event::Envelope;

/// A sink receives batched events and persists them.
pub trait Sink: Send + Sync {
    fn write_batch(&self, events: &[Envelope]) -> Result<()>;
}

/// In-memory sink for testing and development.
pub struct MemorySink {
    events: std::sync::Mutex<Vec<Envelope>>,
}

impl MemorySink {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Sink for MemorySink {
    fn write_batch(&self, events: &[Envelope]) -> Result<()> {
        self.events.lock().unwrap().extend_from_slice(events);
        Ok(())
    }
}
