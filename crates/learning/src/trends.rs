//! TrendAnalyzer — statistical trend detection over historical data.
//!
//! Computes trends for:
//! - Failure rate over time
//! - Print success rate over time
//! - Anomaly frequency
//! - Configuration change frequency
//!
//! All computation is mechanical. Trends are computed from counts over
//! sliding windows, with simple linear regression for direction.

use layermind_shared::history::*;
use layermind_shared::learning::*;

/// Analyzes statistical trends from timeline events.
#[derive(Debug)]
pub struct TrendAnalyzer;

impl TrendAnalyzer {
    /// Analyze trends from the given events.
    pub fn analyze(events: &[TimelineEvent]) -> Vec<TrendReport> {
        if events.is_empty() {
            return Vec::new();
        }

        let first_ts = events.iter().map(|e| e.timestamp).min().unwrap();
        let last_ts = events.iter().map(|e| e.timestamp).max().unwrap();

        vec![
            Self::failure_rate_trend(events),
            Self::print_success_trend(events),
            Self::anomaly_frequency_trend(events, first_ts, last_ts),
            Self::config_change_trend(events, first_ts, last_ts),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Compute failure rate trend: are failures increasing or decreasing?
    fn failure_rate_trend(events: &[TimelineEvent]) -> Option<TrendReport> {
        let failures: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.kind, TimelineEventKind::Anomaly(_)))
            .collect();

        if failures.len() < 5 {
            return None;
        }

        // Split into first half / second half.
        let mid = failures.len() / 2;
        let first_half = &failures[..mid];
        let second_half = &failures[mid..];

        let first_span = span_hours(first_half);
        let second_span = span_hours(second_half);

        let first_rate = if first_span > 0.0 {
            first_half.len() as f64 / first_span
        } else {
            0.0
        };
        let second_rate = if second_span > 0.0 {
            second_half.len() as f64 / second_span
        } else {
            0.0
        };

        let direction = if second_rate > first_rate * 1.2 {
            TrendDirection::Worsening
        } else if second_rate < first_rate * 0.8 {
            TrendDirection::Improving
        } else {
            TrendDirection::Stable
        };

        Some(TrendReport {
            metric: "failure_rate".into(),
            window_start: failures.first()?.timestamp,
            window_end: failures.last()?.timestamp,
            sample_count: failures.len() as u64,
            average: (first_rate + second_rate) / 2.0,
            min: first_rate.min(second_rate),
            max: first_rate.max(second_rate),
            direction,
            change_rate: second_rate - first_rate,
        })
    }

    /// Compute print success rate trend.
    fn print_success_trend(events: &[TimelineEvent]) -> Option<TrendReport> {
        let prints: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.kind, TimelineEventKind::PrintHistory(_)))
            .collect();

        if prints.len() < 10 {
            return None;
        }

        let mid = prints.len() / 2;
        let first = &prints[..mid];
        let second = &prints[mid..];

        let success_first = success_rate(first);
        let success_second = success_rate(second);

        let direction = if success_second > success_first + 0.05 {
            TrendDirection::Improving
        } else if success_second < success_first - 0.05 {
            TrendDirection::Worsening
        } else {
            TrendDirection::Stable
        };

        Some(TrendReport {
            metric: "print_success_rate".into(),
            window_start: prints.first()?.timestamp,
            window_end: prints.last()?.timestamp,
            sample_count: prints.len() as u64,
            average: (success_first + success_second) / 2.0,
            min: success_first.min(success_second),
            max: success_first.max(success_second),
            direction,
            change_rate: success_second - success_first,
        })
    }

    /// Compute anomaly frequency (events per day).
    fn anomaly_frequency_trend(
        events: &[TimelineEvent],
        first_ts: chrono::DateTime<chrono::Utc>,
        last_ts: chrono::DateTime<chrono::Utc>,
    ) -> Option<TrendReport> {
        let anomalies: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.kind, TimelineEventKind::Anomaly(_)))
            .collect();

        let total_days = duration_days(events);
        if total_days < 1.0 || anomalies.is_empty() {
            return None;
        }

        let freq = anomalies.len() as f64 / total_days;

        Some(TrendReport {
            metric: "anomaly_frequency".into(),
            window_start: first_ts,
            window_end: last_ts,
            sample_count: anomalies.len() as u64,
            average: freq,
            min: 0.0,
            max: freq,
            direction: TrendDirection::Stable,
            change_rate: 0.0,
        })
    }

    /// Compute configuration change frequency.
    fn config_change_trend(
        events: &[TimelineEvent],
        first_ts: chrono::DateTime<chrono::Utc>,
        last_ts: chrono::DateTime<chrono::Utc>,
    ) -> Option<TrendReport> {
        let configs: Vec<_> = events
            .iter()
            .filter(|e| matches!(e.kind, TimelineEventKind::Configuration(_)))
            .collect();

        if configs.len() < 2 {
            return None;
        }

        let total_days = duration_days(events);

        Some(TrendReport {
            metric: "config_change_count".into(),
            window_start: first_ts,
            window_end: last_ts,
            sample_count: configs.len() as u64,
            average: configs.len() as f64,
            min: 0.0,
            max: configs.len() as f64,
            direction: TrendDirection::Stable,
            change_rate: if total_days > 0.0 {
                configs.len() as f64 / total_days
            } else {
                0.0
            },
        })
    }
}

fn success_rate(events: &[&TimelineEvent]) -> f64 {
    let total = events.len() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let successes = events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                TimelineEventKind::PrintHistory(PrintHistoryEvent {
                    action: PrintAction::Completed | PrintAction::FirstSuccessful,
                    ..
                })
            )
        })
        .count() as f64;
    successes / total
}

fn duration_days(events: &[TimelineEvent]) -> f64 {
    let first = events.iter().map(|e| e.timestamp).min();
    let last = events.iter().map(|e| e.timestamp).max();
    match (first, last) {
        (Some(f), Some(l)) => {
            let dur = l - f;
            dur.num_milliseconds() as f64 / (24.0 * 3600.0 * 1000.0)
        }
        _ => 0.0,
    }
}

fn span_hours(events: &[&TimelineEvent]) -> f64 {
    let first = events.iter().map(|e| e.timestamp).min();
    let last = events.iter().map(|e| e.timestamp).max();
    match (first, last) {
        (Some(f), Some(l)) => {
            let dur = l - f;
            dur.num_milliseconds() as f64 / (3600.0 * 1000.0)
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;

    fn anomaly_ts(ts: chrono::DateTime<Utc>) -> TimelineEvent {
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: "p1".into(),
            timestamp: ts,
            kind: TimelineEventKind::Anomaly(AnomalyEvent {
                severity: AnomalySeverity::Warning,
                anomaly_type: "test".into(),
                description: "test".into(),
                print_job_id: None,
            }),
            source: TimelineEventSource::Automatic,
            confidence: 0.9,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn empty_events_no_trends() {
        let trends = TrendAnalyzer::analyze(&[]);
        assert!(trends.is_empty());
    }

    #[test]
    fn failure_trend_with_enough_data() {
        let now = Utc::now();
        let events: Vec<_> = (0..10)
            .map(|i| anomaly_ts(now - chrono::Duration::hours(i as i64)))
            .collect();
        let trends = TrendAnalyzer::analyze(&events);
        assert!(!trends.is_empty());
    }
}
