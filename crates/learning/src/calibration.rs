//! CalibrationTracker — analyzes calibration history for a printer.
//!
//! Tracks:
//! - Calibration type frequency
//! - Intervals between calibrations
//! - Overdue calibrations
//! - Calibration timing patterns

use chrono::Duration;
use layermind_shared::history::*;
use layermind_shared::learning::*;

/// Analyzes calibration patterns from timeline events.
#[derive(Debug)]
pub struct CalibrationTracker;

impl CalibrationTracker {
    /// Produce a calibration summary from timeline events.
    pub fn summarize(events: &[TimelineEvent]) -> Option<CalibrationSummary> {
        let cals: Vec<&TimelineEvent> = events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    TimelineEventKind::Configuration(ConfigurationEvent {
                        action: ConfigAction::PidTuned
                            | ConfigAction::InputShaperCalibrated
                            | ConfigAction::BedMeshGenerated
                            | ConfigAction::ProbeCalibrated
                            | ConfigAction::PressureAdvanceTuned,
                        ..
                    })
                )
            })
            .collect();

        if cals.is_empty() {
            return None;
        }

        // Count by action type.
        use std::collections::HashMap;
        let mut counts: HashMap<String, u64> = HashMap::new();
        let mut latest: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();

        for e in &cals {
            if let TimelineEventKind::Configuration(ref ce) = e.kind {
                let key = format!("{:?}", ce.action);
                *counts.entry(key.clone()).or_insert(0) += 1;
                latest
                    .entry(key)
                    .and_modify(|t| {
                        if e.timestamp > *t {
                            *t = e.timestamp;
                        }
                    })
                    .or_insert(e.timestamp);
            }
        }

        let total = cals.len() as u64;
        let by_type: Vec<(String, u64)> = {
            let mut v: Vec<_> = counts.into_iter().collect();
            v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
            v
        };

        // Average interval: total time span / number of calibrations.
        let first_ts = cals.iter().map(|e| e.timestamp).min()?;
        let last_ts = cals.iter().map(|e| e.timestamp).max()?;
        let span_days = (last_ts - first_ts).num_days() as f64;
        let avg_interval = if cals.len() > 1 {
            Some(span_days / (cals.len() - 1) as f64)
        } else {
            None
        };

        let most_frequent = by_type.first().map(|(t, _)| t.clone());
        let last_calibration = cals.iter().map(|e| e.timestamp).max();

        // Overdue: any calibration type whose last occurrence is >2x the
        // average interval from now.
        let now = chrono::Utc::now();
        let mut overdue = Vec::new();
        if let Some(interval) = avg_interval {
            for (cal_type, last) in &latest {
                let days_since = (now - *last).num_days() as f64;
                if days_since > interval * 2.0 {
                    overdue.push(cal_type.clone());
                }
            }
        }

        Some(CalibrationSummary {
            total_calibrations: total,
            by_type,
            avg_interval_days: avg_interval,
            most_frequent,
            last_calibration,
            overdue,
        })
    }

    /// Find the most recent calibration of a specific type.
    pub fn last_of_type(events: &[TimelineEvent], action: ConfigAction) -> Option<&TimelineEvent> {
        events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    TimelineEventKind::Configuration(ConfigurationEvent {
                        action: a,
                        ..
                    }) if a == action
                )
            })
            .max_by_key(|e| e.timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn cal_event(ts: chrono::DateTime<Utc>, action: ConfigAction) -> TimelineEvent {
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: "p1".into(),
            timestamp: ts,
            kind: TimelineEventKind::Configuration(ConfigurationEvent {
                action,
                config_key: "test".into(),
                previous_value: None,
                new_value: None,
            }),
            source: TimelineEventSource::Automatic,
            confidence: 1.0,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn no_calibrations_returns_none() {
        assert!(CalibrationTracker::summarize(&[]).is_none());
    }

    #[test]
    fn summarizes_multiple_calibrations() {
        let now = Utc::now();
        let events = vec![
            cal_event(now - chrono::Duration::days(30), ConfigAction::PidTuned),
            cal_event(
                now - chrono::Duration::days(20),
                ConfigAction::BedMeshGenerated,
            ),
            cal_event(now - chrono::Duration::days(10), ConfigAction::PidTuned),
            cal_event(
                now - chrono::Duration::days(5),
                ConfigAction::InputShaperCalibrated,
            ),
        ];
        let summary = CalibrationTracker::summarize(&events);
        assert!(summary.is_some());
        let s = summary.unwrap();
        assert_eq!(s.total_calibrations, 4);
        assert!(s.by_type.len() > 1);
        assert!(s.most_frequent.is_some());
    }

    #[test]
    fn last_of_type_finds_most_recent() {
        let now = Utc::now();
        let events = vec![
            cal_event(now - chrono::Duration::days(10), ConfigAction::PidTuned),
            cal_event(now - chrono::Duration::days(2), ConfigAction::PidTuned),
        ];
        let last = CalibrationTracker::last_of_type(&events, ConfigAction::PidTuned);
        assert!(last.is_some());
        assert!(last.unwrap().timestamp > now - chrono::Duration::days(5));
    }
}
