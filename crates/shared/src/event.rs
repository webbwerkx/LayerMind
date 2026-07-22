//! Canonical event types flowing through the LayerMind event bus.
//!
//! Every event in the system is one of these variants. Integration crates
//! (moonraker, future octoprint, etc.) produce raw protocol messages; the
//! printer crate normalizes them into these canonical types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::printer::PrinterState;
use crate::types::Temperature;

/// A timestamped event from a specific printer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub event_id: Uuid,
    pub printer_id: String,
    pub timestamp: DateTime<Utc>,
    pub payload: Event,
}

/// Every observable printer event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    // --- Lifecycle ---
    Connected,
    Disconnected {
        reason: String,
    },
    PrinterReady,

    // --- State ---
    StateChanged {
        state: PrinterState,
    },

    // --- Thermal ---
    TemperatureUpdate {
        temperatures: Vec<Temperature>,
    },
    HeaterFault {
        heater: String,
        message: String,
    },

    // --- Motion ---
    PositionUpdate {
        x: f64,
        y: f64,
        z: f64,
    },
    SpeedUpdate {
        speed: f64,
    },

    // --- Print Job ---
    PrintStarted {
        filename: String,
        estimated_time: Option<f64>,
    },
    PrintProgress {
        progress: f64,
        elapsed: f64,
        remaining: Option<f64>,
        current_layer: Option<u32>,
        total_layers: Option<u32>,
    },
    PrintPaused {
        reason: Option<String>,
    },
    PrintResumed,
    PrintCompleted {
        total_time: f64,
        filament_used: Option<f64>,
    },
    PrintFailed {
        reason: Option<String>,
    },
    PrintCancelled,

    // --- G-code ---
    GcodeResponse {
        command: String,
        response: String,
    },

    // --- Errors & Warnings ---
    Error {
        code: Option<String>,
        message: String,
    },
    Warning {
        message: String,
    },

    // --- Raw passthrough (for unclassified events) ---
    Raw {
        namespace: String,
        key: Option<String>,
        value: serde_json::Value,
    },
}
