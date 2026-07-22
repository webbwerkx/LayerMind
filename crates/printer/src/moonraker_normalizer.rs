//! Normalizes Moonraker JSON-RPC messages into canonical LayerMind events.
//!
//! Moonraker pushes `notify_status_update` notifications containing
//! printer object state. This module parses those objects and emits
//! typed `Event` variants. It also tracks state to avoid emitting
//! duplicate events when values haven't changed meaningfully.

use layermind_moonraker::protocol::{RpcMessage, StatusUpdate};
use layermind_shared::event::Event;
use layermind_shared::types::Temperature;

/// Threshold for temperature change detection (°C).
/// We don't emit TemperatureUpdate if the change is below this.
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
///
/// This is the main entry point. It extracts the status update from
/// Moonraker notifications, parses each printer object, and emits
/// typed events when values have changed.
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

    let status = match StatusUpdate::from_notification(params) {
        Some(s) => s,
        None => {
            tracing::warn!("failed to parse status update notification");
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

fn normalize_temperatures(status: &StatusUpdate, state: &mut NormalizerState) -> Vec<Event> {
    let mut temps = Vec::new();

    if let Some(ref bed) = status.heater_bed {
        for (i, &current) in bed.temperatures.iter().enumerate() {
            temps.push(Temperature {
                sensor: format!("heater_bed_{i}"),
                current,
                target: bed.target,
                power: Some(bed.power),
            });
        }
    }

    if let Some(ref extruder) = status.extruder {
        for (i, &current) in extruder.temperatures.iter().enumerate() {
            temps.push(Temperature {
                sensor: format!("extruder_{i}"),
                current,
                target: extruder.target,
                power: Some(extruder.power),
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

fn normalize_print_state(status: &StatusUpdate, state: &mut NormalizerState) -> Vec<Event> {
    let ps = match &status.print_stats {
        Some(ps) => ps,
        None => return vec![],
    };

    let current_state = &ps.state;

    // Only emit when state actually changes.
    let prev = state.last_print_state.clone();
    if prev.as_deref() == Some(current_state) {
        return vec![];
    }
    state.last_print_state = Some(current_state.clone());

    match current_state.as_str() {
        "printing" => {
            vec![Event::PrintStarted {
                filename: ps.filename.clone(),
                estimated_time: None,
            }]
        }
        "paused" => {
            vec![Event::PrintPaused { reason: None }]
        }
        "pausing" => {
            vec![Event::PrintPaused {
                reason: Some("printer is pausing".into()),
            }]
        }
        "complete" => {
            vec![Event::PrintCompleted {
                total_time: ps.print_duration,
                filament_used: if ps.filament_used > 0.0 {
                    Some(ps.filament_used)
                } else {
                    None
                },
            }]
        }
        "error" => {
            vec![Event::PrintFailed {
                reason: Some(ps.message.clone()),
            }]
        }
        "cancelled" => {
            vec![Event::PrintCancelled]
        }
        "standby" | "ready" | "idle" => {
            vec![Event::StateChanged {
                state: layermind_shared::printer::PrinterState::Idle,
            }]
        }
        _ => vec![],
    }
}

fn normalize_progress(status: &StatusUpdate) -> Vec<Event> {
    let sd = match &status.virtual_sdcard {
        Some(sd) => sd,
        None => return vec![],
    };

    let ps = match &status.print_stats {
        Some(ps) => ps,
        None => return vec![],
    };

    if !sd.is_active {
        return vec![];
    }

    let elapsed = ps.print_duration;
    let progress = sd.progress;

    // Estimate remaining time from progress and elapsed.
    let remaining = if progress > 0.0 && elapsed > 0.0 {
        let total_est = elapsed / progress;
        Some(total_est - elapsed)
    } else {
        None
    };

    vec![Event::PrintProgress {
        progress,
        elapsed,
        remaining,
        current_layer: ps.info.current_layer,
        total_layers: ps.info.total_layer,
    }]
}

fn normalize_position(status: &StatusUpdate, state: &mut NormalizerState) -> Vec<Event> {
    let th = match &status.toolhead {
        Some(th) => th,
        None => return vec![],
    };

    if th.position.len() < 3 {
        return vec![];
    }

    let x = th.position[0];
    let y = th.position[1];
    let z = th.position[2];

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

fn normalize_speed(status: &StatusUpdate, state: &mut NormalizerState) -> Vec<Event> {
    let mr = match &status.motion_report {
        Some(mr) => mr,
        None => return vec![],
    };

    let speed = mr.live_velocity;

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

fn normalize_fan(status: &StatusUpdate, state: &mut NormalizerState) -> Vec<Event> {
    let fan = match &status.fan {
        Some(f) => f,
        None => return vec![],
    };

    let changed = match state.last_fan_speed {
        Some(last) => (fan.speed - last).abs() >= FAN_CHANGE_THRESHOLD,
        None => true,
    };

    if changed {
        state.last_fan_speed = Some(fan.speed);
        vec![Event::FanUpdate {
            name: "part_cooling".into(),
            speed: fan.speed,
            rpm: fan.rpm,
        }]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_status_json(overrides: &str) -> serde_json::Value {
        let base = r#"{
            "heater_bed": { "temperatures": [60.0], "target": 60.0, "power": 0.0 },
            "extruder": { "temperatures": [210.0], "target": 210.0, "power": 0.3 },
            "print_stats": { "filename": "test.gcode", "total_duration": 100.0, "print_duration": 50.0, "filament_used": 1200.0, "state": "printing", "message": "", "info": { "total_layer": 100, "current_layer": 50 } },
            "virtual_sdcard": { "progress": 0.5, "file_position": 5000, "is_active": true },
            "toolhead": { "position": [100.0, 100.0, 5.0], "status": "Ready", "homed_axes": "xyz" },
            "motion_report": { "live_position": [100.0, 100.0, 5.0], "live_velocity": 50.0, "live_extruder_velocity": 5.0, "steppers": ["x","y","z","e"] },
            "gcode_move": { "speed": 6000, "speed_factor": 1.0, "extrude_factor": 1.0, "absolute_coordinates": true, "absolute_extrude": true, "position": [100.0, 100.0, 5.0], "homing_origin": [0.0, 0.0, 0.0, 0.0] },
            "fan": { "speed": 0.5, "rpm": null }
        }"#;

        let mut base_val: serde_json::Value = serde_json::from_str(base).unwrap();
        if !overrides.is_empty() {
            let ov: serde_json::Value = serde_json::from_str(overrides).unwrap();
            if let (serde_json::Value::Object(b), serde_json::Value::Object(o)) =
                (&mut base_val, ov)
            {
                for (k, v) in o {
                    b.insert(k, v);
                }
            }
        }
        base_val
    }

    fn make_status_msg(value: &serde_json::Value) -> RpcMessage {
        RpcMessage {
            jsonrpc: "2.0".into(),
            method: Some("notify_status_update".into()),
            params: Some(serde_json::json!([value])),
            result: None,
            error: None,
            id: None,
        }
    }

    #[test]
    fn parses_temperature_update() {
        let val = make_status_json("");
        let msg = make_status_msg(&val);
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
    fn parses_print_started() {
        let val = make_status_json("");
        let msg = make_status_msg(&val);
        let mut state = NormalizerState::new();

        let events = normalize(&msg, &mut state);
        let print_event = events
            .iter()
            .find(|e| matches!(e, Event::PrintStarted { .. }));

        assert!(print_event.is_some(), "should emit PrintStarted");
    }

    #[test]
    fn parses_progress() {
        let val = make_status_json("");
        let msg = make_status_msg(&val);
        let mut state = NormalizerState::new();

        let events = normalize(&msg, &mut state);
        let progress_event = events
            .iter()
            .find(|e| matches!(e, Event::PrintProgress { .. }));

        assert!(progress_event.is_some(), "should emit PrintProgress");
        if let Some(Event::PrintProgress { progress, .. }) = progress_event {
            assert!((*progress - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn suppresses_unchanged_temperatures() {
        let val = make_status_json("");
        let msg = make_status_msg(&val);
        let mut state = NormalizerState::new();

        // First call emits.
        let events1 = normalize(&msg, &mut state);
        assert!(
            events1
                .iter()
                .any(|e| matches!(e, Event::TemperatureUpdate { .. }))
        );

        // Second call with same values suppresses.
        let events2 = normalize(&msg, &mut state);
        assert!(
            !events2
                .iter()
                .any(|e| matches!(e, Event::TemperatureUpdate { .. }))
        );
    }

    #[test]
    fn emits_on_significant_temp_change() {
        let val = make_status_json("");
        let msg = make_status_msg(&val);
        let mut state = NormalizerState::new();

        let _ = normalize(&msg, &mut state);

        let val_changed = make_status_json(
            r#"{"extruder": {"temperatures": [215.0], "target": 210.0, "power": 0.5}}"#,
        );
        let msg_changed = make_status_msg(&val_changed);

        let events = normalize(&msg_changed, &mut state);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::TemperatureUpdate { .. })),
            "should emit on significant temp change"
        );
    }

    #[test]
    fn parses_print_completed() {
        let val = make_status_json(
            r#"{"print_stats": {"filename": "done.gcode", "total_duration": 3600.0, "print_duration": 3500.0, "filament_used": 5000.0, "state": "complete", "message": "", "info": {}}}"#,
        );
        let msg = make_status_msg(&val);
        let mut state = NormalizerState::new();

        let events = normalize(&msg, &mut state);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::PrintCompleted { .. })),
            "should emit PrintCompleted"
        );
    }
}
