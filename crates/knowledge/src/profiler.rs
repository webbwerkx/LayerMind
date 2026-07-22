//! Printer profile builder.
//!
//! Aggregates observations into a persistent PrinterProfile. The
//! profile evolves over time — successful prints improve reliability
//! score, repeated anomalies become known issues, and print history
//! builds behavioral patterns.

use chrono::{DateTime, Utc};
use layermind_shared::knowledge::{KnownIssue, PrinterProfile};
use layermind_shared::observation::{Observation, ObservationKind};

/// Builds and maintains a PrinterProfile from observations.
#[derive(Debug)]
pub struct PrinterProfiler {
    profile: PrinterProfile,
}

impl PrinterProfiler {
    pub fn new(printer_id: String) -> Self {
        Self {
            profile: PrinterProfile::new(printer_id),
        }
    }

    /// Feed an observation into the profile. Returns true if the
    /// profile was meaningfully updated.
    pub fn process(&mut self, observation: &Observation) -> bool {
        let mut updated = false;

        match &observation.kind {
            ObservationKind::PrintSummary {
                success,
                duration_secs,
                ..
            } => {
                if *success {
                    self.profile.behavior.successful_prints += 1;
                } else {
                    self.profile.behavior.failed_prints += 1;
                }
                self.update_avg_duration(*duration_secs);
                updated = true;
            }

            ObservationKind::PrintCompleted { duration_secs, .. } => {
                self.profile.behavior.successful_prints += 1;
                self.update_avg_duration(*duration_secs);
                updated = true;
            }

            ObservationKind::PrintFailed {
                duration_secs,
                reason,
                ..
            } => {
                self.profile.behavior.failed_prints += 1;
                self.update_avg_duration(*duration_secs);
                updated = true;
            }

            ObservationKind::AnomalyDetected {
                category, message, ..
            } => {
                self.upsert_known_issue(*category, message.clone(), observation.timestamp);
                self.recompute_reliability();
                updated = true;
            }

            ObservationKind::HealthSnapshot {
                temperature_stability,
                success_rate,
                ..
            } => {
                let reliability = self.compute_reliability(*temperature_stability, *success_rate);
                self.profile.behavior.reliability_score = Some(reliability);
                updated = true;
            }

            _ => {}
        }

        if updated {
            self.profile.updated_at = Utc::now();
        }

        updated
    }

    /// Return the current profile snapshot.
    pub fn profile(&self) -> &PrinterProfile {
        &self.profile
    }

    /// Update hardware information.
    pub fn set_hardware(&mut self, model: Option<String>, firmware: Option<String>) {
        self.profile.hardware.model = model;
        self.profile.hardware.firmware = firmware;
        self.profile.updated_at = Utc::now();
    }

    /// Mark a known issue as resolved.
    pub fn resolve_issue(
        &mut self,
        category: layermind_shared::observation::AnomalyCategory,
    ) -> bool {
        let found = if let Some(issue) = self
            .profile
            .behavior
            .known_issues
            .iter_mut()
            .find(|i| i.category == category && !i.resolved)
        {
            issue.resolved = true;
            true
        } else {
            false
        };
        if found {
            self.recompute_reliability();
            self.profile.updated_at = Utc::now();
        }
        found
    }

    fn update_avg_duration(&mut self, duration_secs: f64) {
        let total_prints =
            self.profile.behavior.successful_prints + self.profile.behavior.failed_prints;

        if total_prints == 0 {
            return;
        }

        let current_avg = self
            .profile
            .behavior
            .avg_print_duration_secs
            .unwrap_or(duration_secs);

        // Exponential moving average (weight recent prints more).
        let alpha = 0.3;
        let new_avg = alpha * duration_secs + (1.0 - alpha) * current_avg;

        self.profile.behavior.avg_print_duration_secs = Some(new_avg);
    }

    fn upsert_known_issue(
        &mut self,
        category: layermind_shared::observation::AnomalyCategory,
        description: String,
        timestamp: DateTime<Utc>,
    ) {
        if let Some(issue) = self
            .profile
            .behavior
            .known_issues
            .iter_mut()
            .find(|i| i.category == category && !i.resolved)
        {
            issue.bump(timestamp);
            issue.description = description;
        } else {
            self.profile.behavior.known_issues.push(KnownIssue::new(
                category,
                description,
                timestamp,
            ));
        }
    }

    /// Composite reliability score: 0.0 (unreliable) to 1.0 (perfect).
    fn compute_reliability(&self, temperature_stability: f64, success_rate: Option<f64>) -> f64 {
        let temp_score = temperature_stability;
        let success_score = success_rate.unwrap_or(1.0);

        // Weighted average: 40% temp stability, 60% success rate.
        let score = 0.4 * temp_score + 0.6 * success_score;

        // Penalize known open issues.
        let open_issues = self
            .profile
            .behavior
            .known_issues
            .iter()
            .filter(|i| !i.resolved)
            .count() as f64;

        let issue_penalty = (open_issues * 0.05).min(0.5);

        (score - issue_penalty).max(0.0)
    }

    /// Recompute reliability from current profile state (no new
    /// HealthSnapshot needed). Uses the last-known values.
    fn recompute_reliability(&mut self) {
        let current = self.profile.behavior.reliability_score.unwrap_or(1.0);
        // Approximate: keep the temp/success component but apply
        // updated issue penalty.
        let open_issues = self
            .profile
            .behavior
            .known_issues
            .iter()
            .filter(|i| !i.resolved)
            .count() as f64;
        let issue_penalty = (open_issues * 0.05).min(0.5);
        let base = (current + issue_penalty).min(1.0);
        self.profile.behavior.reliability_score = Some((base - issue_penalty).max(0.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::observation::{AnomalyCategory, Severity};
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
    fn successful_prints_update_counts() {
        let mut profiler = PrinterProfiler::new("p1".into());

        profiler.process(&obs(ObservationKind::PrintCompleted {
            filename: "ok.gcode".into(),
            duration_secs: 1000.0,
            success: true,
        }));

        assert_eq!(profiler.profile().behavior.successful_prints, 1);
        assert_eq!(profiler.profile().behavior.failed_prints, 0);
        assert!(
            profiler
                .profile()
                .behavior
                .avg_print_duration_secs
                .is_some()
        );
    }

    #[test]
    fn anomaly_adds_known_issue() {
        let mut profiler = PrinterProfiler::new("p1".into());

        profiler.process(&obs(ObservationKind::AnomalyDetected {
            category: AnomalyCategory::TemperatureInstability,
            severity: Severity::Warning,
            message: "unstable temps".into(),
            evidence: vec![],
        }));

        assert_eq!(profiler.profile().behavior.known_issues.len(), 1);
        assert_eq!(
            profiler.profile().behavior.known_issues[0].occurrence_count,
            1
        );
    }

    #[test]
    fn repeated_anomaly_bumps_count() {
        let mut profiler = PrinterProfiler::new("p1".into());

        let anomaly = ObservationKind::AnomalyDetected {
            category: AnomalyCategory::RepeatedFailures,
            severity: Severity::Warning,
            message: "failures".into(),
            evidence: vec![],
        };

        profiler.process(&obs(anomaly.clone()));
        profiler.process(&obs(anomaly));

        assert_eq!(
            profiler.profile().behavior.known_issues[0].occurrence_count,
            2
        );
    }

    #[test]
    fn resolve_issue_marks_resolved() {
        let mut profiler = PrinterProfiler::new("p1".into());

        profiler.process(&obs(ObservationKind::AnomalyDetected {
            category: AnomalyCategory::CalibrationOverdue,
            severity: Severity::Warning,
            message: "stale cal".into(),
            evidence: vec![],
        }));

        assert!(profiler.resolve_issue(AnomalyCategory::CalibrationOverdue));
        assert!(profiler.profile().behavior.known_issues[0].resolved);
    }

    #[test]
    fn reliability_score_penalizes_open_issues() {
        let mut profiler = PrinterProfiler::new("p1".into());

        // Perfect health snapshot.
        profiler.process(&obs(ObservationKind::HealthSnapshot {
            temperature_stability: 1.0,
            success_rate: Some(1.0),
            recent_error_count: 0,
            recent_warning_count: 0,
            seconds_since_calibration: None,
            uptime_secs: 3600.0,
        }));

        let perfect = profiler.profile().behavior.reliability_score.unwrap();
        assert!((perfect - 1.0).abs() < 0.01);

        // Add an unresolved issue.
        profiler.process(&obs(ObservationKind::AnomalyDetected {
            category: AnomalyCategory::ExcessiveErrors,
            severity: Severity::Critical,
            message: "many errors".into(),
            evidence: vec![],
        }));

        let degraded = profiler.profile().behavior.reliability_score.unwrap();
        assert!(degraded < perfect);
    }
}
