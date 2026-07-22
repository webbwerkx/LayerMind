//! Moonraker WebSocket client — pure protocol adapter.
//!
//! Responsibilities:
//! - Connect to Moonraker's WebSocket endpoint.
//! - Authenticate using API key when configured.
//! - Subscribe to printer object updates.
//! - Handle reconnects with exponential backoff.
//! - Publish all incoming JSON-RPC messages to a broadcast channel.
//!
//! This crate contains no business logic. It is a pure protocol adapter.
//! Downstream crates (printer) consume raw messages and normalize them
//! into canonical LayerMind event types.

use layermind_config::MoonrakerConfig;
use layermind_shared::error::Result;
use tokio::sync::broadcast;
use tokio::sync::watch;

pub mod client;
pub mod protocol;

/// Default broadcast channel capacity for raw Moonraker messages.
const CHANNEL_CAPACITY: usize = 1024;

/// A connected Moonraker client that publishes raw JSON-RPC messages.
///
/// Create with [`MoonrakerClient::new`], then call [`run`](MoonrakerClient::run)
/// to start the connection loop. Messages flow through the broadcast receiver
/// returned from `new()`.
pub struct MoonrakerClient {
    config: MoonrakerConfig,
    tx: broadcast::Sender<protocol::RpcMessage>,
}

impl MoonrakerClient {
    /// Create a new client and a receiver for incoming messages.
    ///
    /// The client does not connect until [`run`](MoonrakerClient::run) is called.
    pub fn new(config: MoonrakerConfig) -> (Self, broadcast::Receiver<protocol::RpcMessage>) {
        let (tx, rx) = broadcast::channel(CHANNEL_CAPACITY);
        (Self { config, tx }, rx)
    }

    /// Run the client until the shutdown signal fires.
    ///
    /// Manages the full lifecycle: connect → subscribe → read messages
    /// → reconnect on failure. Blocks until shutdown or unrecoverable error.
    pub async fn run(&self, shutdown: watch::Receiver<()>) -> Result<()> {
        client::run_connection_loop(&self.config, self.tx.clone(), shutdown).await
    }

    /// Return a clone of the sender for additional subscribers.
    pub fn sender(&self) -> broadcast::Sender<protocol::RpcMessage> {
        self.tx.clone()
    }
}
