//! TimelineStore — the core append-only event log.
//!
//! Events are inserted and never modified. The store maintains an
//! in-memory index for fast queries and can be backed by a database
//! for persistence.

use chrono::Utc;
use layermind_shared::history::*;
use std::collections::HashMap;
use uuid::Uuid;

/// The primary event store — append-only, indexed in memory.
///
/// In production this is backed by PostgreSQL (via the database crate).
/// The in-memory index is rebuilt on startup.
#[derive(Debug, Default)]
pub struct TimelineStore {
    events: Vec<TimelineEvent>,
    /// Index: printer_id → list of event indices (for fast lookup).
    index_by_printer: HashMap<String, Vec<usize>>,
    /// Index: category → list of event indices.
    index_by_category: HashMap<TimelineCategory, Vec<usize>>,
}

impl TimelineStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new event. Returns the event with its assigned ID.
    pub fn record(
        &mut self,
        printer_id: &str,
        kind: TimelineEventKind,
        source: TimelineEventSource,
        confidence: f64,
        metadata: serde_json::Value,
    ) -> &TimelineEvent {
        let event = TimelineEvent {
            id: Uuid::new_v4().to_string(),
            printer_id: printer_id.to_string(),
            timestamp: Utc::now(),
            kind,
            source,
            confidence,
            metadata,
        };
        let idx = self.events.len();
        self.events.push(event);

        // Update indices.
        let ev = &self.events[idx];
        self.index_by_printer
            .entry(ev.printer_id.clone())
            .or_default()
            .push(idx);

        let cat = TimelineCategory::from(&ev.kind);
        self.index_by_category.entry(cat).or_default().push(idx);

        &self.events[idx]
    }

    /// Record a batch of pre-constructed events (e.g. from database
    /// reload on startup).
    pub fn load(&mut self, events: Vec<TimelineEvent>) {
        for event in events {
            let idx = self.events.len();
            let printer_id = event.printer_id.clone();
            let cat = TimelineCategory::from(&event.kind);
            self.events.push(event);
            self.index_by_printer
                .entry(printer_id)
                .or_default()
                .push(idx);
            self.index_by_category.entry(cat).or_default().push(idx);
        }
    }

    /// Return all events for a printer, most recent first.
    pub fn for_printer(&self, printer_id: &str) -> Vec<&TimelineEvent> {
        self.index_by_printer
            .get(printer_id)
            .map(|indices| {
                let mut evs: Vec<&TimelineEvent> =
                    indices.iter().map(|&i| &self.events[i]).collect();
                evs.sort_by_key(|e| std::cmp::Reverse(e.timestamp));
                evs
            })
            .unwrap_or_default()
    }

    /// Return total event count.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Return true if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Return all events (for full-scope queries).
    pub fn all(&self) -> &[TimelineEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_anomaly(printer: &str, desc: &str) -> TimelineEventKind {
        TimelineEventKind::Anomaly(AnomalyEvent {
            severity: AnomalySeverity::Warning,
            anomaly_type: "test".into(),
            description: desc.into(),
            print_job_id: None,
        })
    }

    fn make_hw(printer: &str, component: &str, action: HardwareAction) -> TimelineEventKind {
        TimelineEventKind::Hardware(HardwareEvent {
            action,
            component_id: component.into(),
            component_type: "probe".into(),
            previous_value: None,
            new_value: Some("installed".into()),
        })
    }

    #[test]
    fn records_and_retrieves_events() {
        let mut store = TimelineStore::new();
        store.record(
            "p1",
            make_anomaly("p1", "thermal warning"),
            TimelineEventSource::Automatic,
            0.9,
            serde_json::json!({}),
        );
        store.record(
            "p1",
            make_hw("p1", "probe_0", HardwareAction::Installed),
            TimelineEventSource::User,
            1.0,
            serde_json::json!({"user": "curtis"}),
        );
        store.record(
            "p2",
            make_anomaly("p2", "comms failure"),
            TimelineEventSource::Automatic,
            0.8,
            serde_json::json!({}),
        );

        assert_eq!(store.len(), 3);

        let p1_events = store.for_printer("p1");
        assert_eq!(p1_events.len(), 2);
        // Most recent first.
        assert!(matches!(p1_events[0].kind, TimelineEventKind::Hardware(_)));
    }

    #[test]
    fn load_rebuilds_indices() {
        let events = vec![TimelineEvent {
            id: "e1".into(),
            printer_id: "p1".into(),
            timestamp: Utc::now(),
            kind: make_anomaly("p1", "test"),
            source: TimelineEventSource::Automatic,
            confidence: 0.9,
            metadata: serde_json::json!({}),
        }];

        let mut store = TimelineStore::new();
        store.load(events);
        assert_eq!(store.for_printer("p1").len(), 1);
    }

    #[test]
    fn empty_store_returns_empty() {
        let store = TimelineStore::new();
        assert!(store.for_printer("nonexistent").is_empty());
        assert!(store.is_empty());
    }

    #[test]
    fn events_indexed_by_category() {
        let mut store = TimelineStore::new();
        store.record(
            "p1",
            make_hw("p1", "bed", HardwareAction::Replaced),
            TimelineEventSource::User,
            1.0,
            serde_json::json!({}),
        );
        store.record(
            "p1",
            make_anomaly("p1", "error"),
            TimelineEventSource::Automatic,
            0.9,
            serde_json::json!({}),
        );

        let hw_count = store
            .index_by_category
            .get(&TimelineCategory::Hardware)
            .map(|v| v.len())
            .unwrap_or(0);
        let anom_count = store
            .index_by_category
            .get(&TimelineCategory::Anomaly)
            .map(|v| v.len())
            .unwrap_or(0);

        assert_eq!(hw_count, 1);
        assert_eq!(anom_count, 1);
    }
}
