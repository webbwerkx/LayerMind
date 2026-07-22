//! Knowledge types — the output of the Knowledge Engine.
//!
//! Knowledge records are higher-level than observations: they track
//! lifecycle state, aggregate printer profiles, and build chronological
//! timelines. The Knowledge Engine consumes Observations and produces
//! these structured Knowledge records.
//!
//! Future AI engines consume Knowledge as their primary input.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::observation::{AnomalyCategory, Severity};

// ── Knowledge Envelope ──────────────────────────────────────────────

/// A timestamped knowledge record emitted by the Knowledge Engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knowledge {
    pub id: Uuid,
    pub printer_id: String,
    pub timestamp: DateTime<Utc>,
    pub kind: KnowledgeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum KnowledgeKind {
    /// An observation has been recorded and scored.
    ObservationTracked {
        observation_id: Uuid,
        importance: f64,
        confidence: f64,
    },

    /// An observation has transitioned to resolved.
    ObservationResolved {
        observation_id: Uuid,
        resolution: String,
    },

    /// The printer profile has been updated with new information.
    ProfileUpdated { profile: PrinterProfile },

    /// A timeline event has been added.
    TimelineEventAdded { entry: TimelineEntry },

    /// Knowledge snapshot — emitted periodically for dashboards.
    KnowledgeSnapshot {
        active_observation_count: usize,
        resolved_observation_count: usize,
        profile_age_secs: f64,
        timeline_event_count: usize,
    },
}

// ── Printer Profile ─────────────────────────────────────────────────

/// Aggregated knowledge about a printer. Evolves over time as new
/// observations arrive and issues are resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterProfile {
    pub printer_id: String,
    pub hardware: PrinterHardware,
    pub behavior: PrinterBehavior,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrinterHardware {
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrinterBehavior {
    pub successful_prints: u64,
    pub failed_prints: u64,
    pub avg_print_duration_secs: Option<f64>,
    pub known_issues: Vec<KnownIssue>,
    pub reliability_score: Option<f64>,
}

/// A known issue tracked in the printer's profile. Issues accumulate
/// occurrence count and can be marked resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownIssue {
    pub category: AnomalyCategory,
    pub description: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub occurrence_count: u64,
    pub resolved: bool,
}

// ── Timeline ────────────────────────────────────────────────────────

/// A chronological entry in a printer's event timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub event_type: TimelineEventType,
    pub description: String,
    pub severity: Option<Severity>,
    pub metadata: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    CalibrationPerformed,
    NozzleChanged,
    FirmwareUpdated,
    FailureDetected,
    IssueResolved,
    PrintMilestone,
    MaintenancePerformed,
    Custom(String),
}

// ── Observation State ───────────────────────────────────────────────

/// Lifecycle state of a tracked observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationState {
    Active,
    Acknowledged,
    Resolved,
}

impl ObservationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Acknowledged => "acknowledged",
            Self::Resolved => "resolved",
        }
    }
}

// ── Constructors ────────────────────────────────────────────────────

impl Knowledge {
    pub fn new(printer_id: String, kind: KnowledgeKind) -> Self {
        Self {
            id: Uuid::now_v7(),
            printer_id,
            timestamp: Utc::now(),
            kind,
        }
    }
}

impl PrinterProfile {
    pub fn new(printer_id: String) -> Self {
        Self {
            printer_id,
            hardware: PrinterHardware::default(),
            behavior: PrinterBehavior::default(),
            updated_at: Utc::now(),
        }
    }
}

impl KnownIssue {
    pub fn new(category: AnomalyCategory, description: String, timestamp: DateTime<Utc>) -> Self {
        Self {
            category,
            description,
            first_seen: timestamp,
            last_seen: timestamp,
            occurrence_count: 1,
            resolved: false,
        }
    }

    pub fn bump(&mut self, timestamp: DateTime<Utc>) {
        self.occurrence_count += 1;
        self.last_seen = timestamp;
    }
}

impl TimelineEntry {
    pub fn new(
        event_type: TimelineEventType,
        description: String,
        severity: Option<Severity>,
        metadata: serde_json::Value,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            event_type,
            description,
            severity,
            metadata,
            occurred_at,
        }
    }
}
