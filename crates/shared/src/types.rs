//! Common data types used throughout LayerMind.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single temperature reading from a named sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Temperature {
    pub sensor: String,
    pub current: f64,
    pub target: f64,
    pub power: Option<f64>,
}

/// Fan state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanState {
    pub name: String,
    pub speed: f64,
    pub rpm: Option<f64>,
}

/// A material/filament entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilamentInfo {
    pub id: Option<String>,
    pub material: String,
    pub brand: Option<String>,
    pub color: Option<String>,
    pub diameter: f64,
    pub spool_weight: Option<f64>,
    pub cost: Option<f64>,
}

/// Axis position in millimeters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A summary of a completed or failed print.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintSummary {
    pub print_id: String,
    pub filename: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub total_time: f64,
    pub filament_used_mm: Option<f64>,
    pub total_layers: Option<u32>,
    pub failure_reason: Option<String>,
}
