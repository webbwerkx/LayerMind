//! Moonraker protocol types and JSON-RPC message handling.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A raw JSON-RPC message from Moonraker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMessage {
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

/// Moonraker-specific subscription namespaces.
#[derive(Debug, Clone, Copy)]
pub enum Subscription {
    PrinterObjects,
    GcodeResponses,
    Timelapse,
    Announcements,
}
