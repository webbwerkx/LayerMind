//! Built-in telemetry sink implementations.
//!
//! The `Sink` trait is defined in `layermind_shared::sink`. This module
//! provides testing/development sinks. For production storage (PostgreSQL)
//! see `layermind_database::DatabaseSink`.

use std::sync::Mutex;

use async_trait::async_trait;
use layermind_shared::error::Result;
use layermind_shared::event::Envelope;
use layermind_shared::sink::Sink;

/// In-memory sink for testing and development.
pub struct MemorySink {
    events: Mutex<Vec<Envelope>>,
}

impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySink {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn events(&self) -> Vec<Envelope> {
        self.events
            .lock()
            .expect("MemorySink mutex poisoned")
            .clone()
    }
}

#[async_trait]
impl Sink for MemorySink {
    async fn write_batch(&self, events: &[Envelope]) -> Result<()> {
        self.events
            .lock()
            .expect("MemorySink mutex poisoned")
            .extend_from_slice(events);
        Ok(())
    }
}
