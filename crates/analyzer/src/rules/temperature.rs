//! Temperature stability detection rule.
//!
//! Flags when temperature readings show excessive oscillation around
//! the target, indicating PID tuning issues or hardware problems.

use layermind_shared::event::{Envelope, Event};

use super::{AnomalyCategory, Detection, Rule, Severity};

/// Maximum acceptable average temperature deviation (°C).
const MAX_DEVIATION: f64 = 3.0;

/// Minimum temperature events required before analyzing.
const MIN_SAMPLES: usize = 10;

#[derive(Debug)]
pub struct TemperatureStabilityRule;

impl TemperatureStabilityRule {
    pub fn new() -> Self {
        Self
    }
}

impl Rule for TemperatureStabilityRule {
    fn analyze(&self, window: &[Envelope]) -> Vec<Detection> {
        let temps: Vec<f64> = window
            .iter()
            .filter_map(|e| match &e.payload {
                Event::TemperatureUpdate { temperatures } => Some(
                    temperatures
                        .iter()
                        .map(|t| (t.current - t.target).abs())
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect();

        if temps.len() < MIN_SAMPLES {
            return vec![];
        }

        let avg_deviation: f64 = temps.iter().sum::<f64>() / temps.len() as f64;

        if avg_deviation > MAX_DEVIATION {
            vec![Detection {
                category: AnomalyCategory::TemperatureInstability,
                severity: if avg_deviation > 6.0 {
                    Severity::Critical
                } else {
                    Severity::Warning
                },
                message: format!(
                    "Temperature instability detected: average deviation {:.1}°C from target over {} readings",
                    avg_deviation,
                    temps.len()
                ),
                evidence: vec![format!(
                    "avg_deviation={:.2}°C, threshold={:.1}°C, sample_count={}",
                    avg_deviation,
                    MAX_DEVIATION,
                    temps.len()
                )],
            }]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::types::Temperature;
    use uuid::Uuid;

    fn temp_envelope(current: f64, target: f64) -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            payload: Event::TemperatureUpdate {
                temperatures: vec![Temperature {
                    sensor: "e0".into(),
                    current,
                    target,
                    power: Some(0.5),
                }],
            },
        }
    }

    #[test]
    fn stable_temps_no_detection() {
        let rule = TemperatureStabilityRule::new();
        let window: Vec<_> = (0..20).map(|_| temp_envelope(210.0, 210.0)).collect();
        assert!(rule.analyze(&window).is_empty());
    }

    #[test]
    fn unstable_temps_triggers_detection() {
        let rule = TemperatureStabilityRule::new();
        let window: Vec<_> = (0..20).map(|_| temp_envelope(215.0, 210.0)).collect();
        let detections = rule.analyze(&window);
        assert_eq!(detections.len(), 1);
        assert_eq!(
            detections[0].category,
            AnomalyCategory::TemperatureInstability
        );
    }

    #[test]
    fn too_few_samples_no_detection() {
        let rule = TemperatureStabilityRule::new();
        let window: Vec<_> = (0..5).map(|_| temp_envelope(220.0, 210.0)).collect();
        assert!(rule.analyze(&window).is_empty());
    }
}
