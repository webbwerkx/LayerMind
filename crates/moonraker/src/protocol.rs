//! Moonraker protocol types and JSON-RPC message handling.
//!
//! Moonraker speaks JSON-RPC 2.0 over WebSocket. This module defines
//! the request/response types, the printer object status model, and
//! helpers for constructing subscription requests.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ── JSON-RPC 2.0 Primitives ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

impl RpcRequest {
    pub fn new(method: &str, params: Option<Value>, id: u64) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id: Some(Value::Number(id.into())),
        }
    }
}

/// A raw JSON-RPC message from Moonraker (request, response, or notification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMessage {
    pub jsonrpc: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<RpcError>,
    #[serde(default)]
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

// ── Subscription Building ────────────────────────────────────────────

/// Objects we subscribe to from Moonraker's printer.objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrinterObject {
    HeaterBed,
    Extruder,
    PrintStats,
    VirtualSdCard,
    Toolhead,
    MotionReport,
    GcodeMove,
    Fan,
}

impl PrinterObject {
    pub fn as_key(&self) -> &'static str {
        match self {
            Self::HeaterBed => "heater_bed",
            Self::Extruder => "extruder",
            Self::PrintStats => "print_stats",
            Self::VirtualSdCard => "virtual_sdcard",
            Self::Toolhead => "toolhead",
            Self::MotionReport => "motion_report",
            Self::GcodeMove => "gcode_move",
            Self::Fan => "fan",
        }
    }

    /// All printer objects we subscribe to for intelligence.
    pub fn all() -> &'static [PrinterObject] {
        &[
            Self::HeaterBed,
            Self::Extruder,
            Self::PrintStats,
            Self::VirtualSdCard,
            Self::Toolhead,
            Self::MotionReport,
            Self::GcodeMove,
            Self::Fan,
        ]
    }
}

/// Build a `printer.objects.subscribe` request.
pub fn subscribe_request(objects: &[PrinterObject], id: u64) -> RpcRequest {
    let obj_map: HashMap<&str, Value> = objects.iter().map(|o| (o.as_key(), Value::Null)).collect();

    RpcRequest::new(
        "printer.objects.subscribe",
        Some(serde_json::json!({ "objects": obj_map })),
        id,
    )
}

/// Build a `printer.objects.query` request for a one-shot status snapshot.
pub fn query_request(objects: &[PrinterObject], id: u64) -> RpcRequest {
    let obj_map: HashMap<&str, Value> = objects.iter().map(|o| (o.as_key(), Value::Null)).collect();

    RpcRequest::new(
        "printer.objects.query",
        Some(serde_json::json!({ "objects": obj_map })),
        id,
    )
}

/// Build a `server.info` request to get Moonraker server information.
pub fn server_info_request(id: u64) -> RpcRequest {
    RpcRequest::new("server.info", None, id)
}

// ── Moonraker Printer Object Status ───────────────────────────────────

/// The parsed status response from a `notify_status_update` notification.
/// Moonraker sends a list with a single element containing object states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    #[serde(default, rename = "heater_bed")]
    pub heater_bed: Option<HeaterState>,

    #[serde(default)]
    pub extruder: Option<HeaterState>,

    #[serde(default, rename = "print_stats")]
    pub print_stats: Option<PrintStats>,

    #[serde(default, rename = "virtual_sdcard")]
    pub virtual_sdcard: Option<VirtualSdCard>,

    #[serde(default)]
    pub toolhead: Option<ToolheadState>,

    #[serde(default, rename = "motion_report")]
    pub motion_report: Option<MotionReport>,

    #[serde(default, rename = "gcode_move")]
    pub gcode_move: Option<GcodeMove>,

    #[serde(default)]
    pub fan: Option<FanState>,
}

/// Heater state (used for bed, extruder, chamber, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaterState {
    pub temperatures: Vec<f64>,
    pub target: f64,
    pub power: f64,
}

/// Print job statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintStats {
    #[serde(default)]
    pub filename: String,

    #[serde(default, rename = "total_duration")]
    pub total_duration: f64,

    #[serde(default, rename = "print_duration")]
    pub print_duration: f64,

    #[serde(default, rename = "filament_used")]
    pub filament_used: f64,

    #[serde(default)]
    pub state: String,

    #[serde(default)]
    pub message: String,

    #[serde(default)]
    pub info: PrintInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrintInfo {
    #[serde(default, rename = "total_layer")]
    pub total_layer: Option<u32>,

    #[serde(default, rename = "current_layer")]
    pub current_layer: Option<u32>,
}

/// Virtual SD card state (file progress).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSdCard {
    pub progress: f64,

    #[serde(rename = "file_position")]
    pub file_position: i64,

    #[serde(default, rename = "is_active")]
    pub is_active: bool,

    #[serde(default, rename = "file_path")]
    pub file_path: Option<String>,
}

/// Toolhead state (position and status).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolheadState {
    pub position: Vec<f64>,

    #[serde(default)]
    pub status: String,

    #[serde(default, rename = "homed_axes")]
    pub homed_axes: String,
}

/// Motion report with live kinematics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionReport {
    #[serde(default, rename = "live_position")]
    pub live_position: Vec<f64>,

    #[serde(default, rename = "live_velocity")]
    pub live_velocity: f64,

    #[serde(default, rename = "live_extruder_velocity")]
    pub live_extruder_velocity: f64,

    #[serde(default)]
    pub steppers: Vec<String>,
}

/// G-code move state (feedrate, flow, coordinate mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcodeMove {
    pub speed: f64,

    #[serde(rename = "speed_factor")]
    pub speed_factor: f64,

    #[serde(rename = "extrude_factor")]
    pub extrude_factor: f64,

    #[serde(rename = "absolute_coordinates")]
    pub absolute_coordinates: bool,

    #[serde(rename = "absolute_extrude")]
    pub absolute_extrude: bool,

    pub position: Vec<f64>,

    #[serde(default)]
    pub homing_origin: Vec<f64>,
}

/// Fan state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanState {
    pub speed: f64,
    pub rpm: Option<f64>,
}

// ── Helpers ──────────────────────────────────────────────────────────

impl StatusUpdate {
    /// Attempt to parse a `notify_status_update` notification params.
    /// Moonraker wraps the status object in a single-element array:
    /// `{ "params": [{ ...status... }] }`
    pub fn from_notification(params: &Value) -> Option<Self> {
        let arr = params.as_array()?;
        let obj = arr.first()?;
        serde_json::from_value(obj.clone()).ok()
    }
}

impl RpcMessage {
    /// True if this message is a notification (method present, no id).
    pub fn is_notification(&self) -> bool {
        self.method.is_some() && self.id.is_none()
    }

    /// True if this is a status update notification.
    pub fn is_status_update(&self) -> bool {
        self.method.as_deref() == Some("notify_status_update")
    }
}
