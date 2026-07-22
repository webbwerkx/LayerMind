//! Timeline builder — constructs a chronological history of important
//! printer events.
//!
//! The timeline captures human-meaningful moments: calibration runs,
//! failures, milestones, maintenance events. Each entry includes a
//! type, description, severity, and structured metadata.

use layermind_shared::knowledge::{KnowledgeKind, TimelineEntry, TimelineEventType};
use layermind_shared::observation::{Observation, ObservationKind, Severity};

/// Builds a chronological timeline for a single printer.
#[derive(Debug)]
pub struct TimelineBuilder {
    entries: Vec<TimelineEntry>,
    print_count: u64,
}

impl TimelineBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            print_count: 0,
        }
    }

    /// Process an observation. Returns knowledge records for any
    /// timeline entries that were added.
    pub fn process(&mut self, observation: &Observation) -> Vec<KnowledgeKind> {
        let entry = match &observation.kind {
            ObservationKind::PrintCompleted { .. } => {
                self.print_count += 1;
                self.milestone_check(observation)
            }

            ObservationKind::PrintFailed { reason, .. } => {
                self.print_count += 1;
                let desc = reason.as_deref().unwrap_or("unknown reason");
                Some(TimelineEntry::new(
                    TimelineEventType::FailureDetected,
                    format!("Print failed: {}", desc),
                    Some(Severity::Warning),
                    serde_json::json!({"reason": reason}),
                    observation.timestamp,
                ))
            }

            ObservationKind::AnomalyDetected {
                category,
                severity,
                message,
                ..
            } => {
                let event_type = match category {
                    _ => TimelineEventType::FailureDetected,
                };
                Some(TimelineEntry::new(
                    event_type,
                    message.clone(),
                    Some(*severity),
                    serde_json::json!({"category": format!("{:?}", category)}),
                    observation.timestamp,
                ))
            }

            _ => None,
        };

        if let Some(entry) = entry {
            let kind = KnowledgeKind::TimelineEventAdded {
                entry: entry.clone(),
            };
            self.entries.push(entry);
            vec![kind]
        } else {
            vec![]
        }
    }

    /// Add a manually recorded timeline event (e.g., from user action).
    pub fn record_manual(
        &mut self,
        event_type: TimelineEventType,
        description: String,
        severity: Option<Severity>,
        metadata: serde_json::Value,
        occurred_at: chrono::DateTime<chrono::Utc>,
    ) -> TimelineEntry {
        let entry = TimelineEntry::new(event_type, description, severity, metadata, occurred_at);
        self.entries.push(entry.clone());
        entry
    }

    fn milestone_check(&self, observation: &Observation) -> Option<TimelineEntry> {
        // Fire milestones at common print counts.
        let milestone = match self.print_count {
            1 => Some("First successful print"),
            10 => Some("10 prints completed"),
            50 => Some("50 prints completed"),
            100 => Some("100 prints completed — milestone!"),
            500 => Some("500 prints completed"),
            1000 => Some("1,000 prints completed — workhorse"),
            _ => None,
        };

        milestone.map(|desc| {
            TimelineEntry::new(
                TimelineEventType::PrintMilestone,
                desc.into(),
                Some(Severity::Info),
                serde_json::json!({"print_number": self.print_count}),
                observation.timestamp,
            )
        })
    }

    pub fn entries(&self) -> &[TimelineEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn obs(kind: ObservationKind) -> Observation {
        Observation {
            id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            kind,
        }
    }

    #[test]
    fn print_failure_adds_timeline_entry() {
        let mut builder = TimelineBuilder::new();
        let results = builder.process(&obs(ObservationKind::PrintFailed {
            filename: "test.gcode".into(),
            duration_secs: 100.0,
            reason: Some("thermal runaway".into()),
        }));

        assert_eq!(results.len(), 1);
        assert_eq!(builder.len(), 1);
        assert_eq!(
            builder.entries()[0].event_type,
            TimelineEventType::FailureDetected
        );
    }

    #[test]
    fn first_print_milestone() {
        let mut builder = TimelineBuilder::new();
        let results = builder.process(&obs(ObservationKind::PrintCompleted {
            filename: "first.gcode".into(),
            duration_secs: 100.0,
            success: true,
        }));

        assert_eq!(results.len(), 1);
        assert_eq!(
            builder.entries()[0].event_type,
            TimelineEventType::PrintMilestone
        );
    }

    #[test]
    fn anomaly_adds_timeline_entry() {
        let mut builder = TimelineBuilder::new();
        let results = builder.process(&obs(ObservationKind::AnomalyDetected {
            category: layermind_shared::observation::AnomalyCategory::TemperatureInstability,
            severity: Severity::Critical,
            message: "temp unstable".into(),
            evidence: vec![],
        }));

        assert_eq!(results.len(), 1);
    }

    #[test]
    fn health_snapshot_does_not_add_entry() {
        let mut builder = TimelineBuilder::new();
        let results = builder.process(&obs(ObservationKind::HealthSnapshot {
            temperature_stability: 1.0,
            success_rate: None,
            recent_error_count: 0,
            recent_warning_count: 0,
            seconds_since_calibration: None,
            uptime_secs: 0.0,
        }));

        assert!(results.is_empty());
    }

    #[test]
    fn manual_entry_added() {
        let mut builder = TimelineBuilder::new();
        builder.record_manual(
            TimelineEventType::NozzleChanged,
            "Replaced 0.4mm with 0.6mm".into(),
            Some(Severity::Info),
            serde_json::json!({"old": "0.4mm", "new": "0.6mm"}),
            Utc::now(),
        );
        assert_eq!(builder.len(), 1);
    }
}
