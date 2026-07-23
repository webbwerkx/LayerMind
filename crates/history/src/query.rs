//! TimelineQueryEngine — strongly-typed filtering of timeline events.
//!
//! All queries are deterministic. No AI, no fuzzy matching, no
//! heuristics. The engine filters by exact criteria: printer, category,
//! component, time range, event kind.

use chrono::{DateTime, Utc};
use layermind_shared::history::*;

/// Filters timeline events against a [`TimelineQuery`].
///
/// Only events matching all specified criteria are returned. An empty
/// query returns all events.
#[derive(Debug)]
pub struct TimelineQueryEngine;

impl TimelineQueryEngine {
    /// Execute a query against a slice of events.
    pub fn query<'a>(events: &'a [TimelineEvent], query: &TimelineQuery) -> Vec<&'a TimelineEvent> {
        let mut results: Vec<&TimelineEvent> =
            events.iter().filter(|e| Self::matches(e, query)).collect();

        // Sort by timestamp descending (most recent first).
        results.sort_by_key(|e| std::cmp::Reverse(e.timestamp));

        // Apply pagination.
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        if offset > 0 {
            results = results.into_iter().skip(offset).collect();
        }
        results.truncate(limit);

        results
    }

    /// Check if a single event matches the query.
    fn matches(event: &TimelineEvent, query: &TimelineQuery) -> bool {
        // Printer filter.
        if let Some(ref pid) = query.printer_id {
            if event.printer_id != *pid {
                return false;
            }
        }

        // Category filter.
        if let Some(ref cats) = query.categories {
            let event_cat = TimelineCategory::from(&event.kind);
            if !cats.contains(&event_cat) {
                return false;
            }
        }

        // Component filter (applies to hardware, maintenance events).
        if let Some(ref comp_id) = query.component_id {
            let matches = match &event.kind {
                TimelineEventKind::Hardware(hw) => hw.component_id == *comp_id,
                TimelineEventKind::Maintenance(m) => m.component == *comp_id,
                _ => false,
            };
            if !matches {
                return false;
            }
        }

        // Time range filter.
        if let Some(from) = query.from {
            if event.timestamp < from {
                return false;
            }
        }
        if let Some(to) = query.to {
            if event.timestamp > to {
                return false;
            }
        }

        true
    }

    /// Find the last event of a specific category for a printer.
    pub fn last_event_by_category<'a>(
        events: &'a [TimelineEvent],
        printer_id: &str,
        category: TimelineCategory,
    ) -> Option<&'a TimelineEvent> {
        let query = TimelineQuery {
            printer_id: Some(printer_id.into()),
            categories: Some(vec![category]),
            limit: Some(1),
            ..Default::default()
        };
        Self::query(events, &query).into_iter().next()
    }

    /// Find all events between two timestamps for a printer.
    pub fn in_time_range<'a>(
        events: &'a [TimelineEvent],
        printer_id: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<&'a TimelineEvent> {
        let query = TimelineQuery {
            printer_id: Some(printer_id.into()),
            from: Some(from),
            to: Some(to),
            ..Default::default()
        };
        Self::query(events, &query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn event(printer: &str, ts: DateTime<Utc>, cat: TimelineCategory) -> TimelineEvent {
        let kind = match cat {
            TimelineCategory::Hardware => TimelineEventKind::Hardware(HardwareEvent {
                action: HardwareAction::Installed,
                component_id: "probe_0".into(),
                component_type: "probe".into(),
                previous_value: None,
                new_value: None,
            }),
            TimelineCategory::Anomaly => TimelineEventKind::Anomaly(AnomalyEvent {
                severity: AnomalySeverity::Warning,
                anomaly_type: "test".into(),
                description: "test".into(),
                print_job_id: None,
            }),
            TimelineCategory::Firmware => TimelineEventKind::Firmware(FirmwareEvent {
                action: FirmwareAction::Updated,
                component: "klipper".into(),
                previous_version: None,
                new_version: Some("v1.0".into()),
            }),
            _ => TimelineEventKind::SnapshotCreated(SnapshotKind::Machine),
        };
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: printer.into(),
            timestamp: ts,
            kind,
            source: TimelineEventSource::Automatic,
            confidence: 1.0,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn filters_by_printer() {
        let now = Utc::now();
        let events = vec![
            event("p1", now - Duration::hours(2), TimelineCategory::Hardware),
            event("p1", now - Duration::hours(1), TimelineCategory::Anomaly),
            event("p2", now, TimelineCategory::Hardware),
        ];

        let q = TimelineQuery {
            printer_id: Some("p1".into()),
            ..Default::default()
        };
        let results = TimelineQueryEngine::query(&events, &q);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn filters_by_category() {
        let now = Utc::now();
        let events = vec![
            event("p1", now - Duration::hours(2), TimelineCategory::Hardware),
            event("p1", now - Duration::hours(1), TimelineCategory::Anomaly),
            event("p1", now, TimelineCategory::Hardware),
        ];

        let q = TimelineQuery {
            printer_id: Some("p1".into()),
            categories: Some(vec![TimelineCategory::Anomaly]),
            ..Default::default()
        };
        let results = TimelineQueryEngine::query(&events, &q);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].kind, TimelineEventKind::Anomaly(_)));
    }

    #[test]
    fn filters_by_time_range() {
        let base = Utc::now();
        let events = vec![
            event("p1", base - Duration::days(30), TimelineCategory::Hardware),
            event("p1", base - Duration::days(10), TimelineCategory::Anomaly),
            event("p1", base - Duration::days(1), TimelineCategory::Hardware),
        ];

        let results =
            TimelineQueryEngine::in_time_range(&events, "p1", base - Duration::days(7), base);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn pagination() {
        let base = Utc::now();
        let events: Vec<_> = (0..10)
            .map(|i| event("p1", base - Duration::hours(i), TimelineCategory::Anomaly))
            .collect();

        let q = TimelineQuery {
            printer_id: Some("p1".into()),
            limit: Some(3),
            offset: Some(2),
            ..Default::default()
        };
        let results = TimelineQueryEngine::query(&events, &q);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn last_event_by_category_finds_most_recent() {
        let base = Utc::now();
        let events = vec![
            event("p1", base - Duration::days(10), TimelineCategory::Firmware),
            event("p1", base - Duration::days(5), TimelineCategory::Firmware),
            event("p1", base, TimelineCategory::Anomaly),
        ];

        let last_fw =
            TimelineQueryEngine::last_event_by_category(&events, "p1", TimelineCategory::Firmware);
        assert!(last_fw.is_some());
        // The most recent firmware event should be the one from 5 days ago.
        let ts = last_fw.unwrap().timestamp;
        assert!(ts > base - Duration::days(6));
    }
}
