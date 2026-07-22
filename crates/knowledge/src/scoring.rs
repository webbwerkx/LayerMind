//! Importance and confidence scoring for observations.
//!
//! Simple, deterministic scoring based on observation properties.
//! Not overcomplicated — importance is severity × repeat penalty,
//! confidence is based on evidence quantity.

use layermind_shared::observation::{AnomalyCategory, ObservationKind, Severity};

/// Scores an observation for importance and confidence.
#[derive(Debug, Default)]
pub struct KnowledgeScorer;

impl KnowledgeScorer {
    pub fn new() -> Self {
        Self
    }

    /// Score importance: 0.0 (trivial) to 1.0 (critical).
    ///
    /// Factors:
    /// - Severity weight (info=0.2, warning=0.5, critical=1.0)
    /// - Is it an anomaly? (×1.5 multiplier)
    /// - Repeat occurrences increase importance (logarithmic)
    pub fn importance(&self, kind: &ObservationKind, repeat_count: u64) -> f64 {
        let base = match kind {
            ObservationKind::AnomalyDetected { severity, .. } => match severity {
                Severity::Info => 0.3,
                Severity::Warning => 0.6,
                Severity::Critical => 0.9,
            },
            ObservationKind::PrintFailed { .. } => 0.7,
            ObservationKind::PrintSummary { success, .. } if !*success => 0.6,
            ObservationKind::PrintCompleted { .. } => 0.1,
            ObservationKind::HealthSnapshot { .. } => 0.15,
            _ => 0.2,
        };

        let repeat_bonus = if repeat_count > 1 {
            (repeat_count as f64).ln() * 0.1
        } else {
            0.0
        };

        (base + repeat_bonus).clamp(0.0, 1.0)
    }

    /// Score confidence: how certain we are this observation is meaningful.
    ///
    /// Factors:
    /// - Anomalies start at 0.7 (rules are heuristics, not guarantees)
    /// - Health snapshots are highly confident (0.9, deterministic calculations)
    /// - Print lifecycle events are high confidence (0.95, direct state transitions)
    /// - Evidence count boosts confidence
    pub fn confidence(&self, kind: &ObservationKind) -> f64 {
        let evidence_count = match kind {
            ObservationKind::AnomalyDetected { evidence, .. } => evidence.len(),
            _ => 1,
        };

        let base = match kind {
            ObservationKind::AnomalyDetected { .. } => 0.7,
            ObservationKind::HealthSnapshot { .. } => 0.9,
            ObservationKind::PrintSummary { .. } => 0.95,
            ObservationKind::PrintStarted { .. }
            | ObservationKind::PrintCompleted { .. }
            | ObservationKind::PrintFailed { .. } => 0.95,
        };

        let evidence_bonus = (evidence_count as f64 - 1.0).max(0.0) * 0.05;

        (base + evidence_bonus).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_anomaly_high_importance() {
        let scorer = KnowledgeScorer::new();
        let obs = ObservationKind::AnomalyDetected {
            category: AnomalyCategory::TemperatureInstability,
            severity: Severity::Critical,
            message: "test".into(),
            evidence: vec!["e1".into(), "e2".into()],
        };
        let imp = scorer.importance(&obs, 1);
        assert!(imp > 0.8);
    }

    #[test]
    fn repeat_anomalies_increase_importance() {
        let scorer = KnowledgeScorer::new();
        let obs = ObservationKind::AnomalyDetected {
            category: AnomalyCategory::RepeatedFailures,
            severity: Severity::Warning,
            message: "test".into(),
            evidence: vec!["e1".into()],
        };
        let single = scorer.importance(&obs, 1);
        let repeated = scorer.importance(&obs, 5);
        assert!(repeated > single);
    }

    #[test]
    fn health_snapshot_high_confidence() {
        let scorer = KnowledgeScorer::new();
        let obs = ObservationKind::HealthSnapshot {
            temperature_stability: 0.9,
            success_rate: Some(0.8),
            recent_error_count: 0,
            recent_warning_count: 0,
            seconds_since_calibration: None,
            uptime_secs: 3600.0,
        };
        assert!(scorer.confidence(&obs) > 0.85);
    }

    #[test]
    fn anomaly_confidence_scales_with_evidence() {
        let scorer = KnowledgeScorer::new();
        let one = ObservationKind::AnomalyDetected {
            category: AnomalyCategory::ExcessiveErrors,
            severity: Severity::Warning,
            message: "test".into(),
            evidence: vec!["e1".into()],
        };
        let many = ObservationKind::AnomalyDetected {
            category: AnomalyCategory::ExcessiveErrors,
            severity: Severity::Warning,
            message: "test".into(),
            evidence: vec!["e1".into(), "e2".into(), "e3".into(), "e4".into()],
        };
        assert!(scorer.confidence(&many) > scorer.confidence(&one));
    }
}
