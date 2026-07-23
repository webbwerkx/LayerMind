//! OptimizationEngine — derives actionable recommendations from
//! patterns, trends, component health, and machine capabilities.
//!
//! All suggestions are deterministic and evidence-backed. Every
//! suggestion requires human approval before execution. The engine
//! never makes autonomous changes.

use chrono::Utc;
use layermind_shared::learning::*;
use layermind_shared::machine::MachineProfile;

const IS_RECALIBRATION_DAYS: i64 = 30;
const FULL_CALIBRATION_DAYS: i64 = 14;
const FULL_CALIBRATION_UNHEALTHY_THRESHOLD: usize = 3;
const PID_TUNE_TEMP_THRESHOLD: f64 = 250.0;
const HEALTH_WARNING_THRESHOLD: f64 = 0.7;
const HEALTH_MODERATE_THRESHOLD: f64 = 0.6;

/// Derives tuning, calibration, and maintenance recommendations.
#[derive(Debug)]
pub struct OptimizationEngine;

impl OptimizationEngine {
    /// Analyze learning outputs + machine profile → optimization report.
    pub fn analyze(
        summary: &BehaviorSummary,
        machine: Option<&MachineProfile>,
    ) -> OptimizationReport {
        let mut suggestions = Vec::new();

        if let Some(profile) = machine {
            if let Some(s) = Self::pressure_advance_retune(profile, summary) {
                suggestions.push(s);
            }
            if let Some(s) = Self::input_shaping_recalibrate(profile, summary) {
                suggestions.push(s);
            }
            if let Some(s) = Self::pid_tune_needed(profile, summary) {
                suggestions.push(s);
            }
        }

        if let Some(s) = Self::bed_mesh_needed(summary) {
            suggestions.push(s);
        }

        let maintenance = Self::maintenance_from_health(summary);
        let calibration_plan = Self::full_calibration_plan(summary, machine.is_some());

        let total = (suggestions.len() + maintenance.len()) as u64
            + if calibration_plan.is_some() { 1 } else { 0 };

        OptimizationReport {
            suggestions,
            calibration_plan,
            maintenance_actions: maintenance,
            generated_at: Utc::now(),
            total_suggestions: total,
        }
    }

    // ── Rule 1: PA retune ──────────────────────────────────────

    fn pressure_advance_retune(
        profile: &MachineProfile,
        summary: &BehaviorSummary,
    ) -> Option<TuningSuggestion> {
        if !profile.capabilities.supports_pressure_advance.value {
            return None;
        }

        let unhealthy: Vec<_> = summary
            .component_health
            .iter()
            .filter(|ch| ch.health_score < HEALTH_WARNING_THRESHOLD)
            .collect();
        if unhealthy.is_empty() {
            return None;
        }

        let pa_loop = summary.patterns.iter().any(|p| {
            matches!(p.kind, PatternKind::CalibrationLoop)
                && p.description.to_lowercase().contains("pressureadvance")
        });

        let pa_recent = summary
            .calibration
            .as_ref()
            .and_then(|c| c.last_calibration)
            .map(|ts| (Utc::now() - ts).num_days() < 14)
            .unwrap_or(false);

        if !pa_loop && !pa_recent {
            return None;
        }

        let mut facts = Vec::new();
        for ch in &unhealthy {
            facts.push(format!(
                "{}: health={:.2}",
                ch.component_type, ch.health_score
            ));
        }

        Some(TuningSuggestion {
            category: TuningCategory::PressureAdvance,
            parameter: "pressure_advance".into(),
            current_value: None,
            suggested_value: "retune".into(),
            evidence: SuggestionEvidence {
                trigger: "pressure_advance_degradation".into(),
                supporting_facts: facts,
                pattern_references: vec![],
            },
            expected_benefit: "Improved extrusion consistency and reduced artifacts".into(),
            risk: RiskLevel::Low,
        })
    }

    // ── Rule 2: IS recalibration ───────────────────────────────

    fn input_shaping_recalibrate(
        profile: &MachineProfile,
        summary: &BehaviorSummary,
    ) -> Option<TuningSuggestion> {
        if !profile.capabilities.supports_input_shaping.value {
            return None;
        }

        let accel_unhealthy = summary.component_health.iter().any(|ch| {
            ch.component_type.to_lowercase().contains("accelerometer")
                && ch.health_score < HEALTH_WARNING_THRESHOLD
        });

        let old_calibration = summary
            .calibration
            .as_ref()
            .and_then(|c| c.last_calibration)
            .map(|ts| (Utc::now() - ts).num_days() > IS_RECALIBRATION_DAYS)
            .unwrap_or(true);

        let accel_aging = summary
            .aging
            .iter()
            .any(|a| a.component_type.to_lowercase().contains("accel"));

        if !accel_unhealthy && !old_calibration && !accel_aging {
            return None;
        }

        let mut facts = Vec::new();
        if accel_unhealthy {
            facts.push("accelerometer health declining".into());
        }
        if old_calibration {
            facts.push(format!(
                "last input shaper calibration >{} days ago",
                IS_RECALIBRATION_DAYS
            ));
        }
        if accel_aging {
            facts.push("accelerometer aging detected".into());
        }

        Some(TuningSuggestion {
            category: TuningCategory::InputShaping,
            parameter: "input_shaper".into(),
            current_value: None,
            suggested_value: "recalibrate".into(),
            evidence: SuggestionEvidence {
                trigger: "input_shaping_recalibration".into(),
                supporting_facts: facts,
                pattern_references: vec![],
            },
            expected_benefit: "Better input shaping resonance compensation".into(),
            risk: RiskLevel::Low,
        })
    }

    // ── Rule 3: PID tune ───────────────────────────────────────

    fn pid_tune_needed(
        profile: &MachineProfile,
        summary: &BehaviorSummary,
    ) -> Option<TuningSuggestion> {
        let hotend_unhealthy = summary.component_health.iter().any(|ch| {
            ch.component_type.to_lowercase().contains("hotend")
                && ch.health_score < HEALTH_MODERATE_THRESHOLD
        });

        let thermal_aging = summary.aging.iter().any(|a| a.component_type == "thermal");

        if !hotend_unhealthy && !thermal_aging {
            return None;
        }

        if profile.capabilities.maximum_temperature.value <= PID_TUNE_TEMP_THRESHOLD {
            return None;
        }

        let mut facts = Vec::new();
        if hotend_unhealthy {
            let h = summary
                .component_health
                .iter()
                .find(|ch| {
                    ch.component_type.to_lowercase().contains("hotend")
                        && ch.health_score < HEALTH_MODERATE_THRESHOLD
                })
                .unwrap();
            facts.push(format!(
                "hotend health={:.2}, {} anomalies",
                h.health_score, h.anomaly_count
            ));
        }
        if thermal_aging {
            let a = summary
                .aging
                .iter()
                .find(|a| a.component_type == "thermal")
                .unwrap();
            for wi in &a.wear_indicators {
                facts.push(wi.clone());
            }
        }

        Some(TuningSuggestion {
            category: TuningCategory::PidTune,
            parameter: "pid_tune".into(),
            current_value: None,
            suggested_value: "pid_tune".into(),
            evidence: SuggestionEvidence {
                trigger: "thermal_instability".into(),
                supporting_facts: facts,
                pattern_references: vec![],
            },
            expected_benefit: "Stabilized hotend temperature and reduced thermal oscillation"
                .into(),
            risk: RiskLevel::Medium,
        })
    }

    // ── Rule 4: Bed mesh ───────────────────────────────────────

    fn bed_mesh_needed(summary: &BehaviorSummary) -> Option<TuningSuggestion> {
        let overdue = summary
            .calibration
            .as_ref()
            .map(|c| c.overdue.contains(&"BedMeshGenerated".to_string()))
            .unwrap_or(false);

        let probe_unhealthy = summary.component_health.iter().any(|ch| {
            ch.component_type.to_lowercase().contains("probe")
                && ch.health_score < HEALTH_WARNING_THRESHOLD
        });

        if !overdue && !probe_unhealthy {
            return None;
        }

        let trigger = if overdue {
            "bed_mesh_overdue"
        } else {
            "probe_health_declining"
        };

        let mut facts = Vec::new();
        if overdue {
            facts.push("bed mesh calibration overdue (>2x avg interval)".into());
        }
        if probe_unhealthy {
            let p = summary
                .component_health
                .iter()
                .find(|ch| {
                    ch.component_type.to_lowercase().contains("probe")
                        && ch.health_score < HEALTH_WARNING_THRESHOLD
                })
                .unwrap();
            facts.push(format!("probe health={:.2}", p.health_score));
        }

        Some(TuningSuggestion {
            category: TuningCategory::BedMesh,
            parameter: "bed_mesh".into(),
            current_value: None,
            suggested_value: "recalibrate".into(),
            evidence: SuggestionEvidence {
                trigger: trigger.into(),
                supporting_facts: facts,
                pattern_references: vec![],
            },
            expected_benefit: "Improved first-layer adhesion and Z accuracy".into(),
            risk: RiskLevel::Low,
        })
    }

    // ── Rule 5: Full calibration plan ──────────────────────────

    fn full_calibration_plan(
        summary: &BehaviorSummary,
        caps_known: bool,
    ) -> Option<CalibrationPlan> {
        let success_worsening = summary
            .trends
            .iter()
            .any(|t| t.metric == "print_success_rate" && t.direction == TrendDirection::Worsening);

        let no_recent_cal = summary
            .calibration
            .as_ref()
            .and_then(|c| c.last_calibration)
            .map(|ts| (Utc::now() - ts).num_days() > FULL_CALIBRATION_DAYS)
            .unwrap_or(true);

        let unhealthy_count = summary
            .component_health
            .iter()
            .filter(|ch| ch.health_score < HEALTH_MODERATE_THRESHOLD)
            .count();

        if !success_worsening || !no_recent_cal {
            return None;
        }
        if unhealthy_count < FULL_CALIBRATION_UNHEALTHY_THRESHOLD {
            return None;
        }

        let has_critical = summary
            .component_health
            .iter()
            .any(|ch| ch.health_score < 0.3);

        let steps = vec![
            TuningSuggestion {
                category: TuningCategory::PidTune,
                parameter: "pid_tune".into(),
                current_value: None,
                suggested_value: "retune".into(),
                evidence: SuggestionEvidence {
                    trigger: "full_calibration_sequence".into(),
                    supporting_facts: vec![],
                    pattern_references: vec![],
                },
                expected_benefit: "Thermal stability baseline".into(),
                risk: RiskLevel::Low,
            },
            TuningSuggestion {
                category: TuningCategory::BedMesh,
                parameter: "bed_mesh".into(),
                current_value: None,
                suggested_value: "recalibrate".into(),
                evidence: SuggestionEvidence {
                    trigger: "full_calibration_sequence".into(),
                    supporting_facts: vec![],
                    pattern_references: vec![],
                },
                expected_benefit: "Flat first layer baseline".into(),
                risk: RiskLevel::Low,
            },
        ];

        let urgency = if has_critical {
            PlanUrgency::Urgent
        } else {
            PlanUrgency::Recommended
        };

        Some(CalibrationPlan {
            steps,
            estimated_duration_minutes: if caps_known { 60 } else { 45 },
            urgency,
            generated_at: Utc::now(),
        })
    }

    // ── Rule 6: Maintenance from health ────────────────────────

    fn maintenance_from_health(summary: &BehaviorSummary) -> Vec<TuningSuggestion> {
        summary
            .component_health
            .iter()
            .filter(|ch| !ch.warnings.is_empty())
            .map(|ch| TuningSuggestion {
                category: TuningCategory::Maintenance,
                parameter: ch.component_id.clone(),
                current_value: Some(format!("health={:.2}", ch.health_score)),
                suggested_value: "inspect and service".into(),
                evidence: SuggestionEvidence {
                    trigger: ch
                        .warnings
                        .first()
                        .map(|w| w.message.clone())
                        .unwrap_or_default(),
                    supporting_facts: ch.warnings.iter().map(|w| w.evidence.clone()).collect(),
                    pattern_references: vec![],
                },
                expected_benefit: "Restore component health and prevent failure".into(),
                risk: Self::risk_for_health(ch.health_score),
            })
            .collect()
    }

    // ── Helpers ─────────────────────────────────────────────────

    fn risk_for_health(health_score: f64) -> RiskLevel {
        if health_score < 0.3 {
            RiskLevel::High
        } else if health_score < HEALTH_MODERATE_THRESHOLD {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_shared::machine::{
        CapabilitySet, MachineHardware, MachineIdentity, MachineType, Property,
    };

    fn test_machine() -> MachineProfile {
        MachineProfile {
            identity: MachineIdentity {
                printer_id: "p1".into(),
                nickname: None,
                manufacturer: None,
                model: None,
                custom_build: Property::observed(false),
                machine_type: Property::observed(MachineType::Cartesian),
                serial_number: None,
                firmware: None,
                discovered_at: Utc::now(),
                updated_at: Utc::now(),
            },
            hardware: MachineHardware::default(),
            capabilities: CapabilitySet::empty(),
            generated_at: Utc::now(),
        }
    }

    fn pa_capable_machine() -> MachineProfile {
        let mut m = test_machine();
        m.capabilities.supports_pressure_advance = Property::observed(true);
        m.capabilities.supports_input_shaping = Property::observed(true);
        m.capabilities.maximum_temperature = Property::observed(300.0);
        m
    }

    fn unhealthy_summary(component_type: &str, score: f64) -> BehaviorSummary {
        let mut s = BehaviorSummary::new(50);
        s.component_health = vec![ComponentHealth {
            component_id: format!("{}_0", component_type),
            component_type: component_type.into(),
            health_score: score,
            warnings: vec![ComponentWarning {
                severity: if score < 0.3 {
                    WarningSeverity::Critical
                } else if score < 0.6 {
                    WarningSeverity::Moderate
                } else {
                    WarningSeverity::Early
                },
                message: format!("{} degraded", component_type),
                detected_at: Utc::now(),
                evidence: "test".into(),
            }],
            degradation_rate: 0.1,
            anomaly_count: 5,
            assessed_at: Utc::now(),
        }];
        s
    }

    #[test]
    fn empty_summary_produces_empty_report() {
        let report = OptimizationEngine::analyze(&BehaviorSummary::new(0), None);
        assert_eq!(report.total_suggestions, 0);
        assert!(report.suggestions.is_empty());
        assert!(report.maintenance_actions.is_empty());
        assert!(report.calibration_plan.is_none());
    }

    #[test]
    fn no_machine_only_maintenance_and_calibration() {
        let summary = unhealthy_summary("probe", 0.5);
        let report = OptimizationEngine::analyze(&summary, None);
        assert!(!report.maintenance_actions.is_empty());
        // Bed mesh is suggested because probe health is below threshold.
        assert!(report
            .suggestions
            .iter()
            .any(|s| matches!(s.category, TuningCategory::BedMesh)));
    }

    #[test]
    fn bed_mesh_suggestion_from_overdue() {
        let mut summary = unhealthy_summary("probe", 0.5);
        summary.calibration = Some(CalibrationSummary {
            total_calibrations: 10,
            by_type: vec![],
            avg_interval_days: Some(7.0),
            most_frequent: None,
            last_calibration: Some(Utc::now() - chrono::Duration::days(30)),
            overdue: vec!["BedMeshGenerated".into()],
        });

        let report = OptimizationEngine::analyze(&summary, None);
        assert!(report
            .suggestions
            .iter()
            .any(|s| matches!(s.category, TuningCategory::BedMesh)));
    }

    #[test]
    fn pa_capable_with_degradation_suggests_retune() {
        let machine = pa_capable_machine();
        let mut summary = unhealthy_summary("probe", 0.5);
        summary.patterns = vec![LearnedPattern {
            description: "CalibrationLoop: PressureAdvanceTuned".into(),
            kind: PatternKind::CalibrationLoop,
            occurrences: 4,
            first_seen: Utc::now() - chrono::Duration::days(2),
            last_seen: Utc::now(),
            confidence: 0.8,
            related_events: vec![],
        }];

        let report = OptimizationEngine::analyze(&summary, Some(&machine));
        let pa = report
            .suggestions
            .iter()
            .find(|s| matches!(s.category, TuningCategory::PressureAdvance));
        assert!(pa.is_some());
        assert_eq!(pa.unwrap().risk, RiskLevel::Low);
    }

    #[test]
    fn input_shaping_recalibrate_when_old() {
        let machine = pa_capable_machine();
        let mut summary = unhealthy_summary("accelerometer", 0.5);
        summary.calibration = Some(CalibrationSummary {
            total_calibrations: 5,
            by_type: vec![],
            avg_interval_days: Some(30.0),
            most_frequent: None,
            last_calibration: Some(Utc::now() - chrono::Duration::days(60)),
            overdue: vec![],
        });

        let report = OptimizationEngine::analyze(&summary, Some(&machine));
        let is = report
            .suggestions
            .iter()
            .find(|s| matches!(s.category, TuningCategory::InputShaping));
        assert!(is.is_some());
    }

    #[test]
    fn pid_tune_with_thermal_degradation() {
        let machine = pa_capable_machine();
        let mut summary = unhealthy_summary("hotend", 0.4);
        summary.aging = vec![AgingReport {
            component_id: "thermal_pred".into(),
            component_type: "thermal".into(),
            installed: None,
            age_days: None,
            estimated_remaining_days: None,
            estimation_basis: "anomaly_frequency_analysis".into(),
            wear_indicators: vec!["thermal frequency exceeds 2.0/day".into()],
        }];

        let report = OptimizationEngine::analyze(&summary, Some(&machine));
        let pid = report
            .suggestions
            .iter()
            .find(|s| matches!(s.category, TuningCategory::PidTune));
        assert!(pid.is_some());
        assert_eq!(pid.unwrap().risk, RiskLevel::Medium);
    }

    #[test]
    fn full_calibration_when_many_unhealthy() {
        let mut summary = BehaviorSummary::new(100);
        summary.trends = vec![TrendReport {
            metric: "print_success_rate".into(),
            window_start: Utc::now() - chrono::Duration::days(14),
            window_end: Utc::now(),
            sample_count: 50,
            average: 0.7,
            min: 0.6,
            max: 0.8,
            direction: TrendDirection::Worsening,
            change_rate: -0.15,
        }];
        summary.component_health = (0..4)
            .map(|i| {
                let score = if i == 0 { 0.2 } else { 0.4 };
                let severity = if score < 0.3 {
                    WarningSeverity::Critical
                } else {
                    WarningSeverity::Moderate
                };
                ComponentHealth {
                    component_id: format!("comp_{}", i),
                    component_type: format!("comp_{}", i),
                    health_score: score,
                    warnings: vec![ComponentWarning {
                        severity,
                        message: "degraded".into(),
                        detected_at: Utc::now(),
                        evidence: "test".into(),
                    }],
                    degradation_rate: 0.1,
                    anomaly_count: 5,
                    assessed_at: Utc::now(),
                }
            })
            .collect();
        summary.calibration = Some(CalibrationSummary {
            total_calibrations: 5,
            by_type: vec![],
            avg_interval_days: Some(14.0),
            most_frequent: None,
            last_calibration: Some(Utc::now() - chrono::Duration::days(30)),
            overdue: vec![],
        });

        let report = OptimizationEngine::analyze(&summary, None);
        assert!(report.calibration_plan.is_some());
        let plan = report.calibration_plan.as_ref().unwrap();
        assert!(matches!(plan.urgency, PlanUrgency::Urgent));
        assert!(plan.steps.len() >= 2);
    }

    #[test]
    fn maintenance_actions_from_warnings() {
        let summary = unhealthy_summary("probe", 0.5);
        let report = OptimizationEngine::analyze(&summary, None);
        assert!(!report.maintenance_actions.is_empty());
        assert_eq!(
            report.maintenance_actions[0].category,
            TuningCategory::Maintenance
        );
    }

    #[test]
    fn risk_scales_with_health() {
        assert_eq!(OptimizationEngine::risk_for_health(0.1), RiskLevel::High);
        assert_eq!(OptimizationEngine::risk_for_health(0.4), RiskLevel::Medium);
        assert_eq!(OptimizationEngine::risk_for_health(0.8), RiskLevel::Low);
    }
}
