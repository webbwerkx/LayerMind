//! Historical timeline types — immutable records of printer state
//! changes over time.
//!
//! The timeline is the long-term memory of every printer. It records
//! what changed, when, and from what to what. Every future AI feature
//! consumes this timeline.
//!
//! # Architecture
//!
//! ```text
//! TimelineEvent ─── immutable change records (insert-only)
//! PrinterSnapshot ── periodic full-state captures
//! SnapshotDiff ──── computed delta between any two snapshots
//! TimelineQuery ─── strongly-typed filter for querying the timeline
//! ```
//!
//! This module is deterministic. No AI, no heuristics, no predictions.
//! It only records facts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::machine::{CapabilitySet, MachineHardware, MachineIdentity, MachineProfile};

// ── Timeline Event ──────────────────────────────────────────────────

/// An immutable record of a single change in printer state.
///
/// Events are insert-only — never updated, never deleted. The timeline
/// is an append-only log of facts. Every event carries:
/// - **When** it happened (timestamp)
/// - **What** changed (event kind + old/new values)
/// - **How** we know (source)
/// - **How certain** we are (confidence)
/// - **Extra context** (metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Unique identifier for this event.
    pub id: String,
    /// The printer this event belongs to.
    pub printer_id: String,
    /// When the event occurred (wall-clock time).
    pub timestamp: DateTime<Utc>,
    /// What kind of change this is.
    pub kind: TimelineEventKind,
    /// Where the information came from.
    pub source: TimelineEventSource,
    /// 0.0 = rumour, 1.0 = directly witnessed.
    pub confidence: f64,
    /// Arbitrary structured metadata (JSON).
    pub metadata: serde_json::Value,
}

// ── Event Kind ──────────────────────────────────────────────────────

/// Strongly-typed classification of every timeline event.
///
/// The variant names are the canonical taxonomy. Every event fits into
/// exactly one category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", content = "detail")]
pub enum TimelineEventKind {
    // ── Hardware ──────────────────────────────────────────
    Hardware(HardwareEvent),

    // ── Firmware ──────────────────────────────────────────
    Firmware(FirmwareEvent),

    // ── Configuration ─────────────────────────────────────
    Configuration(ConfigurationEvent),

    // ── Capability ────────────────────────────────────────
    Capability(CapabilityEvent),

    // ── Telemetry anomaly ─────────────────────────────────
    Anomaly(AnomalyEvent),

    // ── Print history ─────────────────────────────────────
    PrintHistory(PrintHistoryEvent),

    // ── Maintenance ───────────────────────────────────────
    Maintenance(MaintenanceEvent),

    // ── Snapshot taken ────────────────────────────────────
    SnapshotCreated(SnapshotKind),
}

// ── Hardware Events ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareEvent {
    pub action: HardwareAction,
    /// Which component changed (e.g. "extruder_0", "hotend").
    pub component_id: String,
    /// Human-readable component type.
    pub component_type: String,
    /// Previous value, serialized to string for storage.
    pub previous_value: Option<String>,
    /// New value, serialized to string for storage.
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareAction {
    Installed,
    Replaced,
    Removed,
    Upgraded,
    Configured,
    Discovered,
}

// ── Firmware Events ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareEvent {
    pub action: FirmwareAction,
    pub component: String, // "klipper", "moonraker", "mcu_mcu"
    pub previous_version: Option<String>,
    pub new_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirmwareAction {
    Updated,
    Downgraded,
    McuFirmwareChanged,
    ModuleLoaded,
    ModuleUnloaded,
}

// ── Configuration Events ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationEvent {
    pub action: ConfigAction,
    /// Config key that changed (e.g. "pressure_advance", "rotation_distance.x").
    pub config_key: String,
    pub previous_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigAction {
    Changed,
    Added,
    Removed,
    /// A full config reload / restart occurred.
    Reloaded,
    /// PID tuning completed.
    PidTuned,
    /// Input shaper calibration completed.
    InputShaperCalibrated,
    /// Bed mesh recalibrated.
    BedMeshGenerated,
    /// Probe offset calibrated.
    ProbeCalibrated,
    /// Pressure advance tuned.
    PressureAdvanceTuned,
}

// ── Capability Events ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEvent {
    pub action: CapabilityAction,
    /// Capability name (matches CapabilitySet field names).
    pub capability: String,
    pub previous_value: Option<bool>,
    pub new_value: Option<bool>,
    /// If a hardware change triggered this capability change.
    pub triggered_by_hardware_change: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityAction {
    Gained,
    Lost,
    /// Capability was already present but not previously recorded.
    Discovered,
}

// ── Anomaly Events ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEvent {
    pub severity: AnomalySeverity,
    pub anomaly_type: String,
    pub description: String,
    /// If this anomaly was associated with a specific print job.
    pub print_job_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Warning,
    Error,
    Critical,
}

// ── Print History Events ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintHistoryEvent {
    pub action: PrintAction,
    /// Optional print job ID for correlation.
    pub print_job_id: Option<String>,
    pub filename: Option<String>,
    pub duration_secs: Option<f64>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrintAction {
    Started,
    Completed,
    Failed,
    Cancelled,
    Paused,
    Resumed,
    /// A calibration print completed.
    CalibrationCompleted,
    /// First ever successful print on this printer.
    FirstSuccessful,
}

// ── Maintenance Events ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceEvent {
    pub action: MaintenanceAction,
    /// What was maintained (e.g. "nozzle", "belts", "rails").
    pub component: String,
    /// Free-form notes from the operator.
    pub notes: Option<String>,
    /// If this was scheduled vs. ad-hoc.
    pub scheduled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaintenanceAction {
    Replaced,
    Tightened,
    Lubricated,
    Cleaned,
    Inspected,
    ScheduledService,
    ManualEntry,
}

// ── Snapshot Kind ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotKind {
    Machine,
    Configuration,
    Capability,
    Knowledge,
    FullPrinter,
}

// ── Event Source ───────────────────────────────────────────────────

/// Where a timeline event originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineEventSource {
    /// Automatically detected by LayerMind's analysis pipeline.
    Automatic,
    /// Reported by the Moonraker API.
    Moonraker,
    /// Parsed from printer.cfg or included files.
    ConfigFile,
    /// Manually entered by the user.
    User,
    /// Loaded from an external import.
    Import,
    /// Generated by a snapshot diff.
    SnapshotDiff,
}

/// Top-level category for filtering queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimelineCategory {
    Hardware,
    Firmware,
    Configuration,
    Capability,
    Anomaly,
    PrintHistory,
    Maintenance,
    Snapshot,
}

impl From<&TimelineEventKind> for TimelineCategory {
    fn from(kind: &TimelineEventKind) -> Self {
        match kind {
            TimelineEventKind::Hardware(_) => TimelineCategory::Hardware,
            TimelineEventKind::Firmware(_) => TimelineCategory::Firmware,
            TimelineEventKind::Configuration(_) => TimelineCategory::Configuration,
            TimelineEventKind::Capability(_) => TimelineCategory::Capability,
            TimelineEventKind::Anomaly(_) => TimelineCategory::Anomaly,
            TimelineEventKind::PrintHistory(_) => TimelineCategory::PrintHistory,
            TimelineEventKind::Maintenance(_) => TimelineCategory::Maintenance,
            TimelineEventKind::SnapshotCreated(_) => TimelineCategory::Snapshot,
        }
    }
}

// ── Printer Snapshot ───────────────────────────────────────────────

/// A full point-in-time capture of a printer's complete state.
///
/// Snapshots are immutable records that capture everything LayerMind
/// knows about a printer at a specific moment. They enable:
/// - Diffs (what changed between then and now?)
/// - Historical queries (what did it look like last month?)
/// - Rollback analysis (was it healthy before this change?)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterSnapshot {
    pub printer_id: String,
    pub snapshot_id: String,
    pub timestamp: DateTime<Utc>,
    /// The machine profile at this point in time.
    pub machine: Option<MachineProfile>,
    /// The capability set at this point in time.
    pub capabilities: Option<CapabilitySet>,
    /// Configuration hash at this point in time.
    pub config_hash: Option<String>,
    /// What triggered this snapshot.
    pub trigger: SnapshotTrigger,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineSnapshot {
    pub printer_id: String,
    pub timestamp: DateTime<Utc>,
    pub identity: Option<MachineIdentity>,
    pub hardware: MachineHardware,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub printer_id: String,
    pub timestamp: DateTime<Utc>,
    pub capabilities: CapabilitySet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotTrigger {
    /// Automatically taken on a schedule (e.g. hourly).
    Scheduled,
    /// Taken because a significant change was detected.
    ChangeDetected,
    /// Manually requested by the user.
    Manual,
    /// Taken before a firmware update.
    PreUpdate,
    /// Taken after a firmware update.
    PostUpdate,
    /// Taken before a print starts.
    PrePrint,
    /// Taken after a print completes or fails.
    PostPrint,
}

// ── Snapshot Diff ──────────────────────────────────────────────────

/// The computed difference between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub from_snapshot_id: String,
    pub to_snapshot_id: String,
    pub from_timestamp: DateTime<Utc>,
    pub to_timestamp: DateTime<Utc>,
    /// List of individual changes found.
    pub changes: Vec<SnapshotChange>,
    /// Total number of changes detected.
    pub total_changes: usize,
}

/// A single atomic change between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotChange {
    /// What changed (e.g. "hardware.extruders[0].details.extruder_type").
    pub path: String,
    pub category: TimelineCategory,
    pub previous_value: Option<String>,
    pub new_value: Option<String>,
}

// ── Timeline Query ─────────────────────────────────────────────────

/// A strongly-typed query for filtering timeline events.
///
/// All fields are optional; only specified filters are applied.
/// An empty query returns all events.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelineQuery {
    pub printer_id: Option<String>,
    pub categories: Option<Vec<TimelineCategory>>,
    pub component_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub event_kinds: Option<Vec<String>>,
    /// Maximum number of events to return.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

// ── Timeline Statistics ────────────────────────────────────────────

/// Aggregate statistics computed from a slice of timeline events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStatistics {
    pub total_events: usize,
    pub events_by_category: Vec<(TimelineCategory, usize)>,
    pub first_event: Option<DateTime<Utc>>,
    pub last_event: Option<DateTime<Utc>>,
    pub hardware_changes: usize,
    pub firmware_updates: usize,
    pub config_changes: usize,
    pub anomalies_detected: usize,
    pub maintenance_actions: usize,
    pub total_prints: usize,
    pub failed_prints: usize,
    pub successful_prints: usize,
}

impl TimelineStatistics {
    pub fn compute(events: &[TimelineEvent]) -> Self {
        use std::collections::HashMap;

        let mut cats: HashMap<TimelineCategory, usize> = HashMap::new();
        let mut hw = 0usize;
        let mut fw = 0usize;
        let mut cfg = 0usize;
        let mut anom = 0usize;
        let mut maint = 0usize;
        let mut total_prints = 0usize;
        let mut failed = 0usize;
        let mut success = 0usize;
        let mut first: Option<DateTime<Utc>> = None;
        let mut last: Option<DateTime<Utc>> = None;

        for e in events {
            let cat = TimelineCategory::from(&e.kind);
            *cats.entry(cat).or_insert(0) += 1;

            match &e.kind {
                TimelineEventKind::Hardware(_) => hw += 1,
                TimelineEventKind::Firmware(_) => fw += 1,
                TimelineEventKind::Configuration(_) => cfg += 1,
                TimelineEventKind::Anomaly(_) => anom += 1,
                TimelineEventKind::Maintenance(_) => maint += 1,
                TimelineEventKind::PrintHistory(pe) => {
                    total_prints += 1;
                    match pe.action {
                        PrintAction::Failed => failed += 1,
                        PrintAction::Completed | PrintAction::FirstSuccessful => success += 1,
                        _ => {}
                    }
                }
                _ => {}
            }

            if first.is_none() || e.timestamp < first.unwrap() {
                first = Some(e.timestamp);
            }
            if last.is_none() || e.timestamp > last.unwrap() {
                last = Some(e.timestamp);
            }
        }

        let mut events_by_category: Vec<_> = cats.into_iter().collect();
        events_by_category.sort_by_key(|(_, c)| std::cmp::Reverse(*c));

        Self {
            total_events: events.len(),
            events_by_category,
            first_event: first,
            last_event: last,
            hardware_changes: hw,
            firmware_updates: fw,
            config_changes: cfg,
            anomalies_detected: anom,
            maintenance_actions: maint,
            total_prints,
            failed_prints: failed,
            successful_prints: success,
        }
    }
}

// ── History Summary (for Context) ──────────────────────────────────

/// A digest of recent history for inclusion in [`PrinterContext`].
///
/// This is a lightweight summary — no trend analysis, no predictions.
/// It answers "what changed recently?" without the full timeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistorySummary {
    pub last_hardware_change: Option<DateTime<Utc>>,
    pub last_firmware_update: Option<DateTime<Utc>>,
    pub last_config_change: Option<DateTime<Utc>>,
    pub last_calibration: Option<DateTime<Utc>>,
    pub last_maintenance: Option<DateTime<Utc>>,
    pub recent_changes: Vec<RecentChange>,
    pub total_events: u64,
    pub config_age_days: Option<f64>,
    pub hardware_age_days: Option<f64>,
}

/// A recent change, summarized for context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentChange {
    pub timestamp: DateTime<Utc>,
    pub category: TimelineCategory,
    pub summary: String,
}

// ── History Index ──────────────────────────────────────────────────

/// An in-memory index over timeline events for fast lookups.
///
/// Not persisted — rebuilt on startup from the database. Provides
/// O(log n) lookups by printer, category, and time range.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryIndex {
    /// Events grouped by printer_id for fast per-printer queries.
    pub by_printer: std::collections::HashMap<String, Vec<TimelineEvent>>,
}
