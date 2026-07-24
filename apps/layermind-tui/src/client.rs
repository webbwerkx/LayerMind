use crate::app::{AppState, EventLevel, PrinterSnapshot};
use chrono::Utc;
use layermind_config::MoonrakerConfig;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Poll Moonraker for the latest printer state and update the app state.
pub async fn poll_moonraker(config: &MoonrakerConfig, state: &Arc<Mutex<AppState>>) {
    let (server_info, printer_info, printer_objects) =
        match layermind_moonraker::client::query_hardware_info(config).await {
            Ok(r) => r,
            Err(e) => {
                let mut app = state.lock().await;
                if app.connected {
                    app.connected = false;
                    app.connection_error = Some(e.to_string());
                    app.add_event(format!("Disconnected: {e}"), EventLevel::Error);
                }
                return;
            }
        };

    let mut app = state.lock().await;

    if !app.connected {
        app.connected = true;
        app.connecting = false;
        app.connection_error = None;
        app.add_event("Connected to Moonraker", EventLevel::Info);
    }

    app.last_refresh = Some(Utc::now());

    // Parse hostname from printer_info.
    app.printer.hostname = printer_info
        .get("hostname")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Parse klipper/moonraker versions.
    app.printer.klipper_version = printer_info
        .get("klipper_path")
        .and_then(|v| v.as_str())
        .map(String::from);

    app.printer.moonraker_version = server_info
        .get("result")
        .and_then(|r| r.get("moonraker_version"))
        .or_else(|| server_info.get("moonraker_version"))
        .and_then(|v| v.as_str())
        .map(String::from);

    // Parse printer objects from the status response.
    let status = printer_objects
        .get("result")
        .and_then(|r| r.get("status"))
        .or_else(|| printer_objects.get("status"));

    let Some(status) = status else {
        return;
    };

    // Extruder.
    if let Some(ext) = status.get("extruder") {
        app.printer.extruder_temp = ext.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.0);
        app.printer.extruder_target = ext.get("target").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }

    // Heater bed.
    if let Some(bed) = status.get("heater_bed") {
        app.printer.bed_temp = bed.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.0);
        app.printer.bed_target = bed.get("target").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }

    // Toolhead.
    if let Some(th) = status.get("toolhead") {
        if let Some(pos) = th.get("position").and_then(|v| v.as_array()) {
            for (i, p) in pos.iter().enumerate().take(4) {
                if let Some(v) = p.as_f64() {
                    app.printer.position[i] = v;
                }
            }
        }
        app.printer.homed_axes = th
            .get("homed_axes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }

    // Fan.
    if let Some(fan) = status.get("fan") {
        app.printer.fan_speed = fan.get("speed").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }

    // Print stats.
    if let Some(ps) = status.get("print_stats") {
        let new_state = ps.get("state").and_then(|v| v.as_str()).unwrap_or("unknown");
        if new_state != app.printer.state {
            let old_state = app.printer.state.clone();
            app.printer.state = new_state.to_string();
            if old_state != "printing" && new_state == "printing" {
                app.add_event("Print started", EventLevel::Info);
            } else if old_state == "printing" && new_state == "complete" {
                app.add_event("Print completed", EventLevel::Info);
            } else if old_state == "printing" && new_state == "error" {
                app.add_event("Print failed", EventLevel::Error);
            }
        }
        app.printer.print_elapsed = ps
            .get("print_duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
    }

    // Virtual SD card.
    if let Some(vsd) = status.get("virtual_sdcard") {
        let new_progress = vsd.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if (new_progress - app.printer.print_progress).abs() > 0.01 {
            app.printer.print_progress = new_progress;
        }
    }

    // Gcode move for speed.
    if let Some(gc) = status.get("gcode_move") {
        app.printer.speed = gc.get("speed").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
}

/// Parse printer objects JSON into a PrinterSnapshot (standalone, no state).
pub fn parse_printer_objects(printer_objects: &serde_json::Value) -> PrinterSnapshot {
    let mut snap = PrinterSnapshot::default();

    let status = printer_objects
        .get("result")
        .and_then(|r| r.get("status"))
        .or_else(|| printer_objects.get("status"));

    let Some(status) = status else {
        return snap;
    };

    if let Some(ext) = status.get("extruder") {
        snap.extruder_temp = ext.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.0);
        snap.extruder_target = ext.get("target").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    if let Some(bed) = status.get("heater_bed") {
        snap.bed_temp = bed.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.0);
        snap.bed_target = bed.get("target").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    if let Some(th) = status.get("toolhead") {
        if let Some(pos) = th.get("position").and_then(|v| v.as_array()) {
            for (i, p) in pos.iter().enumerate().take(4) {
                if let Some(v) = p.as_f64() {
                    snap.position[i] = v;
                }
            }
        }
        snap.homed_axes = th.get("homed_axes").and_then(|v| v.as_str()).unwrap_or("").to_string();
    }
    if let Some(fan) = status.get("fan") {
        snap.fan_speed = fan.get("speed").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    if let Some(ps) = status.get("print_stats") {
        snap.state = ps.get("state").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        snap.print_filename = ps.get("filename").and_then(|v| v.as_str()).map(String::from);
        snap.print_elapsed = ps.get("print_duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    if let Some(vsd) = status.get("virtual_sdcard") {
        snap.print_progress = vsd.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    if let Some(gc) = status.get("gcode_move") {
        snap.speed = gc.get("speed").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }

    snap
}


