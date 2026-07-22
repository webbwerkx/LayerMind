//! Printer abstraction types.

use serde::{Deserialize, Serialize};

/// Klipper-style printer states, mapped from Moonraker's reported states.
/// These are intentionally generic — every printer integration maps to these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrinterState {
    Idle,
    Printing,
    Paused,
    Pausing,
    Cancelling,
    Error,
    Complete,
    Standby,
    Unknown,
}

/// High-level printer metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInfo {
    pub id: String,
    pub name: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub state: PrinterState,
}
