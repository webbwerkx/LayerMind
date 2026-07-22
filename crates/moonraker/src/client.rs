//! Moonraker WebSocket client implementation.

use layermind_shared::error::Result;
use std::time::Duration;

pub struct Client {
    uri: String,
    api_key: Option<String>,
}

impl Client {
    pub fn new(uri: String, api_key: Option<String>) -> Self {
        Self { uri, api_key }
    }

    pub async fn connect_and_run(&self) -> Result<()> {
        tracing::info!(uri = %self.uri, "Moonraker client starting");
        // TODO: Establish WebSocket, auth, subscribe loop
        Ok(())
    }

    fn backoff(attempt: u32) -> Duration {
        Duration::from_secs_f64(2.0_f64.powi(attempt as i32).min(60.0))
    }
}
