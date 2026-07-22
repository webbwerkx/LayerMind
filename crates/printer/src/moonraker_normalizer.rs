//! Normalizes Moonraker JSON-RPC messages into canonical LayerMind events.
//!
//! Moonraker uses a structured object model. This module maps known
//! Moonraker printer objects into typed `Event` variants.

use layermind_moonraker::protocol::RawMessage;
use layermind_shared::event::Event;
use layermind_shared::types::Temperature;

/// Attempt to convert a raw Moonraker message into a canonical event.
/// Returns `None` if the message type is unrecognized or uninteresting.
pub fn normalize(raw: &RawMessage) -> Option<Event> {
    match raw.method.as_deref() {
        Some("notify_status_update") => normalize_status(raw),
        Some("notify_gcode_response") => normalize_gcode(raw),
        Some("notify_proc_stat_update") => None, // OS-level stats, skip
        _ => {
            // Surface unrecognized-but-potentially-useful events as Raw
            Some(Event::Raw {
                namespace: "moonraker".into(),
                key: raw.method.clone(),
                value: raw.params.clone().unwrap_or_default(),
            })
        }
    }
}

fn normalize_status(raw: &RawMessage) -> Option<Event> {
    // TODO: Parse Moonraker printer.objects.status response
    None
}

fn normalize_gcode(raw: &RawMessage) -> Option<Event> {
    let params = raw.params.as_ref()?;
    let response = params.as_array()?.first()?.as_str()?;
    Some(Event::GcodeResponse {
        command: String::new(),
        response: response.into(),
    })
}

/// Extract temperature readings from Moonraker's heater objects.
fn extract_temperatures(_params: &serde_json::Value) -> Vec<Temperature> {
    // TODO: Parse Moonraker temperature object structure
    vec![]
}
