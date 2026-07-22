//! Database entity models.
//!
//! These mirror the PostgreSQL schema. Each struct maps to a table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Printer {
    pub id: Uuid,
    pub name: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintJob {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub filename: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: Option<bool>,
    pub filament_used_mm: Option<f64>,
    pub total_layers: Option<i32>,
    pub failure_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub print_job_id: Option<Uuid>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filament {
    pub id: Uuid,
    pub material: String,
    pub brand: Option<String>,
    pub color: Option<String>,
    pub diameter: f64,
    pub spool_weight_g: Option<f64>,
    pub cost_per_kg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub id: Uuid,
    pub print_job_id: Uuid,
    pub category: String,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub resolved: bool,
}
