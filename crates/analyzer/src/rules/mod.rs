//! Detection rules — deterministic analysis of event windows.
//!
//! Each rule examines a window of recent events and returns detections
//! when conditions are met. Rules are stateless functions; the
//! DetectionEngine manages state across invocations.

use layermind_shared::event::Envelope;
use layermind_shared::observation::{AnomalyCategory, Severity};

mod calibration;
mod errors;
mod failures;
mod temperature;

/// A detection produced by a rule.
#[derive(Debug, Clone)]
pub struct Detection {
    pub category: AnomalyCategory,
    pub severity: Severity,
    pub message: String,
    pub evidence: Vec<String>,
}

/// A rule analyzes an event window and returns zero or more detections.
trait Rule {
    fn analyze(&self, window: &[Envelope]) -> Vec<Detection>;
}

/// Coordinates all detection rules and manages state.
#[derive(Debug)]
pub struct DetectionEngine {
    temp_rule: temperature::TemperatureStabilityRule,
    error_rule: errors::ErrorFrequencyRule,
    failure_rule: failures::FailurePatternRule,
    cal_rule: calibration::CalibrationStalenessRule,
}

impl Default for DetectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DetectionEngine {
    pub fn new() -> Self {
        Self {
            temp_rule: temperature::TemperatureStabilityRule::new(),
            error_rule: errors::ErrorFrequencyRule::new(),
            failure_rule: failures::FailurePatternRule::new(),
            cal_rule: calibration::CalibrationStalenessRule::new(),
        }
    }

    /// Run all rules against the current event window.
    pub fn analyze(&mut self, window: &[Envelope]) -> Vec<Detection> {
        let mut detections = Vec::new();
        detections.extend(self.temp_rule.analyze(window));
        detections.extend(self.error_rule.analyze(window));
        detections.extend(self.failure_rule.analyze(window));
        detections.extend(self.cal_rule.analyze(window));
        detections
    }
}
