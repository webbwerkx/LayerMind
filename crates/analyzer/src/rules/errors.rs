//! Error frequency detection rule.
//!
//! Flags when error or warning events exceed normal rates, indicating
//! a degraded printer state.

use layermind_shared::event::{Envelope, Event};

use super::{AnomalyCategory, Detection, Rule, Severity};

/// Error events per window that trigger a warning.
const WARNING_THRESHOLD: usize = 5;

/// Error events per window that trigger a critical alert.
const CRITICAL_THRESHOLD: usize = 15;

/// Window size in events (look at last N events).
const WINDOW_SIZE: usize = 100;

#[derive(Debug)]
pub struct ErrorFrequencyRule;

impl ErrorFrequencyRule {
    pub fn new() -> Self {
        Self
    }
}

impl Rule for ErrorFrequencyRule {
    fn analyze(&self, window: &[Envelope]) -> Vec<Detection> {
        let recent: Vec<&Envelope> = window.iter().rev().take(WINDOW_SIZE).collect();

        let error_count = recent
            .iter()
            .filter(|e| matches!(e.payload, Event::Error { .. }))
            .count();

        let warning_count = recent
            .iter()
            .filter(|e| matches!(e.payload, Event::Warning { .. }))
            .count();

        let total_issues = error_count + warning_count;

        if total_issues >= CRITICAL_THRESHOLD {
            vec![Detection {
                category: AnomalyCategory::ExcessiveErrors,
                severity: Severity::Critical,
                message: format!(
                    "Excessive errors: {} errors and {} warnings in last {} events",
                    error_count, warning_count, WINDOW_SIZE
                ),
                evidence: vec![format!(
                    "error_count={}, warning_count={}, window_size={}",
                    error_count, warning_count, WINDOW_SIZE
                )],
            }]
        } else if total_issues >= WARNING_THRESHOLD {
            vec![Detection {
                category: AnomalyCategory::ExcessiveErrors,
                severity: Severity::Warning,
                message: format!(
                    "Elevated error rate: {} errors and {} warnings in last {} events",
                    error_count, warning_count, WINDOW_SIZE
                ),
                evidence: vec![format!(
                    "error_count={}, warning_count={}, window_size={}",
                    error_count, warning_count, WINDOW_SIZE
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

    fn error_envelope() -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            payload: Event::Error {
                code: None,
                message: "test error".into(),
            },
        }
    }

    fn temp_envelope() -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            payload: Event::TemperatureUpdate {
                temperatures: vec![],
            },
        }
    }

    #[test]
    fn low_error_count_no_detection() {
        let rule = ErrorFrequencyRule::new();
        let mut window: Vec<_> = (0..95).map(|_| temp_envelope()).collect();
        window.extend((0..3).map(|_| error_envelope()));
        assert!(rule.analyze(&window).is_empty());
    }

    #[test]
    fn moderate_errors_triggers_warning() {
        let rule = ErrorFrequencyRule::new();
        let mut window: Vec<_> = (0..90).map(|_| temp_envelope()).collect();
        window.extend((0..8).map(|_| error_envelope()));
        let detections = rule.analyze(&window);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, Severity::Warning);
    }

    #[test]
    fn many_errors_triggers_critical() {
        let rule = ErrorFrequencyRule::new();
        let mut window: Vec<_> = (0..80).map(|_| temp_envelope()).collect();
        window.extend((0..20).map(|_| error_envelope()));
        let detections = rule.analyze(&window);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, Severity::Critical);
    }
}
