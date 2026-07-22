//! Moonraker WebSocket client.
//!
//! Responsibilities:
//! - Connect to Moonraker's websocket endpoint.
//! - Authenticate using API key when configured.
//! - Subscribe to printer object updates.
//! - Handle reconnects with exponential backoff.
//! - Normalize incoming JSON-RPC messages.
//! - Publish raw events to the internal channel.
//!
//! This crate contains no business logic. It is a pure protocol adapter.

use layermind_config::MoonrakerConfig;
use layermind_shared::error::Result;
use tokio::sync::broadcast;

pub mod client;
pub mod protocol;

/// A connected Moonraker client publishing raw events.
pub struct MoonrakerClient {
    config: MoonrakerConfig,
    tx: broadcast::Sender<protocol::RawMessage>,
}

impl MoonrakerClient {
    pub fn new(config: MoonrakerConfig) -> (Self, broadcast::Receiver<protocol::RawMessage>) {
        let (tx, rx) = broadcast::channel(256);
        (Self { config, tx }, rx)
    }

    pub async fn connect(&self) -> Result<()> {
        let url = &self.config.url;
        tracing::info!(url, "connecting to Moonraker");
        // TODO: WebSocket connect, authenticate, subscribe
        Ok(())
    }
}
