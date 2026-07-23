//! PrintAnalyzer — compares print performance over time windows.
//!
//! Computes:
//! - Success rate per window
//! - Failure clustering by type
//! - Print duration trends

use chrono::Duration;
use layermind_shared::history::*;
use layermind_shared::learning::*;

/// Analyzes print success patterns.
#[derive(Debug)]
pub struct PrintAnalyzer;

impl PrintAnalyzer {
    /// Compare recent prints vs. older prints to detect changes.
    pub fn compare_recent(events: &[TimelineEvent]) -> Option<PrintComparison> {
        let prints: Vec<&TimelineEvent> = events
            .iter()
            .filter(|e| matches!(e.kind, TimelineEventKind::PrintHistory(_)))
            .collect();

        if prints.len() < 10 {
            return None;
        }

        // Split by midpoint — compare earlier half vs later half.
        let mid = prints.len() / 2;
        let earlier = &prints[..mid];
        let later = &prints[mid..];

        let before = Self::window_from(earlier);
        let after = Self::window_from(later);

        let before_rate = if before.total_prints > 0 {
            before.successful_prints as f64 / before.total_prints as f64
        } else {
            0.0
        };
        let after_rate = if after.total_prints > 0 {
            after.successful_prints as f64 / after.total_prints as f64
        } else {
            0.0
        };

        let mut changes = Vec::new();
        if (after_rate - before_rate).abs() > 0.1 {
            changes.push(format!(
                "Success rate changed from {:.0}% to {:.0}%",
                before_rate * 100.0,
                after_rate * 100.0
            ));
        }

        let before_fails = before.total_prints - before.successful_prints;
        let after_fails = after.total_prints - after.successful_prints;

        Some(PrintComparison {
            before_period: before,
            after_period: after,
            success_rate_change: after_rate - before_rate,
            avg_duration_change_secs: 0.0, // requires telemetry data
            failure_count_before: before_fails,
            failure_count_after: after_fails,
            significant_changes: changes,
        })
    }

    /// Build success windows over sliding intervals.
    pub fn success_windows(events: &[TimelineEvent], window_days: i64) -> Vec<SuccessWindow> {
        let prints: Vec<&TimelineEvent> = events
            .iter()
            .filter(|e| matches!(e.kind, TimelineEventKind::PrintHistory(_)))
            .collect();

        if prints.is_empty() {
            return Vec::new();
        }

        let first = prints.iter().map(|e| e.timestamp).min().unwrap();
        let last = prints.iter().map(|e| e.timestamp).max().unwrap();
        let total_span = (last - first).num_days();

        let mut windows = Vec::new();
        for offset in (0..=total_span).step_by(window_days as usize) {
            let window_start = first + Duration::days(offset);
            let window_end = window_start + Duration::days(window_days);

            let in_window: Vec<_> = prints
                .iter()
                .filter(|e| e.timestamp >= window_start && e.timestamp < window_end)
                .copied()
                .collect();

            if !in_window.is_empty() {
                let total = in_window.len() as u64;
                let successes = in_window
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
                    .count() as u64;
                let fails = total - successes;

                let most_common_failure = in_window
                    .iter()
                    .filter(|e| {
                        matches!(
                            e.kind,
                            TimelineEventKind::PrintHistory(PrintHistoryEvent {
                                action: PrintAction::Failed,
                                ..
                            })
                        )
                    })
                    .filter_map(|e| {
                        if let TimelineEventKind::PrintHistory(ref pe) = e.kind {
                            pe.failure_reason.clone()
                        } else {
                            None
                        }
                    })
                    .fold(std::collections::HashMap::new(), |mut acc, reason| {
                        *acc.entry(reason).or_insert(0) += 1;
                        acc
                    })
                    .into_iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|(r, _)| r);

                windows.push(SuccessWindow {
                    window_days,
                    total_prints: total,
                    success_rate: if total > 0 {
                        successes as f64 / total as f64
                    } else {
                        0.0
                    },
                    failure_rate: if total > 0 {
                        fails as f64 / total as f64
                    } else {
                        0.0
                    },
                    most_common_failure,
                });
            }
        }

        windows
    }

    fn window_from(events: &[&TimelineEvent]) -> TimeWindow {
        let total = events.len() as u64;
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
            .count() as u64;

        TimeWindow {
            start: events.iter().map(|e| e.timestamp).min().unwrap_or_default(),
            end: events.iter().map(|e| e.timestamp).max().unwrap_or_default(),
            total_prints: total,
            successful_prints: successes,
            failed_prints: total - successes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn print_event(ts: chrono::DateTime<Utc>, success: bool) -> TimelineEvent {
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: "p1".into(),
            timestamp: ts,
            kind: TimelineEventKind::PrintHistory(PrintHistoryEvent {
                action: if success {
                    PrintAction::Completed
                } else {
                    PrintAction::Failed
                },
                print_job_id: None,
                filename: None,
                duration_secs: None,
                failure_reason: None,
            }),
            source: TimelineEventSource::Automatic,
            confidence: 1.0,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn compare_recent_requires_enough_data() {
        let r = PrintAnalyzer::compare_recent(&[]);
        assert!(r.is_none());

        let now = Utc::now();
        let few: Vec<_> = (0..3)
            .map(|i| print_event(now - chrono::Duration::hours(i), i % 2 == 0))
            .collect();
        assert!(PrintAnalyzer::compare_recent(&few).is_none());
    }

    #[test]
    fn compare_recent_detects_success_change() {
        let now = Utc::now();
        let mut events: Vec<_> = (0..5)
            .map(|i| print_event(now - chrono::Duration::hours(i), false))
            .collect();
        events.extend((5..10).map(|i| print_event(now - chrono::Duration::hours(i), true)));
        let comp = PrintAnalyzer::compare_recent(&events);
        assert!(comp.is_some());
    }

    #[test]
    fn success_windows_partitions_by_time() {
        let now = Utc::now();
        let events: Vec<_> = (0..20)
            .map(|i| print_event(now - chrono::Duration::days(i), i % 3 != 0))
            .collect();
        let windows = PrintAnalyzer::success_windows(&events, 7);
        assert!(!windows.is_empty());
    }
}
