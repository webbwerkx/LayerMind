//! Observation lifecycle tracker.
//!
//! Maintains the set of active observations for a printer and manages
//! transitions between states: Active → Acknowledged → Resolved.

use chrono::{DateTime, Utc};
use layermind_shared::knowledge::{KnowledgeKind, ObservationState};
use layermind_shared::observation::{Observation, ObservationKind};
use uuid::Uuid;

use crate::scoring::KnowledgeScorer;

/// Tracks observations for a single printer.
#[derive(Debug)]
pub struct ObservationTracker {
    entries: Vec<TrackedObservation>,
    scorer: KnowledgeScorer,
}

/// A single observation being tracked.
#[derive(Debug, Clone)]
pub struct TrackedObservation {
    pub id: Uuid,
    pub observation_id: Uuid,
    pub category: String,
    pub severity: String,
    pub importance: f64,
    pub confidence: f64,
    pub state: ObservationState,
    pub resolution: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    /// How many times this same type of observation has appeared.
    pub repeat_count: u64,
}

impl ObservationTracker {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scorer: KnowledgeScorer::new(),
        }
    }

    /// Record a new observation. Returns a knowledge kind for the
    /// tracked observation.
    pub fn record(&mut self, observation: &Observation) -> Option<KnowledgeKind> {
        let category = category_name(&observation.kind);
        let severity = severity_name(&observation.kind);

        // Count repeats of the same category.
        let repeat_count = self
            .entries
            .iter()
            .filter(|e| e.category == category)
            .count() as u64
            + 1;

        let importance = self.scorer.importance(&observation.kind, repeat_count);
        let confidence = self.scorer.confidence(&observation.kind);

        let tracked = TrackedObservation {
            id: Uuid::now_v7(),
            observation_id: observation.id,
            category,
            severity,
            importance,
            confidence,
            state: ObservationState::Active,
            resolution: None,
            created_at: observation.timestamp,
            resolved_at: None,
            repeat_count,
        };

        self.entries.push(tracked);

        Some(KnowledgeKind::ObservationTracked {
            observation_id: observation.id,
            importance,
            confidence,
        })
    }

    /// Resolve an observation by its ID.
    pub fn resolve(&mut self, observation_id: Uuid, resolution: String) -> Option<KnowledgeKind> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.observation_id == observation_id)?;
        entry.state = ObservationState::Resolved;
        entry.resolution = Some(resolution.clone());
        entry.resolved_at = Some(Utc::now());

        Some(KnowledgeKind::ObservationResolved {
            observation_id,
            resolution,
        })
    }

    /// Acknowledge an observation (transition from Active → Acknowledged).
    pub fn acknowledge(&mut self, observation_id: Uuid) -> bool {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.observation_id == observation_id)
        {
            entry.state = ObservationState::Acknowledged;
            true
        } else {
            false
        }
    }

    pub fn active_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.state == ObservationState::Active)
            .count()
    }

    pub fn resolved_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.state == ObservationState::Resolved)
            .count()
    }

    pub fn entries(&self) -> &[TrackedObservation] {
        &self.entries
    }

    pub fn has_active_anomaly(&self, category: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.category == category && e.state == ObservationState::Active)
    }
}

fn category_name(kind: &ObservationKind) -> String {
    match kind {
        ObservationKind::AnomalyDetected { category, .. } => {
            format!("anomaly_{:?}", category).to_lowercase()
        }
        ObservationKind::PrintFailed { .. } => "print_failed".into(),
        ObservationKind::PrintCompleted { .. } => "print_completed".into(),
        ObservationKind::PrintStarted { .. } => "print_started".into(),
        ObservationKind::PrintSummary { .. } => "print_summary".into(),
        ObservationKind::HealthSnapshot { .. } => "health_snapshot".into(),
    }
}

fn severity_name(kind: &ObservationKind) -> String {
    match kind {
        ObservationKind::AnomalyDetected { severity, .. } => {
            format!("{:?}", severity).to_lowercase()
        }
        ObservationKind::PrintFailed { .. } => "warning".into(),
        _ => "info".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::observation::AnomalyCategory;

    fn anomaly_obs(severity: layermind_shared::observation::Severity) -> Observation {
        Observation {
            id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            kind: ObservationKind::AnomalyDetected {
                category: AnomalyCategory::TemperatureInstability,
                severity,
                message: "test".into(),
                evidence: vec!["e1".into()],
            },
        }
    }

    #[test]
    fn record_adds_entry() {
        let mut tracker = ObservationTracker::new();
        let obs = anomaly_obs(layermind_shared::observation::Severity::Warning);
        let tracked = tracker.record(&obs);
        assert!(tracked.is_some());
        assert_eq!(tracker.entries().len(), 1);
        assert_eq!(tracker.active_count(), 1);
    }

    #[test]
    fn resolve_transitions_state() {
        let mut tracker = ObservationTracker::new();
        let obs = anomaly_obs(layermind_shared::observation::Severity::Critical);
        tracker.record(&obs);

        let result = tracker.resolve(obs.id, "fixed".into());
        assert!(result.is_some());
        assert_eq!(tracker.active_count(), 0);
        assert_eq!(tracker.resolved_count(), 1);
    }

    #[test]
    fn acknowledge_transitions_state() {
        let mut tracker = ObservationTracker::new();
        let obs = anomaly_obs(layermind_shared::observation::Severity::Info);
        tracker.record(&obs);

        assert!(tracker.acknowledge(obs.id));
        let entry = &tracker.entries()[0];
        assert_eq!(entry.state, ObservationState::Acknowledged);
    }

    #[test]
    fn repeat_count_increases_for_same_category() {
        let mut tracker = ObservationTracker::new();

        let obs1 = anomaly_obs(layermind_shared::observation::Severity::Warning);
        tracker.record(&obs1);
        assert_eq!(tracker.entries()[0].repeat_count, 1);

        let obs2 = anomaly_obs(layermind_shared::observation::Severity::Warning);
        tracker.record(&obs2);
        assert_eq!(tracker.entries()[1].repeat_count, 2);
    }
}
