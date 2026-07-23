//! FailurePredictor — predicts component failures from historical
//! patterns and trend analysis.
//!
//! Every prediction is mechanically derived from explicit thresholds.
//! No AI, no heuristics, no external services. The predictor takes
//! BehaviorSummary outputs and raw timeline events to produce
//! AgingReport estimates and per-component health assessments.

use chrono::Utc;
use layermind_shared::history::*;
use layermind_shared::learning::*;

// ── Thresholds ──────────────────────────────────────────────────────

const PROBE_FAILURE_CAL_LOOP_THRESHOLD: u64 = 3;
const PROBE_FAILURE_CAL_LOOP_DAYS: i64 = 14;
const THERMAL_ANOMALY_FREQ_THRESHOLD: f64 = 2.0;
const MECHANICAL_FAILURE_RATE_WORSENING: f64 = 1.5;
const COMMS_CLUSTER_THRESHOLD: u64 = 3;
const COMMS_CLUSTER_HOURS: i64 = 24;
const DEGRADATION_WINDOW_DAYS: i64 = 30;
const MIN_EVENTS_FOR_PREDICTION: usize = 10;
const HEALTH_SCORE_DEFAULT: f64 = 1.0;
const HEALTH_DECAY_PER_ANOMALY: f64 = 0.15;
const HEALTH_DECAY_PER_WORSENING_TREND: f64 = 0.30;
const CRITICAL_HEALTH_THRESHOLD: f64 = 0.3;
const WARNING_HEALTH_THRESHOLD: f64 = 0.6;

// ── Predictor ───────────────────────────────────────────────────────

/// Predicts component failures and produces aging reports.
///
/// All predictions are threshold-based and deterministic.
#[derive(Debug)]
pub struct FailurePredictor;

impl FailurePredictor {
    /// Analyze events and produce failure predictions as AgingReports.
    pub fn predict(events: &[TimelineEvent], summary: &BehaviorSummary) -> Vec<AgingReport> {
        if events.len() < MIN_EVENTS_FOR_PREDICTION {
            return Vec::new();
        }

        let mut reports = Vec::new();

        if let Some(r) = Self::predict_probe_failure(summary) {
            reports.push(r);
        }
        if let Some(r) = Self::predict_thermal_degradation(events, summary) {
            reports.push(r);
        }
        if let Some(r) = Self::predict_mechanical_wear(summary) {
            reports.push(r);
        }
        if let Some(r) = Self::predict_comms_issues(events) {
            reports.push(r);
        }

        reports
    }

    /// Compute a health score (0.0–1.0) per component based on
    /// anomaly frequency, trend direction, and hardware age.
    pub fn component_health(
        events: &[TimelineEvent],
        summary: &BehaviorSummary,
    ) -> Vec<ComponentHealth> {
        let components = Self::collect_component_ids(events);
        let now = Utc::now();

        let failure_worsening = summary
            .trends
            .iter()
            .any(|t| t.metric == "failure_rate" && t.direction == TrendDirection::Worsening);

        let mut healths = Vec::new();

        for (comp_id, comp_type) in components {
            let change_count = events
                .iter()
                .filter(|e| {
                    matches!(&e.kind, TimelineEventKind::Hardware(ref hw) if hw.component_id == comp_id)
                        || matches!(&e.kind, TimelineEventKind::Maintenance(ref m) if m.component == comp_id)
                })
                .count() as u64;

            let mut score = HEALTH_SCORE_DEFAULT;
            score -= change_count as f64 * HEALTH_DECAY_PER_ANOMALY;

            if failure_worsening {
                score -= HEALTH_DECAY_PER_WORSENING_TREND;
            }
            let score = score.clamp(0.0, 1.0);

            let degradation_rate = summary
                .trends
                .iter()
                .find(|t| t.metric == "failure_rate")
                .map(|t| t.change_rate)
                .unwrap_or(0.0);

            let mut warnings = Vec::new();

            if score < CRITICAL_HEALTH_THRESHOLD {
                warnings.push(ComponentWarning {
                    severity: WarningSeverity::Critical,
                    message: format!("{} is in critical condition", comp_type),
                    detected_at: now,
                    evidence: format!(
                        "health_score={:.2}, {} anomalies detected",
                        score, change_count
                    ),
                });
            } else if score < WARNING_HEALTH_THRESHOLD {
                warnings.push(ComponentWarning {
                    severity: WarningSeverity::Moderate,
                    message: format!("{} shows significant degradation", comp_type),
                    detected_at: now,
                    evidence: format!(
                        "health_score={:.2}, {} anomalies detected",
                        score, change_count
                    ),
                });
            } else if change_count > 0 {
                warnings.push(ComponentWarning {
                    severity: WarningSeverity::Early,
                    message: format!("{} has minor degradation", comp_type),
                    detected_at: now,
                    evidence: format!("{} anomalies detected", change_count),
                });
            }

            healths.push(ComponentHealth {
                component_id: comp_id,
                component_type: comp_type,
                health_score: score,
                warnings,
                degradation_rate,
                anomaly_count: change_count,
                assessed_at: now,
            });
        }

        healths
    }

    // ── Private prediction rules ─────────────────────────────────

    fn predict_probe_failure(summary: &BehaviorSummary) -> Option<AgingReport> {
        let now = Utc::now();
        let has_probe_loop = summary.patterns.iter().any(|p| {
            p.kind == PatternKind::CalibrationLoop
                && (p.description.contains("BedMesh")
                    || p.description.contains("Probe")
                    || (p.occurrences >= PROBE_FAILURE_CAL_LOOP_THRESHOLD
                        && (now - p.last_seen).num_days() <= PROBE_FAILURE_CAL_LOOP_DAYS))
        });

        let failure_worsening = summary
            .trends
            .iter()
            .any(|t| t.metric == "failure_rate" && t.direction == TrendDirection::Worsening);

        if !has_probe_loop || !failure_worsening {
            return None;
        }

        Some(AgingReport {
            component_id: "predicted_probe_failure".into(),
            component_type: "probe".into(),
            installed: None,
            age_days: None,
            estimated_remaining_days: None,
            estimation_basis: "calibration_loop_plus_failure_trend".into(),
            wear_indicators: vec![
                "repeated calibration within short window".into(),
                "failure rate deteriorating".into(),
            ],
        })
    }

    fn predict_thermal_degradation(
        events: &[TimelineEvent],
        summary: &BehaviorSummary,
    ) -> Option<AgingReport> {
        let thermal_anomalies: Vec<_> = events
            .iter()
            .filter(|e| {
                if let TimelineEventKind::Anomaly(ref ae) = e.kind {
                    ae.anomaly_type == "thermal"
                        || ae.description.to_lowercase().contains("temperature")
                        || ae.description.to_lowercase().contains("overheat")
                } else {
                    false
                }
            })
            .collect();

        if thermal_anomalies.is_empty() {
            return None;
        }

        let window_days = summary
            .trends
            .iter()
            .find(|t| t.metric == "anomaly_frequency")
            .map(|t| (t.window_end - t.window_start).num_days().max(1) as f64)
            .unwrap_or(DEGRADATION_WINDOW_DAYS as f64);

        let freq = thermal_anomalies.len() as f64 / window_days;

        if freq < THERMAL_ANOMALY_FREQ_THRESHOLD {
            return None;
        }

        Some(AgingReport {
            component_id: "predicted_thermal_degradation".into(),
            component_type: "thermal".into(),
            installed: None,
            age_days: None,
            estimated_remaining_days: None,
            estimation_basis: "anomaly_frequency_analysis".into(),
            wear_indicators: vec![
                format!(
                    "thermal anomaly frequency {:.1}/day exceeds threshold {:.1}",
                    freq, THERMAL_ANOMALY_FREQ_THRESHOLD
                ),
                format!("{} thermal anomalies detected", thermal_anomalies.len()),
            ],
        })
    }

    fn predict_mechanical_wear(summary: &BehaviorSummary) -> Option<AgingReport> {
        let is_worsening = summary.trends.iter().any(|t| {
            t.metric == "failure_rate"
                && t.direction == TrendDirection::Worsening
                && t.change_rate > MECHANICAL_FAILURE_RATE_WORSENING
        });

        if !is_worsening {
            return None;
        }

        Some(AgingReport {
            component_id: "predicted_mechanical_wear".into(),
            component_type: "mechanical".into(),
            installed: None,
            age_days: None,
            estimated_remaining_days: None,
            estimation_basis: "failure_rate_trend_analysis".into(),
            wear_indicators: vec![
                "failure rate increasing faster than baseline".into(),
                format!(
                    "worsening rate exceeds {}x threshold",
                    MECHANICAL_FAILURE_RATE_WORSENING
                ),
            ],
        })
    }

    fn predict_comms_issues(events: &[TimelineEvent]) -> Option<AgingReport> {
        let comms_anomalies: Vec<_> = events
            .iter()
            .filter(|e| {
                if let TimelineEventKind::Anomaly(ref ae) = e.kind {
                    ae.anomaly_type.to_lowercase().contains("comm")
                        || ae.anomaly_type.to_lowercase().contains("connection")
                        || ae.description.to_lowercase().contains("communication")
                        || ae.description.to_lowercase().contains("comms")
                        || ae.description.to_lowercase().contains("connection")
                } else {
                    false
                }
            })
            .collect();

        if comms_anomalies.len() as u64 >= COMMS_CLUSTER_THRESHOLD {
            let first = comms_anomalies.iter().map(|e| e.timestamp).min()?;
            let last = comms_anomalies.iter().map(|e| e.timestamp).max()?;
            let span_hours = (last - first).num_hours();

            if span_hours <= COMMS_CLUSTER_HOURS {
                return Some(AgingReport {
                    component_id: "predicted_comms_issues".into(),
                    component_type: "communication".into(),
                    installed: None,
                    age_days: None,
                    estimated_remaining_days: None,
                    estimation_basis: "anomaly_cluster_analysis".into(),
                    wear_indicators: vec![format!(
                        "{} communication anomalies within {}h window",
                        comms_anomalies.len(),
                        span_hours
                    )],
                });
            }
        }

        None
    }

    // ── Helpers ──────────────────────────────────────────────────

    fn collect_component_ids(events: &[TimelineEvent]) -> Vec<(String, String)> {
        use std::collections::BTreeMap;

        let mut ids: BTreeMap<String, String> = BTreeMap::new();

        for e in events {
            match &e.kind {
                TimelineEventKind::Hardware(hw) => {
                    ids.entry(hw.component_id.clone())
                        .or_insert_with(|| hw.component_type.clone());
                }
                TimelineEventKind::Maintenance(m) => {
                    ids.entry(m.component.clone())
                        .or_insert_with(|| "maintenance".into());
                }
                TimelineEventKind::Configuration(ce) => {
                    ids.entry(ce.config_key.clone())
                        .or_insert_with(|| "config".into());
                }
                _ => {}
            }
        }

        ids.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anomaly_event(desc: &str, atype: &str) -> TimelineEvent {
        TimelineEvent {
            id: uuid::Uuid::new_v4().to_string(),
            printer_id: "p1".into(),
            timestamp: Utc::now(),
            kind: TimelineEventKind::Anomaly(AnomalyEvent {
                severity: AnomalySeverity::Warning,
                anomaly_type: atype.into(),
                description: desc.into(),
                print_job_id: None,
            }),
            source: TimelineEventSource::Automatic,
            confidence: 0.9,
            metadata: serde_json::json!({}),
        }
    }

    fn thermal_anomaly() -> TimelineEvent {
        anomaly_event("thermal runaway", "thermal")
    }

    fn dummy_summary() -> BehaviorSummary {
        BehaviorSummary::new(0)
    }

    #[test]
    fn empty_events_no_predictions() {
        let summary = dummy_summary();
        assert!(FailurePredictor::predict(&[], &summary).is_empty());
    }

    #[test]
    fn thermal_above_threshold_emits_prediction() {
        let events: Vec<_> = (0..20).map(|_| thermal_anomaly()).collect();
        let mut summary = BehaviorSummary::new(events.len() as u64);
        summary.trends = vec![TrendReport {
            metric: "anomaly_frequency".into(),
            window_start: Utc::now() - chrono::Duration::days(5),
            window_end: Utc::now(),
            sample_count: 20,
            average: 4.0,
            min: 0.0,
            max: 5.0,
            direction: TrendDirection::Stable,
            change_rate: 0.0,
        }];

        let predictions = FailurePredictor::predict(&events, &summary);
        let thermal = predictions.iter().find(|r| r.component_type == "thermal");
        assert!(thermal.is_some());
        let t = thermal.unwrap();
        assert_eq!(t.estimation_basis, "anomaly_frequency_analysis");
        assert!(!t.wear_indicators.is_empty());
    }

    #[test]
    fn mechanical_worsening_emits_prediction() {
        let events: Vec<_> = (0..15)
            .map(|i| TimelineEvent {
                id: uuid::Uuid::new_v4().to_string(),
                printer_id: "p1".into(),
                timestamp: Utc::now() - chrono::Duration::hours(i),
                kind: TimelineEventKind::Anomaly(AnomalyEvent {
                    severity: AnomalySeverity::Warning,
                    anomaly_type: "mechanical".into(),
                    description: "belt slip".into(),
                    print_job_id: None,
                }),
                source: TimelineEventSource::Automatic,
                confidence: 0.9,
                metadata: serde_json::json!({}),
            })
            .collect();

        let mut summary = BehaviorSummary::new(events.len() as u64);
        summary.trends = vec![TrendReport {
            metric: "failure_rate".into(),
            window_start: Utc::now() - chrono::Duration::days(10),
            window_end: Utc::now(),
            sample_count: 15,
            average: 1.5,
            min: 0.5,
            max: 3.0,
            direction: TrendDirection::Worsening,
            change_rate: 2.0,
        }];

        let predictions = FailurePredictor::predict(&events, &summary);
        let mech = predictions
            .iter()
            .find(|r| r.component_type == "mechanical");
        assert!(mech.is_some());
    }

    #[test]
    fn comms_cluster_emits_prediction() {
        let now = Utc::now();
        let mut events: Vec<_> = (0..5)
            .map(|i| TimelineEvent {
                id: format!("comms_{}", i),
                printer_id: "p1".into(),
                timestamp: now - chrono::Duration::hours(i),
                kind: TimelineEventKind::Anomaly(AnomalyEvent {
                    severity: AnomalySeverity::Error,
                    anomaly_type: "communication".into(),
                    description: "comms failure".into(),
                    print_job_id: None,
                }),
                source: TimelineEventSource::Automatic,
                confidence: 0.9,
                metadata: serde_json::json!({}),
            })
            .collect();

        // Fill to MIN_EVENTS_FOR_PREDICTION (10).
        for i in 0..10 {
            events.push(anomaly_event(&format!("filler_{}", i), "test"));
        }

        let predictions = FailurePredictor::predict(&events, &dummy_summary());
        let comms = predictions
            .iter()
            .find(|r| r.component_type == "communication");
        assert!(comms.is_some());
    }

    #[test]
    fn component_health_starts_at_one() {
        let summary = dummy_summary();
        let hw_events = vec![TimelineEvent {
            id: "e1".into(),
            printer_id: "p1".into(),
            timestamp: Utc::now(),
            kind: TimelineEventKind::Hardware(HardwareEvent {
                action: HardwareAction::Installed,
                component_id: "probe_0".into(),
                component_type: "probe".into(),
                previous_value: None,
                new_value: Some("BLTouch".into()),
            }),
            source: TimelineEventSource::Automatic,
            confidence: 1.0,
            metadata: serde_json::json!({}),
        }];

        let healths = FailurePredictor::component_health(&hw_events, &summary);
        assert!(!healths.is_empty());
        let probe = healths.iter().find(|h| h.component_id == "probe_0");
        assert!(probe.is_some());
        // One hardware event = one anomaly, so score = 1.0 - 0.15 = 0.85.
        assert_eq!(
            probe.unwrap().health_score,
            HEALTH_SCORE_DEFAULT - HEALTH_DECAY_PER_ANOMALY
        );
    }

    #[test]
    fn health_below_critical_generates_warning() {
        let now = Utc::now();
        let mut events = Vec::new();
        // Add enough anomalies to push health below CRITICAL_HEALTH_THRESHOLD.
        // CRITICAL = 0.3, default = 1.0, decay = 0.15 per anomaly.
        // Need (1.0 - 0.3) / 0.15 = 4.67 → 5 anomalies.
        for i in 0..6 {
            events.push(TimelineEvent {
                id: format!("e{}", i),
                printer_id: "p1".into(),
                timestamp: now - chrono::Duration::hours(i),
                kind: TimelineEventKind::Maintenance(MaintenanceEvent {
                    action: MaintenanceAction::Inspected,
                    component: "probe_0".into(),
                    notes: None,
                    scheduled: false,
                }),
                source: TimelineEventSource::Automatic,
                confidence: 1.0,
                metadata: serde_json::json!({}),
            });
        }

        let summary = dummy_summary();
        let healths = FailurePredictor::component_health(&events, &summary);
        let probe = healths
            .iter()
            .find(|h| h.component_id == "probe_0")
            .unwrap();
        assert!(probe.health_score < CRITICAL_HEALTH_THRESHOLD);
        assert!(probe
            .warnings
            .iter()
            .any(|w| w.severity == WarningSeverity::Critical));
    }
}
