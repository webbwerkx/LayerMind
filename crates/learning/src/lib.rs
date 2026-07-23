//! Learning engine — deterministic pattern detection, trend analysis,
//! and statistical insight from historical printer data.
//!
//! # Architecture
//!
//! ```text
//! TimelineEvents ──→ PatternDetector      → Vec<LearnedPattern>
//!                 ──→ TrendAnalyzer        → Vec<TrendReport>
//!                 ──→ PrintAnalyzer        → PrintComparison
//!                 ──→ CalibrationTracker   → CalibrationSummary
//!                 ──→ LearningEngine       → BehaviorSummary
//! ```
//!
//! All engines are deterministic. No AI, no heuristics, no predictions.
//! They analyze recorded history and produce structured findings.
//!
//! Dependencies: shared only. Never reasoning, never ai.

pub mod calibration;
pub mod optimization;
pub mod patterns;
pub mod prediction;
pub mod prints;
pub mod trends;

use chrono::Utc;
use layermind_shared::history::TimelineEvent;
use layermind_shared::learning::*;
use layermind_shared::machine::MachineProfile;

/// Top-level learning engine — orchestrates all sub-analyzers and
/// produces a complete BehaviorSummary.
#[derive(Debug)]
pub struct LearningEngine;

impl LearningEngine {
    /// Analyze a slice of timeline events and produce a complete
    /// behavior summary.
    pub fn analyze(events: &[TimelineEvent], machine: Option<&MachineProfile>) -> BehaviorSummary {
        let patterns = patterns::PatternDetector::detect(events);
        let trends = trends::TrendAnalyzer::analyze(events);
        let print_comparison = prints::PrintAnalyzer::compare_recent(events);
        let calibration = calibration::CalibrationTracker::summarize(events);

        let patterns_found = patterns.len() as u64;

        // Build partial summary for the prediction engine to consume.
        let partial = BehaviorSummary {
            patterns: patterns.clone(),
            trends: trends.clone(),
            print_comparison: print_comparison.clone(),
            calibration: calibration.clone(),
            generated_at: Utc::now(),
            events_analyzed: events.len() as u64,
            patterns_found,
            ..BehaviorSummary::default()
        };

        let aging = prediction::FailurePredictor::predict(events, &partial);
        let component_health = prediction::FailurePredictor::component_health(events, &partial);

        // Build a richer summary for optimization (includes health + aging).
        let full_partial = BehaviorSummary {
            patterns: patterns.clone(),
            trends: trends.clone(),
            print_comparison: print_comparison.clone(),
            calibration: calibration.clone(),
            aging: aging.clone(),
            component_health: component_health.clone(),
            generated_at: Utc::now(),
            events_analyzed: events.len() as u64,
            patterns_found,
            ..BehaviorSummary::default()
        };
        let optimization = optimization::OptimizationEngine::analyze(&full_partial, machine);

        BehaviorSummary {
            patterns,
            trends,
            print_comparison,
            calibration,
            aging,
            component_health,
            optimization: Some(optimization),
            generated_at: Utc::now(),
            events_analyzed: events.len() as u64,
            patterns_found,
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_shared::history::*;

    fn anomaly(desc: &str, severity: AnomalySeverity) -> TimelineEvent {
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: "p1".into(),
            timestamp: Utc::now(),
            kind: TimelineEventKind::Anomaly(AnomalyEvent {
                severity,
                anomaly_type: "test".into(),
                description: desc.into(),
                print_job_id: None,
            }),
            source: TimelineEventSource::Automatic,
            confidence: 0.9,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn empty_events_produces_empty_summary() {
        let summary = LearningEngine::analyze(&[], None);
        assert_eq!(summary.events_analyzed, 0);
        assert!(summary.patterns.is_empty());
        assert!(summary.trends.is_empty());
    }

    #[test]
    fn detects_recurring_anomalies() {
        let events: Vec<_> = (0..5)
            .map(|_| anomaly("thermal warning", AnomalySeverity::Warning))
            .collect();
        let summary = LearningEngine::analyze(&events, None);
        assert!(summary.patterns_found > 0);
    }
}
