//! Integration tests for the Moonraker client against a mock server.
//!
//! Spins up a local WebSocket server that speaks a subset of the Moonraker
//! JSON-RPC protocol, then connects the real MoonrakerClient and verifies
//! message flow, reconnection, and shutdown behavior.

use futures_util::{SinkExt, StreamExt};
use layermind_config::MoonrakerConfig;
use layermind_moonraker::MoonrakerClient;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_tungstenite::accept_async;

/// Start a mock Moonraker server on a random port. Returns the address
/// and a shutdown sender. The server responds to subscriptions with a
/// single status update, then waits for shutdown.
async fn start_mock_server() -> (SocketAddr, watch::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (shutdown_tx, mut shutdown_rx) = watch::channel(());

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    let mut shutdown = shutdown_rx.clone();
                    tokio::spawn(async move {
                        handle_connection(stream, &mut shutdown).await;
                    });
                }
                _ = shutdown_rx.changed() => break,
            }
        }
    });

    (addr, shutdown_tx)
}

/// Handle a single WebSocket connection from a client.
async fn handle_connection(stream: TcpStream, shutdown: &mut watch::Receiver<()>) {
    let ws = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(_) => return,
    };

    let (mut ws_tx, mut ws_rx) = ws.split();

    // Send initial state (Klipper ready + status update).
    let initial = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notify_klippy_ready",
        "params": []
    });
    let _ = ws_tx
        .send(tokio_tungstenite::tungstenite::Message::Text(
            initial.to_string().into(),
        ))
        .await;

    let status = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notify_status_update",
        "params": [{
            "heater_bed": { "temperatures": [60.0], "target": 60.0, "power": 0.0 },
            "extruder": { "temperatures": [210.0], "target": 210.0, "power": 0.3 },
            "print_stats": {
                "filename": "test.gcode", "total_duration": 100.0,
                "print_duration": 50.0, "filament_used": 1200.0,
                "state": "printing", "message": "",
                "info": { "total_layer": 100, "current_layer": 50 }
            },
            "virtual_sdcard": {
                "progress": 0.5, "file_position": 5000, "is_active": true
            },
            "toolhead": {
                "position": [100.0, 100.0, 5.0], "status": "Ready", "homed_axes": "xyz"
            },
            "motion_report": {
                "live_position": [100.0, 100.0, 5.0],
                "live_velocity": 50.0,
                "live_extruder_velocity": 5.0,
                "steppers": ["x", "y", "z", "e"]
            },
            "fan": { "speed": 0.5, "rpm": null }
        }]
    });
    let _ = ws_tx
        .send(tokio_tungstenite::tungstenite::Message::Text(
            status.to_string().into(),
        ))
        .await;

    // Send a second status update so we can verify progress changes.
    let status2 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notify_status_update",
        "params": [{
            "heater_bed": { "temperatures": [60.0], "target": 60.0, "power": 0.0 },
            "extruder": { "temperatures": [210.0], "target": 210.0, "power": 0.3 },
            "print_stats": {
                "filename": "test.gcode", "total_duration": 200.0,
                "print_duration": 100.0, "filament_used": 2400.0,
                "state": "printing", "message": "",
                "info": { "total_layer": 100, "current_layer": 50 }
            },
            "virtual_sdcard": {
                "progress": 0.75, "file_position": 7500, "is_active": true
            },
            "toolhead": {
                "position": [120.0, 110.0, 5.0], "status": "Ready", "homed_axes": "xyz"
            },
            "motion_report": {
                "live_position": [120.0, 110.0, 5.0],
                "live_velocity": 60.0,
                "live_extruder_velocity": 5.0,
                "steppers": ["x", "y", "z", "e"]
            },
            "fan": { "speed": 0.8, "rpm": null }
        }]
    });
    let _ = ws_tx
        .send(tokio_tungstenite::tungstenite::Message::Text(
            status2.to_string().into(),
        ))
        .await;

    // Wait for shutdown or client disconnect.
    loop {
        tokio::select! {
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => break,
                    None => break,
                    _ => {}
                }
            }
            _ = shutdown.changed() => break,
        }
    }
}

#[tokio::test]
async fn client_connects_and_receives_messages() {
    let (addr, shutdown_tx) = start_mock_server().await;
    let url = format!("ws://{}", addr);

    let config = MoonrakerConfig {
        url,
        api_key: None,
        reconnect_interval_secs: 1.0,
        heartbeat_interval_secs: 5.0,
    };

    let (client, mut rx) = MoonrakerClient::new(config);
    let (client_shutdown_tx, client_shutdown_rx) = watch::channel(());

    // Run client in background.
    let client_handle = tokio::spawn(async move { client.run(client_shutdown_rx).await });

    // Collect messages for a brief period.
    let mut messages = Vec::new();
    let collect_duration = Duration::from_secs(2);

    let collect_result = timeout(collect_duration, async {
        loop {
            match rx.recv().await {
                Ok(msg) => messages.push(msg),
                Err(_) => break,
            }
        }
    })
    .await;

    // Timeout is expected — we're just collecting for a fixed window.
    assert!(collect_result.is_err() || !messages.is_empty());

    // Verify we received at least the status update notification.
    let has_status = messages
        .iter()
        .any(|m| m.method.as_deref() == Some("notify_status_update"));
    assert!(has_status, "should receive status update messages");

    // Verify we received klippy_ready.
    let has_ready = messages
        .iter()
        .any(|m| m.method.as_deref() == Some("notify_klippy_ready"));
    assert!(has_ready, "should receive klippy ready");

    // Shutdown.
    let _ = client_shutdown_tx.send(());
    let _ = shutdown_tx.send(());

    // Don't block if client fails — it may fail due to connection reset.
    let _ = timeout(Duration::from_secs(2), client_handle).await;
}

#[tokio::test]
async fn client_handles_reconnection() {
    // Start and immediately kill the server to test reconnect behavior.
    let (addr, shutdown_tx) = start_mock_server().await;
    let url = format!("ws://{}", addr);

    let config = MoonrakerConfig {
        url: url.clone(),
        api_key: None,
        reconnect_interval_secs: 0.1, // Fast reconnect for test
        heartbeat_interval_secs: 5.0,
    };

    let (client, _rx) = MoonrakerClient::new(config);
    let (client_shutdown_tx, client_shutdown_rx) = watch::channel(());

    let client_handle = tokio::spawn(async move { client.run(client_shutdown_rx).await });

    // Let the client connect.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Kill the server.
    let _ = shutdown_tx.send(());
    drop(shutdown_tx);

    // Give the client time to detect disconnection and attempt reconnect.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The client should still be running (trying to reconnect).
    assert!(
        !client_handle.is_finished(),
        "client should still be running during reconnect"
    );

    // Shutdown the client.
    let _ = client_shutdown_tx.send(());
    let _ = timeout(Duration::from_secs(2), client_handle).await;
}

#[tokio::test]
async fn client_shuts_down_on_signal() {
    let (addr, shutdown_tx) = start_mock_server().await;
    let url = format!("ws://{}", addr);

    let config = MoonrakerConfig {
        url,
        api_key: None,
        reconnect_interval_secs: 1.0,
        heartbeat_interval_secs: 5.0,
    };

    let (client, _rx) = MoonrakerClient::new(config);
    let (client_shutdown_tx, client_shutdown_rx) = watch::channel(());

    let client_handle = tokio::spawn(async move { client.run(client_shutdown_rx).await });

    // Let it connect.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Send shutdown.
    let _ = client_shutdown_tx.send(());

    // Client should exit cleanly.
    let result = timeout(Duration::from_secs(2), client_handle).await;
    assert!(
        result.is_ok(),
        "client should shut down within timeout after signal"
    );

    // Cleanup mock server.
    let _ = shutdown_tx.send(());
}
