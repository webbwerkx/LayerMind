//! Moonraker WebSocket connection and reconnection loop.
//!
//! Low-level WebSocket management: connect, authenticate, subscribe,
//! read messages in a loop, reconnect on failure with exponential backoff.

use crate::protocol::{self, PrinterObject, RpcMessage};
use futures_util::{SinkExt, StreamExt};
use layermind_config::MoonrakerConfig;
use layermind_shared::error::Result;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::watch;
use tracing;

/// Connect to Moonraker and run the message loop until shutdown or
/// permanent failure. Publishes all received messages to `tx`.
///
/// Handles reconnection with exponential backoff: 1s → 2s → 4s → ...
/// capped at 60s. On a clean shutdown signal, exits without reconnect.
pub async fn run_connection_loop(
    config: &MoonrakerConfig,
    tx: broadcast::Sender<RpcMessage>,
    mut shutdown: watch::Receiver<()>,
) -> Result<()> {
    let mut attempt: u32 = 0;
    let url = url::Url::parse(&config.url)
        .map_err(|e| layermind_shared::error::Error::Connection(format!("invalid URL: {e}")))?;

    loop {
        attempt += 1;

        if attempt > 1 {
            let delay = backoff_duration(attempt);
            tracing::info!(
                attempt,
                delay_secs = delay.as_secs_f64(),
                "reconnecting to Moonraker"
            );

            tokio::select! {
                () = tokio::time::sleep(delay) => {}
                _ = shutdown.changed() => {
                    tracing::info!("shutdown received during backoff, exiting");
                    return Ok(());
                }
            }
        }

        match connect_and_run(&url, config, &tx, &mut shutdown).await {
            Ok(()) => {
                tracing::info!("Moonraker connection closed cleanly");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    attempt,
                    error = %e,
                    "Moonraker connection error, will retry"
                );
            }
        }
    }
}

/// Establish a WebSocket connection, authenticate if needed, subscribe,
/// then read messages in a loop. Returns on disconnect or shutdown.
async fn connect_and_run(
    url: &url::Url,
    config: &MoonrakerConfig,
    tx: &broadcast::Sender<RpcMessage>,
    shutdown: &mut watch::Receiver<()>,
) -> Result<()> {
    tracing::info!(%url, "connecting to Moonraker");

    let (ws, _resp) = tokio_tungstenite::connect_async(url.as_str())
        .await
        .map_err(|e| {
            layermind_shared::error::Error::Connection(format!("WebSocket connect failed: {e}"))
        })?;

    tracing::info!("WebSocket connected to Moonraker");

    let (mut ws_tx, mut ws_rx) = ws.split();

    // Authenticate if API key is configured.
    if let Some(ref api_key) = config.api_key {
        tracing::debug!("authenticating with API key");
        let auth_req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "access.oneshot_token",
            "params": { "token": api_key },
            "id": 1
        });
        ws_tx
            .send(tokio_tungstenite::tungstenite::Message::Text(
                auth_req.to_string().into(),
            ))
            .await
            .map_err(|e| {
                layermind_shared::error::Error::Connection(format!("auth send failed: {e}"))
            })?;
    }

    // Subscribe to printer objects.
    let objects = PrinterObject::all();
    let sub_req = protocol::subscribe_request(objects, 2);
    let sub_json = serde_json::to_string(&sub_req)
        .map_err(|e| layermind_shared::error::Error::Protocol(format!("serialize failed: {e}")))?;

    ws_tx
        .send(tokio_tungstenite::tungstenite::Message::Text(
            sub_json.into(),
        ))
        .await
        .map_err(|e| {
            layermind_shared::error::Error::Connection(format!("subscribe send failed: {e}"))
        })?;

    tracing::info!(
        objects = ?objects.iter().map(|o| o.as_key()).collect::<Vec<_>>(),
        "subscribed to printer objects"
    );

    // Reset attempt counter on successful connection.
    let mut last_heartbeat = tokio::time::Instant::now();
    let heartbeat_interval = Duration::from_secs_f64(config.heartbeat_interval_secs);

    // Read loop.
    loop {
        tokio::select! {
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                        match serde_json::from_str::<RpcMessage>(&text) {
                            Ok(rpc_msg) => {
                                if !rpc_msg.is_status_update() {
                                    tracing::debug!(method = ?rpc_msg.method, "Moonraker message received");
                                }
                                let _ = tx.send(rpc_msg);
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, raw = %text.chars().take(200).collect::<String>(), "failed to parse Moonraker message");
                            }
                        }
                        last_heartbeat = tokio::time::Instant::now();
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(data))) => {
                        ws_tx
                            .send(tokio_tungstenite::tungstenite::Message::Pong(data))
                            .await
                            .map_err(|e| layermind_shared::error::Error::Connection(format!("pong failed: {e}")))?;
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(frame))) => {
                        tracing::warn!(?frame, "Moonraker sent close frame");
                        return Err(layermind_shared::error::Error::Connection("server closed connection".into()));
                    }
                    Some(Err(e)) => {
                        return Err(layermind_shared::error::Error::Connection(format!("WebSocket error: {e}")));
                    }
                    None => {
                        tracing::warn!("Moonraker WebSocket stream ended");
                        return Err(layermind_shared::error::Error::Connection("stream ended".into()));
                    }
                    _ => {}
                }
            }
            _ = shutdown.changed() => {
                tracing::info!("shutdown received, closing Moonraker connection");
                let _ = ws_tx
                    .send(tokio_tungstenite::tungstenite::Message::Close(None))
                    .await;
                return Ok(());
            }
            _ = tokio::time::sleep(heartbeat_interval) => {
                let elapsed = last_heartbeat.elapsed();
                if elapsed > heartbeat_interval * 3 {
                    tracing::warn!(
                        elapsed_secs = elapsed.as_secs_f64(),
                        "Moonraker heartbeat timeout"
                    );
                    return Err(layermind_shared::error::Error::Connection("heartbeat timeout".into()));
                }
            }
        }
    }
}

/// Connect to Moonraker, query hardware info, and disconnect.
///
/// Returns `(server_info, printer_info, printer_objects)` as raw JSON values.
/// Times out after 10 seconds if responses are incomplete.
pub async fn query_hardware_info(
    config: &MoonrakerConfig,
) -> Result<(
    serde_json::Value,
    serde_json::Value,
    serde_json::Value,
)> {
    let url = url::Url::parse(&config.url)
        .map_err(|e| layermind_shared::error::Error::Connection(format!("invalid URL: {e}")))?;

    let connect_future = tokio_tungstenite::connect_async(url.as_str());
    let (ws, _resp) = tokio::time::timeout(Duration::from_secs(10), connect_future)
        .await
        .map_err(|_| {
            layermind_shared::error::Error::Connection("WebSocket connect timed out".into())
        })?
        .map_err(|e| {
            layermind_shared::error::Error::Connection(format!("WebSocket connect failed: {e}"))
        })?;

    let (mut ws_tx, mut ws_rx) = ws.split();

    // Authenticate if API key is configured.
    if let Some(ref api_key) = config.api_key {
        let auth_req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "access.oneshot_token",
            "params": { "token": api_key },
            "id": 0
        });
        ws_tx
            .send(tokio_tungstenite::tungstenite::Message::Text(
                auth_req.to_string().into(),
            ))
            .await
            .map_err(|e| {
                layermind_shared::error::Error::Connection(format!("auth send failed: {e}"))
            })?;
    }

    // Send three requests with sequential IDs.
    let objects = protocol::PrinterObject::all();
    let requests = vec![
        protocol::server_info_request(1),
        protocol::printer_info_request(2),
        protocol::query_request(objects, 3),
    ];

    for req in &requests {
        let json = serde_json::to_string(req).map_err(|e| {
            layermind_shared::error::Error::Protocol(format!("serialize failed: {e}"))
        })?;
        ws_tx
            .send(tokio_tungstenite::tungstenite::Message::Text(json.into()))
            .await
            .map_err(|e| {
                layermind_shared::error::Error::Connection(format!("request send failed: {e}"))
            })?;
    }

    // Collect responses by ID.
    let mut result_server: Option<serde_json::Value> = None;
    let mut result_printer: Option<serde_json::Value> = None;
    let mut result_objects: Option<serde_json::Value> = None;

    let deadline = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(deadline);

    let result = loop {
        tokio::select! {
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                        if let Ok(rpc_msg) = serde_json::from_str::<protocol::RpcMessage>(&text) {
                            if let (Some(id), Some(result)) = (&rpc_msg.id, &rpc_msg.result) {
                                match id {
                                    serde_json::Value::Number(n) if n.as_i64() == Some(1) => {
                                        result_server = Some(result.clone());
                                    }
                                    serde_json::Value::Number(n) if n.as_i64() == Some(2) => {
                                        result_printer = Some(result.clone());
                                    }
                                    serde_json::Value::Number(n) if n.as_i64() == Some(3) => {
                                        result_objects = Some(result.clone());
                                    }
                                    _ => {}
                                }
                            }

                            if result_server.is_some()
                                && result_printer.is_some()
                                && result_objects.is_some()
                            {
                                break Ok(());
                            }
                        }
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => {
                        break Err(layermind_shared::error::Error::Connection(
                            "server closed connection".into(),
                        ));
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        break Err(layermind_shared::error::Error::Connection(
                            format!("WebSocket error: {e}"),
                        ));
                    }
                    None => {
                        break Err(layermind_shared::error::Error::Connection(
                            "stream ended".into(),
                        ));
                    }
                }
            }
            () = &mut deadline => {
                break Err(layermind_shared::error::Error::Connection(
                    "query timed out after 10s".into(),
                ));
            }
        }
    };

    // Send close frame (best-effort).
    let _ = ws_tx
        .send(tokio_tungstenite::tungstenite::Message::Close(None))
        .await;

    result?;

    Ok((
        result_server.unwrap_or(serde_json::Value::Null),
        result_printer.unwrap_or(serde_json::Value::Null),
        result_objects.unwrap_or(serde_json::Value::Null),
    ))
}

/// Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s, 60s (capped).
fn backoff_duration(attempt: u32) -> Duration {
    let secs = 2.0_f64.powi(attempt.saturating_sub(1) as i32).min(60.0);
    Duration::from_secs_f64(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_increases_and_caps() {
        assert_eq!(backoff_duration(1), Duration::from_secs(1));
        assert_eq!(backoff_duration(2), Duration::from_secs(2));
        assert_eq!(backoff_duration(3), Duration::from_secs(4));
        assert_eq!(backoff_duration(4), Duration::from_secs(8));
        assert_eq!(backoff_duration(7), Duration::from_secs(60)); // 2^6 = 64 → capped
        assert_eq!(backoff_duration(10), Duration::from_secs(60));
    }
}
