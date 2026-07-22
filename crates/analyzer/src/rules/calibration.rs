//! Calibration staleness detection rule.
//!
//! Flags when a printer hasn't been calibrated recently. Long periods
//! without calibration increase the risk of degraded print quality.

use chrono::Utc;
use layermind_shared::event::{Envelope, Event};

use super::{AnomalyCategory, Detection, Rule, Severity};

/// Seconds since last calibration that trigger a warning (7 days).
const STALE_WARNING_SECS: i64 = 7 * 24 * 3600;

/// Seconds since last calibration that trigger a critical alert (30 days).
const STALE_CRITICAL_SECS: i64 = 30 * 24 * 3600;

#[derive(Debug)]
pub struct CalibrationStalenessRule;

impl CalibrationStalenessRule {
    pub fn new() -> Self {
        Self
    }
}

impl Rule for CalibrationStalenessRule {
    fn analyze(&self, window: &[Envelope]) -> Vec<Detection> {
        // Find the most recent event that indicates a calibration.
        let last_calibration = window
            .iter()
            .rev()
            .find(|e| is_calibration_event(&e.payload))
            .map(|e| e.timestamp);

        let elapsed = match last_calibration {
            Some(ts) => (Utc::now() - ts).num_seconds(),
            None => return vec![], // no calibration events ever seen
        };

        if elapsed > STALE_CRITICAL_SECS {
            let days = elapsed / 86400;
            vec![Detection {
                category: AnomalyCategory::CalibrationOverdue,
                severity: Severity::Critical,
                message: format!(
                    "Calibration is {} days overdue. Last calibration was {} days ago.",
                    days - 30,
                    days
                ),
                evidence: vec![format!("seconds_since_calibration={}", elapsed)],
            }]
        } else if elapsed > STALE_WARNING_SECS {
            let days = elapsed / 86400;
            vec![Detection {
                category: AnomalyCategory::CalibrationOverdue,
                severity: Severity::Warning,
                message: format!(
                    "Printer hasn't been calibrated in {} days. Consider running calibration.",
                    days
                ),
                evidence: vec![format!("seconds_since_calibration={}", elapsed)],
            }]
        } else {
            vec![]
        }
    }
}

fn is_calibration_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Raw {
            namespace,
            key,
            ..
        } if namespace == "moonraker" && key.as_deref() == Some("notify_gcode_response")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn calibration_event(age_secs: i64) -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now() - chrono::Duration::seconds(age_secs),
            payload: Event::Raw {
                namespace: "moonraker".into(),
                key: Some("notify_gcode_response".into()),
                value: serde_json::json!({}),
            },
        }
    }

    #[test]
    fn recent_calibration_no_detection() {
        let rule = CalibrationStalenessRule::new();
        let window = vec![calibration_event(3600)]; // 1 hour ago
        assert!(rule.analyze(&window).is_empty());
    }

    #[test]
    fn stale_calibration_triggers_warning() {
        let rule = CalibrationStalenessRule::new();
        let window = vec![calibration_event(STALE_WARNING_SECS + 3600)];
        let detections = rule.analyze(&window);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, Severity::Warning);
    }

    #[test]
    fn very_stale_calibration_triggers_critical() {
        let rule = CalibrationStalenessRule::new();
        let window = vec![calibration_event(STALE_CRITICAL_SECS + 3600)];
        let detections = rule.analyze(&window);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, Severity::Critical);
    }
}
