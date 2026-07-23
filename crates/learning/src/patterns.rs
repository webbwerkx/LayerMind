//! PatternDetector — discovers recurring patterns in timeline events.
//!
//! Detects:
//! - Recurring failures (same failure type, ≥3 occurrences)
//! - Configuration regressions (config change → increased failures)
//! - Calibration loops (same calibration repeated within short window)
//! - Event clusters (related events occurring close together)
//!
//! All detection is frequency-based and threshold-driven. No AI.

use chrono::Duration;
use layermind_shared::history::*;
use layermind_shared::learning::*;

/// Detects patterns in a sequence of timeline events.
#[derive(Debug)]
pub struct PatternDetector;

impl PatternDetector {
    /// Detect all patterns in an event slice.
    pub fn detect(events: &[TimelineEvent]) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();

        patterns.extend(Self::detect_recurring_failures(events));
        patterns.extend(Self::detect_config_regressions(events));
        patterns.extend(Self::detect_calibration_loops(events));
        patterns.extend(Self::detect_event_clusters(events));

        patterns
    }

    /// Find failure types that occur ≥3 times.
    fn detect_recurring_failures(events: &[TimelineEvent]) -> Vec<LearnedPattern> {
        use std::collections::HashMap;

        // Group failures by description.
        let mut failures: HashMap<String, Vec<&TimelineEvent>> = HashMap::new();

        for e in events {
            if let TimelineEventKind::Anomaly(ref ae) = e.kind {
                failures.entry(ae.description.clone()).or_default().push(e);
            }
        }

        failures
            .into_iter()
            .filter(|(_, evs)| evs.len() >= 3)
            .map(|(desc, evs)| {
                let first = evs.iter().map(|e| e.timestamp).min().unwrap();
                let last = evs.iter().map(|e| e.timestamp).max().unwrap();
                let ids: Vec<String> = evs.iter().map(|e| e.id.clone()).collect();
                let count = evs.len() as u64;

                LearnedPattern {
                    description: format!("Recurring failure: {}", desc),
                    kind: PatternKind::RecurringFailure,
                    occurrences: count,
                    first_seen: first,
                    last_seen: last,
                    confidence: (count as f64 * 0.1).min(1.0),
                    related_events: ids,
                }
            })
            .collect()
    }

    /// Detect config changes followed by increased failure rate.
    fn detect_config_regressions(events: &[TimelineEvent]) -> Vec<LearnedPattern> {
        let mut patterns = Vec::new();

        for (i, e) in events.iter().enumerate() {
            if matches!(e.kind, TimelineEventKind::Configuration(_)) {
                // Look at the 5 events following this config change.
                let failures_after = events[i..]
                    .iter()
                    .take(5)
                    .filter(|ev| matches!(ev.kind, TimelineEventKind::Anomaly(_)))
                    .count();

                if failures_after >= 2 {
                    patterns.push(LearnedPattern {
                        description: "Config change followed by failures".into(),
                        kind: PatternKind::ConfigRegression,
                        occurrences: 1,
                        first_seen: e.timestamp,
                        last_seen: e.timestamp,
                        confidence: 0.7,
                        related_events: vec![e.id.clone()],
                    });
                }
            }
        }

        patterns
    }

    /// Detect the same calibration repeated within a 24-hour window.
    fn detect_calibration_loops(events: &[TimelineEvent]) -> Vec<LearnedPattern> {
        use std::collections::HashMap;

        let mut cal_by_type: HashMap<String, Vec<&TimelineEvent>> = HashMap::new();

        for e in events {
            if let TimelineEventKind::Configuration(ref ce) = e.kind {
                if matches!(
                    ce.action,
                    ConfigAction::PidTuned
                        | ConfigAction::InputShaperCalibrated
                        | ConfigAction::BedMeshGenerated
                        | ConfigAction::ProbeCalibrated
                        | ConfigAction::PressureAdvanceTuned
                ) {
                    cal_by_type
                        .entry(format!("{:?}", ce.action))
                        .or_default()
                        .push(e);
                }
            }
        }

        cal_by_type
            .into_iter()
            .filter(|(_, evs)| evs.len() >= 3)
            .filter(|(_, evs)| {
                // Must be within a short window.
                let first = evs.iter().map(|e| e.timestamp).min().unwrap();
                let last = evs.iter().map(|e| e.timestamp).max().unwrap();
                last - first < Duration::hours(48)
            })
            .map(|(cal_type, evs)| {
                let ids: Vec<String> = evs.iter().map(|e| e.id.clone()).collect();
                LearnedPattern {
                    description: format!("Calibration loop: {}", cal_type),
                    kind: PatternKind::CalibrationLoop,
                    occurrences: evs.len() as u64,
                    first_seen: evs.iter().map(|e| e.timestamp).min().unwrap(),
                    last_seen: evs.iter().map(|e| e.timestamp).max().unwrap(),
                    confidence: 0.8,
                    related_events: ids,
                }
            })
            .collect()
    }

    /// Group events that occur within 10 minutes of each other.
    fn detect_event_clusters(events: &[TimelineEvent]) -> Vec<LearnedPattern> {
        if events.len() < 3 {
            return Vec::new();
        }

        let mut patterns = Vec::new();
        let mut cluster_start = 0usize;

        for i in 1..events.len() {
            let gap = events[i].timestamp - events[i - 1].timestamp;
            if gap > Duration::minutes(10) || i == events.len() - 1 {
                let cluster_size = i - cluster_start;
                if cluster_size >= 3 {
                    let ids: Vec<String> = events[cluster_start..i]
                        .iter()
                        .map(|e| e.id.clone())
                        .collect();
                    patterns.push(LearnedPattern {
                        description: format!(
                            "Event cluster: {} events in short window",
                            cluster_size
                        ),
                        kind: PatternKind::EventCluster,
                        occurrences: cluster_size as u64,
                        first_seen: events[cluster_start].timestamp,
                        last_seen: events[i - 1].timestamp,
                        confidence: 0.6,
                        related_events: ids,
                    });
                }
                cluster_start = i;
            }
        }

        patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn hw_event(ts: chrono::DateTime<Utc>, comp: &str) -> TimelineEvent {
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: "p1".into(),
            timestamp: ts,
            kind: TimelineEventKind::Hardware(HardwareEvent {
                action: HardwareAction::Replaced,
                component_id: comp.into(),
                component_type: "test".into(),
                previous_value: None,
                new_value: None,
            }),
            source: TimelineEventSource::Automatic,
            confidence: 1.0,
            metadata: serde_json::json!({}),
        }
    }

    fn anomaly_event(ts: chrono::DateTime<Utc>, desc: &str) -> TimelineEvent {
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: "p1".into(),
            timestamp: ts,
            kind: TimelineEventKind::Anomaly(AnomalyEvent {
                severity: AnomalySeverity::Warning,
                anomaly_type: "thermal".into(),
                description: desc.into(),
                print_job_id: None,
            }),
            source: TimelineEventSource::Automatic,
            confidence: 0.9,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn detects_recurring_failure_patterns() {
        let now = Utc::now();
        let events: Vec<_> = (0..4)
            .map(|i| anomaly_event(now + chrono::Duration::hours(i as i64), "overheat"))
            .collect();
        let patterns = PatternDetector::detect(&events);
        let recurring = patterns
            .iter()
            .any(|p| matches!(p.kind, PatternKind::RecurringFailure));
        assert!(recurring);
    }

    #[test]
    fn ignores_rare_failures() {
        let now = Utc::now();
        let events = vec![
            anomaly_event(now, "once"), // single occurrence, shouldn't pattern
        ];
        let patterns = PatternDetector::detect(&events);
        assert!(patterns.is_empty());
    }

    #[test]
    fn detects_event_clusters() {
        let now = Utc::now();
        let events: Vec<_> = (0..4)
            .map(|i| {
                hw_event(
                    now + chrono::Duration::minutes(i as i64 * 2),
                    &format!("comp_{i}"),
                )
            })
            .collect();
        let patterns = PatternDetector::detect(&events);
        assert!(patterns
            .iter()
            .any(|p| matches!(p.kind, PatternKind::EventCluster)));
    }
}
