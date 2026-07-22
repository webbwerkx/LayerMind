//! Context types — AI-consumable printer briefings.
//!
//! The Context Engine synthesizes knowledge into structured context
//! documents optimized for LLM consumption. Each piece of information
//! carries provenance (observed, inferred, or confirmed) so the AI
//! knows what is hard data and what is our best guess.
//!
//! Architecture:
//!   KnowledgeEngine → broadcast(Knowledge) → ContextEngine (caches state)
//!     → context(printer_id) → PrinterContext (AI-ready briefing)
//!
//! Future specialized views (TroubleshootingContext, CalibrationContext,
//! MaintenanceContext) are different projections over the same cached
//! state — no data duplication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Evidence Quality ────────────────────────────────────────────────

/// Provenance of a piece of information: how we know it.
///
/// Gives the AI a signal about what is hard sensor data vs. what is
/// derived by rules/heuristics vs. what a human has confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceQuality {
    /// Direct measurement from sensor data or printer-reported state.
    Observed,
    /// Derived by rule, heuristic, or statistical analysis.
    Inferred,
    /// Confirmed or resolved by a human operator.
    Confirmed,
}

impl EvidenceQuality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Inferred => "inferred",
            Self::Confirmed => "confirmed",
        }
    }
}

// ── Evidence ────────────────────────────────────────────────────────

/// A single piece of evidence — a fact with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// What kind of fact this is (e.g. "temperature_reading", "anomaly_detected").
    pub fact_type: String,
    /// Human-readable statement of the fact.
    pub statement: String,
    /// How we know this.
    pub quality: EvidenceQuality,
    /// Confidence in this fact (0.0–1.0).
    pub confidence: f64,
    /// When this fact was established.
    pub timestamp: DateTime<Utc>,
    /// Optional link to source observation or knowledge record.
    pub source_id: Option<Uuid>,
}

// ── Printer Context (general briefing) ──────────────────────────────

/// Complete AI-consumable briefing about one printer.
///
/// Designed to be injected into an LLM prompt for reasoning about
/// printer health, troubleshooting, or maintenance decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterContext {
    pub printer_id: String,
    pub generated_at: DateTime<Utc>,
    pub summary: PrinterSummary,
    pub print_history: PrintHistorySummary,
    pub health: HealthSummary,
    pub current_state: CurrentState,
    pub known_issues: Vec<IssueSummary>,
    pub historical_patterns: Vec<HistoricalPattern>,
    pub recent_evidence: Vec<Evidence>,
}

/// Identity, hardware, and current health snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterSummary {
    pub name: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_seen: Option<DateTime<Utc>>,
    pub reliability_score: Option<f64>,
    pub total_observations: u64,
    pub total_prints: u64,
}

/// Aggregated print history — what this printer has done.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintHistorySummary {
    pub total_prints: u64,
    pub successful_prints: u64,
    pub failed_prints: u64,
    pub success_rate: Option<f64>,
    pub avg_duration_secs: Option<f64>,
    pub recent_failures: Vec<RecentFailure>,
    pub common_failure_pattern: Option<String>,
}

/// A recent print failure with contextual detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFailure {
    pub timestamp: DateTime<Utc>,
    pub reason: Option<String>,
    pub failure_count_in_window: u64,
}

/// Current health indicators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub temperature_stability: Option<f64>,
    pub success_rate: Option<f64>,
    pub uptime_secs: f64,
    pub recent_error_count: u64,
    pub recent_warning_count: u64,
    pub reliability_score: Option<f64>,
}

/// What is happening right now.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentState {
    pub is_printing: bool,
    pub active_print_filename: Option<String>,
    pub active_observations: Vec<ObservationSummary>,
    pub pending_warnings: Vec<String>,
    pub recent_events: Vec<Evidence>,
}

/// A condensed observation for the current state view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationSummary {
    pub category: String,
    pub severity: String,
    pub message: String,
    pub importance: f64,
    pub confidence: f64,
    pub quality: EvidenceQuality,
    pub timestamp: DateTime<Utc>,
}

/// A known issue in the printer's profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSummary {
    pub category: String,
    pub description: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub occurrence_count: u64,
    pub resolved: bool,
    pub importance: f64,
}

/// A recurring pattern detected across multiple observations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalPattern {
    pub pattern_type: String,
    pub description: String,
    pub occurrence_count: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub typical_severity: String,
    pub resolved_count: u64,
}

// ── Constructors ────────────────────────────────────────────────────

impl PrinterContext {
    pub fn new(printer_id: String) -> Self {
        Self {
            printer_id,
            generated_at: Utc::now(),
            summary: PrinterSummary::default(),
            print_history: PrintHistorySummary::default(),
            health: HealthSummary::default(),
            current_state: CurrentState::default(),
            known_issues: Vec::new(),
            historical_patterns: Vec::new(),
            recent_evidence: Vec::new(),
        }
    }
}

impl Evidence {
    pub fn new(
        fact_type: String,
        statement: String,
        quality: EvidenceQuality,
        confidence: f64,
        timestamp: DateTime<Utc>,
        source_id: Option<Uuid>,
    ) -> Self {
        Self {
            fact_type,
            statement,
            quality,
            confidence,
            timestamp,
            source_id,
        }
    }

    pub fn observed(
        fact_type: &str,
        statement: &str,
        confidence: f64,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::new(
            fact_type.into(),
            statement.into(),
            EvidenceQuality::Observed,
            confidence,
            timestamp,
            None,
        )
    }

    pub fn inferred(
        fact_type: &str,
        statement: &str,
        confidence: f64,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self::new(
            fact_type.into(),
            statement.into(),
            EvidenceQuality::Inferred,
            confidence,
            timestamp,
            None,
        )
    }
}

impl Default for PrinterSummary {
    fn default() -> Self {
        Self {
            name: String::new(),
            model: None,
            firmware: None,
            first_seen: None,
            last_seen: None,
            reliability_score: None,
            total_observations: 0,
            total_prints: 0,
        }
    }
}

impl Default for PrintHistorySummary {
    fn default() -> Self {
        Self {
            total_prints: 0,
            successful_prints: 0,
            failed_prints: 0,
            success_rate: None,
            avg_duration_secs: None,
            recent_failures: Vec::new(),
            common_failure_pattern: None,
        }
    }
}

impl Default for HealthSummary {
    fn default() -> Self {
        Self {
            temperature_stability: None,
            success_rate: None,
            uptime_secs: 0.0,
            recent_error_count: 0,
            recent_warning_count: 0,
            reliability_score: None,
        }
    }
}

impl Default for CurrentState {
    fn default() -> Self {
        Self {
            is_printing: false,
            active_print_filename: None,
            active_observations: Vec::new(),
            pending_warnings: Vec::new(),
            recent_events: Vec::new(),
        }
    }
}
