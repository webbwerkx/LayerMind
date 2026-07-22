//! Observation types produced by the analyzer engine.
//!
//! Observations are derived insights — higher-level than raw telemetry events
//! but lower-level than AI recommendations. The analyzer produces these
//! deterministically from event windows and accumulated state.
//!
//! Future AI engines consume observations as input for recommendations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A timestamped observation about a printer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: Uuid,
    pub printer_id: String,
    pub timestamp: DateTime<Utc>,
    pub kind: ObservationKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ObservationKind {
    // ── Print Lifecycle ──────────────────────────────────────────
    PrintStarted {
        filename: String,
    },
    PrintCompleted {
        filename: String,
        duration_secs: f64,
        success: bool,
    },
    PrintFailed {
        filename: String,
        duration_secs: f64,
        reason: Option<String>,
    },

    // ── Health Metrics ───────────────────────────────────────────
    HealthSnapshot {
        /// Temperature stability score (0.0 = unstable, 1.0 = perfect).
        temperature_stability: f64,
        /// Print success rate over recent jobs (0.0–1.0).
        success_rate: Option<f64>,
        /// Error events in the current window.
        recent_error_count: u64,
        /// Warning events in the current window.
        recent_warning_count: u64,
        /// Seconds since last calibration (None if never calibrated).
        seconds_since_calibration: Option<f64>,
        /// Total connected uptime in seconds.
        uptime_secs: f64,
    },

    // ── Anomalies / Detections ───────────────────────────────────
    AnomalyDetected {
        category: AnomalyCategory,
        severity: Severity,
        message: String,
        evidence: Vec<String>,
    },

    // ── Print Summary ────────────────────────────────────────────
    PrintSummary {
        filename: String,
        success: bool,
        duration_secs: f64,
        filament_used_mm: Option<f64>,
        total_layers: Option<u32>,
        failure_reason: Option<String>,
        /// Key observations during the print.
        highlights: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyCategory {
    TemperatureInstability,
    ExcessiveErrors,
    RepeatedFailures,
    CalibrationOverdue,
    PrintTimeAnomaly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl Observation {
    pub fn new(printer_id: String, kind: ObservationKind) -> Self {
        Self {
            id: Uuid::now_v7(),
            printer_id,
            timestamp: Utc::now(),
            kind,
        }
    }
}
