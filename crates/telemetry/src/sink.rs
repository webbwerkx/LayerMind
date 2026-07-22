//! Telemetry sinks — destinations for processed events.
//!
//! The `Sink` trait is defined in `layermind_shared` and re-exported here.
//! This module provides built-in sink implementations.
//!
//! For the production database sink, see `layermind_database::DatabaseSink`.

use std::sync::Mutex;

use async_trait::async_trait;
use layermind_shared::error::Result;
use layermind_shared::event::Envelope;

pub use layermind_shared::sink::Sink;

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
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl Sink for MemorySink {
    async fn write_batch(&self, events: &[Envelope]) -> Result<()> {
        self.events.lock().unwrap().extend_from_slice(events);
        Ok(())
    }
}
