//! Storage-agnostic sink trait for the telemetry pipeline.
//!
//! Defines the contract that all storage backends must implement.
//! This lives in `shared` so that both `telemetry` (producer) and
//! `database` (consumer) can depend on the contract without coupling
//! to each other.

use crate::error::Result;
use crate::event::Envelope;
use async_trait::async_trait;

/// A sink receives batched events and persists them.
///
/// Implementations include:
/// - `DatabaseSink` (PostgreSQL, TimescaleDB)
/// - `MemorySink` (testing)
/// - Future: SQLite, file export, cloud storage
#[async_trait]
pub trait Sink: Send + Sync {
    /// Persist a batch of events. Called by the telemetry pipeline
    /// on buffer-full or timed flush. Must be idempotent where possible.
    async fn write_batch(&self, events: &[Envelope]) -> Result<()>;

    /// Flush any buffered state. Called before shutdown.
    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}
