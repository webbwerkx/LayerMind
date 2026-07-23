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
pub mod patterns;
pub mod prints;
pub mod trends;

use chrono::Utc;
use layermind_shared::history::{
    AnomalyEvent, AnomalySeverity, TimelineCategory, TimelineEvent, TimelineEventKind,
};
use layermind_shared::learning::*;

/// Top-level learning engine — orchestrates all sub-analyzers and
/// produces a complete BehaviorSummary.
#[derive(Debug)]
pub struct LearningEngine;

impl LearningEngine {
    /// Analyze a slice of timeline events and produce a complete
    /// behavior summary.
    pub fn analyze(events: &[TimelineEvent], printer_id: &str) -> BehaviorSummary {
        let patterns = patterns::PatternDetector::detect(events);
        let trends = trends::TrendAnalyzer::analyze(events);
        let print_comparison = prints::PrintAnalyzer::compare_recent(events);
        let calibration = calibration::CalibrationTracker::summarize(events);

        let patterns_found = patterns.len() as u64;

        BehaviorSummary {
            patterns,
            trends,
            print_comparison,
            calibration,
            aging: Vec::new(), // Phase 3.2
            generated_at: Utc::now(),
            events_analyzed: events.len() as u64,
            patterns_found,
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

pub(crate) fn category_count(events: &[TimelineEvent], cat: TimelineCategory) -> usize {
    events
        .iter()
        .filter(|e| TimelineCategory::from(&e.kind) == cat)
        .count()
}

pub(crate) fn iter_of_kind<T, F>(events: &[TimelineEvent], f: F) -> Vec<(&TimelineEvent, &T)>
where
    F: Fn(&TimelineEventKind) -> Option<&T>,
{
    events
        .iter()
        .filter_map(|e| f(&e.kind).map(|t| (e, t)))
        .collect()
}

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
        let summary = LearningEngine::analyze(&[], "p1");
        assert_eq!(summary.events_analyzed, 0);
        assert!(summary.patterns.is_empty());
        assert!(summary.trends.is_empty());
    }

    #[test]
    fn detects_recurring_anomalies() {
        let events: Vec<_> = (0..5)
            .map(|_| anomaly("thermal warning", AnomalySeverity::Warning))
            .collect();
        let summary = LearningEngine::analyze(&events, "p1");
        assert!(summary.patterns_found > 0);
    }
}
