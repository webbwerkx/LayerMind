//! Database entity models.
//!
//! Each struct maps to a PostgreSQL table row. These are the canonical
//! representations used by sqlx for query results. The `FromRow` derive
//! enables automatic row-to-struct mapping.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ── Printer ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Printer {
    pub id: Uuid,
    pub name: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

// ── Print Job ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PrintJob {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub filename: String,
    pub status: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration: Option<f64>,
}

// ── Telemetry Event ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TelemetryEvent {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub print_job_id: Option<Uuid>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}

// ── Calibration Event ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CalibrationEvent {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub cal_type: String,
    pub values: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}

// ── AI Observation ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AiObservation {
    pub id: Uuid,
    pub printer_id: Uuid,
    pub category: String,
    pub observation: String,
    pub confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
}
