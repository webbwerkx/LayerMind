//! Learning types — discovered patterns, trends, and behavioral
//! insights from historical printer data.
//!
//! This module defines the canonical types for the learning engine.
//! Everything is deterministic — no AI, no heuristics, no predictions.
//! The learning engine analyzes recorded history and produces
//! structured findings that the AI can consume via context.
//!
//! # Architecture
//!
//! ```text
//! TimelineEvents ───→ PatternDetector ───→ Vec<LearnedPattern>
//!                   ───→ TrendAnalyzer  ───→ Vec<TrendReport>
//!                   ───→ PrintComparer  ───→ PrintComparison
//!                   ───→ CalibrationTracker ───→ CalibrationSummary
//!                   ───→ ComponentAging   ───→ AgingReport
//!                                    │
//!                            BehaviorSummary
//!                                    │
//!                            PrinterContext.learning
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Learned Pattern ─────────────────────────────────────────────────

/// A pattern discovered by analyzing timeline events.
///
/// Patterns are discovered mechanically through frequency analysis,
/// sequence detection, and correlation. No AI involved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// What was discovered.
    pub description: String,
    /// Classification of the pattern.
    pub kind: PatternKind,
    /// How many occurrences were found.
    pub occurrences: u64,
    /// When the pattern was first observed.
    pub first_seen: DateTime<Utc>,
    /// When the pattern was most recently observed.
    pub last_seen: DateTime<Utc>,
    /// How confident we are this is a real pattern (0.0–1.0).
    pub confidence: f64,
    /// Related event IDs that form this pattern.
    pub related_events: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternKind {
    /// Same failure occurring repeatedly.
    RecurringFailure,
    /// Configuration change followed by improvement.
    ConfigImprovement,
    /// Configuration change followed by degradation.
    ConfigRegression,
    /// Repeated calibration within short time window.
    CalibrationLoop,
    /// Failure rate increasing over time.
    DegradationTrend,
    /// Success rate improving after a change.
    ImprovementTrend,
    /// Hardware replacement followed by behavior change.
    HardwareImpact,
    /// Seasonal or time-based pattern.
    PeriodicBehavior,
    /// A sequence of events that commonly occur together.
    EventCluster,
    /// Not yet classified.
    Unclassified,
}

// ── Trend Report ────────────────────────────────────────────────────

/// Statistical trend analysis over a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendReport {
    /// What metric this trend is about.
    pub metric: String,
    /// The time window analyzed.
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    /// Number of data points.
    pub sample_count: u64,
    /// Average value over the window.
    pub average: f64,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// Direction of the trend.
    pub direction: TrendDirection,
    /// Rate of change per day.
    pub change_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Worsening,
    Stable,
    /// Not enough data to determine.
    InsufficientData,
}

// ── Print Comparison ────────────────────────────────────────────────

/// Side-by-side comparison of two prints or print periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintComparison {
    pub before_period: TimeWindow,
    pub after_period: TimeWindow,
    pub success_rate_change: f64,
    pub avg_duration_change_secs: f64,
    pub failure_count_before: u64,
    pub failure_count_after: u64,
    pub significant_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub total_prints: u64,
    pub successful_prints: u64,
    pub failed_prints: u64,
}

// ── Calibration Summary ─────────────────────────────────────────────

/// Analysis of calibration history for a printer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationSummary {
    /// How many calibrations total.
    pub total_calibrations: u64,
    /// Calibration types and their counts.
    pub by_type: Vec<(String, u64)>,
    /// Average days between calibrations.
    pub avg_interval_days: Option<f64>,
    /// Which calibration is performed most frequently.
    pub most_frequent: Option<String>,
    /// When the last calibration occurred.
    pub last_calibration: Option<DateTime<Utc>>,
    /// Calibrations that may be overdue (>2x avg interval).
    pub overdue: Vec<String>,
}

// ── Component Aging ─────────────────────────────────────────────────

/// Estimated component aging based on usage and time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgingReport {
    pub component_id: String,
    pub component_type: String,
    /// When the component was installed.
    pub installed: Option<DateTime<Utc>>,
    /// Estimated age in days since installation.
    pub age_days: Option<f64>,
    /// Estimated remaining life in days (based on typical lifespan).
    pub estimated_remaining_days: Option<f64>,
    /// Reason for the aging estimate.
    pub estimation_basis: String,
    /// Wear indicators observed in data.
    pub wear_indicators: Vec<String>,
}

// ── Behavior Summary ────────────────────────────────────────────────

/// A digest of everything the learning engine has discovered about a
/// printer. This is the main integration point into PrinterContext.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorSummary {
    /// Recurring patterns discovered.
    pub patterns: Vec<LearnedPattern>,
    /// Statistical trends over recent history.
    pub trends: Vec<TrendReport>,
    /// Print quality comparison (before/after last N prints).
    pub print_comparison: Option<PrintComparison>,
    /// Calibration history analysis.
    pub calibration: Option<CalibrationSummary>,
    /// Component aging estimates.
    pub aging: Vec<AgingReport>,
    /// Per-component health assessments from the prediction engine.
    pub component_health: Vec<ComponentHealth>,
    /// When this summary was generated.
    pub generated_at: DateTime<Utc>,
    /// Total timeline events analyzed.
    pub events_analyzed: u64,
    /// Total patterns discovered.
    pub patterns_found: u64,
}

// ── Failure Cluster ─────────────────────────────────────────────────

/// A cluster of related failures — same error, same component,
/// occurring within a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureCluster {
    pub failure_type: String,
    pub component: Option<String>,
    pub count: u64,
    pub first_occurrence: DateTime<Utc>,
    pub last_occurrence: DateTime<Utc>,
    /// True if failures are still occurring in recent history.
    pub is_active: bool,
}

// ── Configuration Drift ─────────────────────────────────────────────

/// Tracks how a configuration value has changed over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDrift {
    pub config_key: String,
    pub values: Vec<(DateTime<Utc>, String)>,
    pub change_count: u64,
    pub first_value: Option<String>,
    pub current_value: Option<String>,
}

// ── Print Success Window ────────────────────────────────────────────

/// Analyzes print success rates over sliding time windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessWindow {
    pub window_days: i64,
    pub total_prints: u64,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub most_common_failure: Option<String>,
}

// ── Component Health ────────────────────────────────────────────────

/// Health assessment for a single component, derived from historical
/// patterns and trend analysis. Used by the Failure Prediction engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component identifier (matches the ID used in timeline events).
    pub component_id: String,
    /// Human-readable component type ("probe", "hotend", "extruder", etc.).
    pub component_type: String,
    /// Overall health score. 1.0 = healthy, 0.0 = imminent failure.
    pub health_score: f64,
    /// Active warnings for this component.
    pub warnings: Vec<ComponentWarning>,
    /// Rate of health decline per day. Positive = getting worse.
    pub degradation_rate: f64,
    /// Number of anomalies attributed to this component.
    pub anomaly_count: u64,
    /// When the health was last assessed.
    pub assessed_at: DateTime<Utc>,
}

/// A specific warning about a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentWarning {
    pub severity: WarningSeverity,
    pub message: String,
    pub detected_at: DateTime<Utc>,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    /// Minor degradation detected — monitor.
    Early,
    /// Significant degradation — plan maintenance soon.
    Moderate,
    /// Component is likely to fail soon — action recommended.
    Critical,
}

impl BehaviorSummary {
    /// Create a new empty summary with just the generation timestamp.
    pub fn new(events_analyzed: u64) -> Self {
        Self {
            events_analyzed,
            generated_at: Utc::now(),
            ..Default::default()
        }
    }

    /// Components currently in warning or critical state.
    pub fn unhealthy_components(&self) -> Vec<&ComponentHealth> {
        self.component_health
            .iter()
            .filter(|ch| ch.health_score < 0.7)
            .collect()
    }
}
