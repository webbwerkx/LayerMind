//! Normalizes Moonraker JSON-RPC messages into canonical LayerMind events.
//!
//! Moonraker pushes `notify_status_update` notifications containing
//! printer object state. This module parses those objects and emits
//! typed `Event` variants. It also tracks state to avoid emitting
//! duplicate events when values haven't changed meaningfully.

use layermind_moonraker::protocol::RpcMessage;
use layermind_shared::event::Event;
use layermind_shared::types::Temperature;

/// Threshold for temperature change detection (°C).
const TEMP_CHANGE_THRESHOLD: f64 = 0.5;

/// Threshold for fan speed change detection.
const FAN_CHANGE_THRESHOLD: f64 = 0.02;

/// Threshold for position change detection (mm).
const POSITION_CHANGE_THRESHOLD: f64 = 0.1;

/// Threshold for speed change detection (mm/s).
const SPEED_CHANGE_THRESHOLD: f64 = 1.0;

/// Holds the last-seen values for change detection.
#[derive(Debug, Clone, Default)]
pub struct NormalizerState {
    pub last_temperatures: Vec<Temperature>,
    pub last_fan_speed: Option<f64>,
    pub last_position: Option<(f64, f64, f64)>,
    pub last_speed: Option<f64>,
    pub last_print_state: Option<String>,
}

impl NormalizerState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Process a raw RPC message and return zero or more canonical events.
pub fn normalize(msg: &RpcMessage, state: &mut NormalizerState) -> Vec<Event> {
    let mut events = Vec::new();

    if !msg.is_status_update() {
        if let Some(ref method) = msg.method {
            events.push(Event::Raw {
                namespace: "moonraker".into(),
                key: Some(method.clone()),
                value: msg.params.clone().unwrap_or_default(),
            });
        }
        return events;
    }

    let params = match &msg.params {
        Some(p) => p,
        None => return events,
    };

    let status = match layermind_moonraker::protocol::StatusUpdate::from_notification(params) {
        Some(s) => s,
        None => {
            tracing::warn!(
                raw = %params.to_string().chars().take(300).collect::<String>(),
                "failed to parse status update notification"
            );
            return events;
        }
    };

    // ── Temperatures ──────────────────────────────────────────────
    events.extend(normalize_temperatures(&status, state));

    // ── Print state ───────────────────────────────────────────────
    events.extend(normalize_print_state(&status, state));

    // ── Print progress ────────────────────────────────────────────
    events.extend(normalize_progress(&status));

    // ── Position ──────────────────────────────────────────────────
    events.extend(normalize_position(&status, state));

    // ── Speed ─────────────────────────────────────────────────────
    events.extend(normalize_speed(&status, state));

    // ── Fan ───────────────────────────────────────────────────────
    events.extend(normalize_fan(&status, state));

    events
}

fn normalize_temperatures(status: &serde_json::Value, state: &mut NormalizerState) -> Vec<Event> {
    let mut temps = Vec::new();

    if let Some(bed) = status.get("heater_bed") {
        if let Some(current) = bed.get("temperature").and_then(|v| v.as_f64()) {
            let target = bed.get("target").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let power = bed.get("power").and_then(|v| v.as_f64()).unwrap_or(0.0);
            temps.push(Temperature {
                sensor: "heater_bed_0".into(),
                current,
                target,
                power: Some(power),
            });
        }
    }

    if let Some(extruder) = status.get("extruder") {
        if let Some(current) = extruder.get("temperature").and_then(|v| v.as_f64()) {
            let target = extruder.get("target").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let power = extruder.get("power").and_then(|v| v.as_f64()).unwrap_or(0.0);
            temps.push(Temperature {
                sensor: "extruder_0".into(),
                current,
                target,
                power: Some(power),
            });
        }
    }

    if temps.is_empty() {
        return vec![];
    }

    let changed = temps.len() != state.last_temperatures.len()
        || temps
            .iter()
            .zip(state.last_temperatures.iter())
            .any(|(new, old)| {
                (new.current - old.current).abs() >= TEMP_CHANGE_THRESHOLD
                    || (new.target - old.target).abs() >= TEMP_CHANGE_THRESHOLD
            });

    if changed {
        state.last_temperatures = temps.clone();
        vec![Event::TemperatureUpdate {
            temperatures: temps,
        }]
    } else {
        vec![]
    }
}

fn normalize_print_state(status: &serde_json::Value, state: &mut NormalizerState) -> Vec<Event> {
    let ps = match status.get("print_stats") {
        Some(ps) => ps,
        None => return vec![],
    };

    let current_state = match ps.get("state").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return vec![],
    };

    let prev = state.last_print_state.clone();
    if prev.as_deref() == Some(current_state) {
        return vec![];
    }
    state.last_print_state = Some(current_state.to_string());

    let filename = ps
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match current_state {
        "printing" => vec![Event::PrintStarted {
            filename,
            estimated_time: None,
        }],
        "paused" => vec![Event::PrintPaused { reason: None }],
        "pausing" => vec![Event::PrintPaused {
            reason: Some("printer is pausing".into()),
        }],
        "complete" => {
            let total_time = ps
                .get("print_duration")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let filament_used = ps
                .get("filament_used")
                .and_then(|v| v.as_f64())
                .filter(|&v| v > 0.0);

            vec![Event::PrintCompleted {
                total_time,
                filament_used,
            }]
        }
        "error" => {
            let reason = ps
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            vec![Event::PrintFailed { reason: Some(reason) }]
        }
        "cancelled" => vec![Event::PrintCancelled],
        "standby" | "ready" | "idle" => vec![Event::StateChanged {
            state: layermind_shared::printer::PrinterState::Idle,
        }],
        _ => vec![],
    }
}

fn normalize_progress(status: &serde_json::Value) -> Vec<Event> {
    let sd = match status.get("virtual_sdcard") {
        Some(sd) => sd,
        None => return vec![],
    };

    let ps = match status.get("print_stats") {
        Some(ps) => ps,
        None => return vec![],
    };

    let is_active = sd
        .get("is_active")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !is_active {
        return vec![];
    }

    let progress = sd.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let elapsed = ps
        .get("print_duration")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let remaining = if progress > 0.0 && elapsed > 0.0 {
        let total_est = elapsed / progress;
        Some(total_est - elapsed)
    } else {
        None
    };

    let current_layer = ps
        .get("info")
        .and_then(|i| i.get("current_layer"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let total_layers = ps
        .get("info")
        .and_then(|i| i.get("total_layer"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    vec![Event::PrintProgress {
        progress,
        elapsed,
        remaining,
        current_layer,
        total_layers,
    }]
}

fn normalize_position(status: &serde_json::Value, state: &mut NormalizerState) -> Vec<Event> {
    let th = match status.get("toolhead") {
        Some(th) => th,
        None => return vec![],
    };

    let position = match th.get("position").and_then(|v| v.as_array()) {
        Some(pos) if pos.len() >= 3 => pos,
        _ => return vec![],
    };

    let x = position[0].as_f64().unwrap_or(0.0);
    let y = position[1].as_f64().unwrap_or(0.0);
    let z = position[2].as_f64().unwrap_or(0.0);

    let changed = match state.last_position {
        Some((lx, ly, lz)) => {
            (x - lx).abs() >= POSITION_CHANGE_THRESHOLD
                || (y - ly).abs() >= POSITION_CHANGE_THRESHOLD
                || (z - lz).abs() >= POSITION_CHANGE_THRESHOLD
        }
        None => true,
    };

    if changed {
        state.last_position = Some((x, y, z));
        vec![Event::PositionUpdate { x, y, z }]
    } else {
        vec![]
    }
}

fn normalize_speed(status: &serde_json::Value, state: &mut NormalizerState) -> Vec<Event> {
    let mr = match status.get("motion_report") {
        Some(mr) => mr,
        None => return vec![],
    };

    let speed = mr
        .get("live_velocity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let changed = match state.last_speed {
        Some(last) => (speed - last).abs() >= SPEED_CHANGE_THRESHOLD,
        None => true,
    };

    if changed {
        state.last_speed = Some(speed);
        vec![Event::SpeedUpdate { speed }]
    } else {
        vec![]
    }
}

fn normalize_fan(status: &serde_json::Value, state: &mut NormalizerState) -> Vec<Event> {
    let fan = match status.get("fan") {
        Some(f) => f,
        None => return vec![],
    };

    let speed = fan.get("speed").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let rpm = fan.get("rpm").and_then(|v| v.as_f64());

    let changed = match state.last_fan_speed {
        Some(last) => (speed - last).abs() >= FAN_CHANGE_THRESHOLD,
        None => true,
    };

    if changed {
        state.last_fan_speed = Some(speed);
        vec![Event::FanUpdate {
            name: "part_cooling".into(),
            speed,
            rpm,
        }]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_moonraker::protocol::RpcMessage;

    fn make_notification(status_obj: serde_json::Value) -> RpcMessage {
        RpcMessage {
            jsonrpc: "2.0".into(),
            method: Some("notify_status_update".into()),
            params: Some(serde_json::json!([status_obj, 50.0])),
            result: None,
            error: None,
            id: None,
        }
    }

    #[test]
    fn parses_temperature_update() {
        let status = serde_json::json!({
            "extruder": {"temperature": 210.0, "target": 210.0, "power": 0.3},
            "heater_bed": {"temperature": 60.0, "target": 60.0, "power": 0.0}
        });
        let msg = make_notification(status);
        let mut state = NormalizerState::new();

        let events = normalize(&msg, &mut state);
        let temp_event = events
            .iter()
            .find(|e| matches!(e, Event::TemperatureUpdate { .. }));

        assert!(temp_event.is_some(), "should emit TemperatureUpdate");
        if let Some(Event::TemperatureUpdate { temperatures }) = temp_event {
            let sensors: Vec<&str> = temperatures.iter().map(|t| t.sensor.as_str()).collect();
            assert!(sensors.contains(&"heater_bed_0"));
            assert!(sensors.contains(&"extruder_0"));
        }
    }

    #[test]
    fn handles_compressed_notification() {
        let status = serde_json::json!({
            "extruder": {"temperature": 27.19},
            "toolhead": {"estimated_print_time": 59.28}
        });
        let msg = make_notification(status);
        let mut state = NormalizerState::new();

        let events = normalize(&msg, &mut state);
        // Should not crash, may or may not emit events
        assert!(true, "should not panic on compressed notification");
    }

    #[test]
    fn parses_print_started() {
        let status = serde_json::json!({
            "print_stats": {"filename": "test.gcode", "state": "printing", "print_duration": 0.0, "filament_used": 0.0, "total_duration": 0.0, "message": "", "info": {}},
            "virtual_sdcard": {"progress": 0.0, "file_position": 0, "is_active": true, "file_path": null}
        });
        let msg = make_notification(status);
        let mut state = NormalizerState::new();

        let events = normalize(&msg, &mut state);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::PrintStarted { .. })),
            "should emit PrintStarted"
        );
    }

    #[test]
    fn parses_print_completed() {
        let status = serde_json::json!({
            "print_stats": {"filename": "done.gcode", "state": "complete", "print_duration": 3500.0, "filament_used": 5000.0, "total_duration": 3600.0, "message": "", "info": {}}
        });
        let msg = make_notification(status);
        let mut state = NormalizerState::new();

        let events = normalize(&msg, &mut state);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::PrintCompleted { .. })),
            "should emit PrintCompleted"
        );
    }

    #[test]
    fn suppresses_unchanged_temperatures() {
        let status = serde_json::json!({
            "extruder": {"temperature": 210.0, "target": 210.0, "power": 0.3},
            "heater_bed": {"temperature": 60.0, "target": 60.0, "power": 0.0}
        });
        let msg = make_notification(status);
        let mut state = NormalizerState::new();

        let events1 = normalize(&msg, &mut state);
        assert!(
            events1
                .iter()
                .any(|e| matches!(e, Event::TemperatureUpdate { .. }))
        );

        let events2 = normalize(&msg, &mut state);
        assert!(
            !events2
                .iter()
                .any(|e| matches!(e, Event::TemperatureUpdate { .. }))
        );
    }

    #[test]
    fn emits_on_significant_temp_change() {
        let status = serde_json::json!({
            "extruder": {"temperature": 210.0, "target": 210.0, "power": 0.3},
            "heater_bed": {"temperature": 60.0, "target": 60.0, "power": 0.0}
        });
        let msg = make_notification(status);
        let mut state = NormalizerState::new();

        let _ = normalize(&msg, &mut state);

        let status_changed = serde_json::json!({
            "extruder": {"temperature": 215.0, "target": 210.0, "power": 0.5},
            "heater_bed": {"temperature": 60.0, "target": 60.0, "power": 0.0}
        });
        let msg_changed = make_notification(status_changed);

        let events = normalize(&msg_changed, &mut state);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::TemperatureUpdate { .. })),
            "should emit on significant temp change"
        );
    }
}
