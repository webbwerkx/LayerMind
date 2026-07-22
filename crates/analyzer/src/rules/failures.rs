//! Failure pattern detection rule.
//!
//! Flags when prints fail repeatedly with similar characteristics,
//! indicating a systemic issue rather than a one-off problem.

use layermind_shared::event::{Envelope, Event};

use super::{AnomalyCategory, Detection, Rule, Severity};

/// Consecutive failures that trigger a warning.
const CONSECUTIVE_WARNING: u32 = 2;

/// Consecutive failures that trigger a critical alert.
const CONSECUTIVE_CRITICAL: u32 = 5;

#[derive(Debug, Default)]
pub struct FailurePatternRule;

impl FailurePatternRule {
    pub fn new() -> Self {
        Self
    }
}

impl Rule for FailurePatternRule {
    fn analyze(&self, window: &[Envelope]) -> Vec<Detection> {
        // Count consecutive failures from the end of the window backwards.
        let mut consecutive = 0u32;
        let mut last_reason = None;

        for e in window.iter().rev() {
            match &e.payload {
                Event::PrintFailed { reason } => {
                    consecutive += 1;
                    if last_reason.is_none() {
                        last_reason = reason.clone();
                    }
                }
                Event::PrintCompleted { .. } | Event::PrintStarted { .. } => {
                    break; // print lifecycle reset
                }
                _ => {}
            }
        }

        if consecutive >= CONSECUTIVE_CRITICAL {
            vec![Detection {
                category: AnomalyCategory::RepeatedFailures,
                severity: Severity::Critical,
                message: format!(
                    "{} consecutive print failures detected. Last reason: {}",
                    consecutive,
                    last_reason.as_deref().unwrap_or("unknown")
                ),
                evidence: vec![format!(
                    "consecutive_failures={}, last_reason={:?}",
                    consecutive, last_reason
                )],
            }]
        } else if consecutive >= CONSECUTIVE_WARNING {
            vec![Detection {
                category: AnomalyCategory::RepeatedFailures,
                severity: Severity::Warning,
                message: format!("{} consecutive print failures detected", consecutive),
                evidence: vec![format!(
                    "consecutive_failures={}, last_reason={:?}",
                    consecutive, last_reason
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
    use uuid::Uuid;

    fn failed_envelope(reason: &str) -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            payload: Event::PrintFailed {
                reason: Some(reason.into()),
            },
        }
    }

    fn completed_envelope() -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            payload: Event::PrintCompleted {
                total_time: 100.0,
                filament_used: None,
            },
        }
    }

    #[test]
    fn single_failure_no_detection() {
        let rule = FailurePatternRule::new();
        let window = vec![failed_envelope("error")];
        assert!(rule.analyze(&window).is_empty());
    }

    #[test]
    fn three_consecutive_failures_triggers_warning() {
        let rule = FailurePatternRule::new();
        let window = vec![
            failed_envelope("e1"),
            failed_envelope("e2"),
            failed_envelope("e3"),
        ];
        let detections = rule.analyze(&window);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, Severity::Warning);
    }

    #[test]
    fn success_resets_count() {
        let rule = FailurePatternRule::new();
        let window = vec![
            failed_envelope("e1"),
            failed_envelope("e2"),
            completed_envelope(),
            failed_envelope("e3"),
        ];
        assert!(rule.analyze(&window).is_empty());
    }
}
