//! Event detection — identifies interesting patterns in telemetry streams.
//!
//! Examples:
//! - Temperature instability → PID tuning recommendation
//! - Repeated ringing → acceleration too high
//! - First layer failures → Z offset may be incorrect

use layermind_shared::event::{Envelope, Event};

/// A detected condition that may warrant an AI analysis.
#[derive(Debug, Clone)]
pub struct Detection {
    pub printer_id: String,
    pub category: DetectionCategory,
    pub severity: Severity,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DetectionCategory {
    TemperatureInstability,
    RingingGhosting,
    FirstLayerFailure,
    LayerShift,
    ExtrusionIssue,
    BedAdhesion,
    MechanicalAnomaly,
    CalibrationNeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// Analyze an envelope for detectable patterns.
pub fn detect(envelope: &Envelope) -> Vec<Detection> {
    match &envelope.payload {
        Event::TemperatureUpdate { temperatures } => {
            detect_temperature_issues(envelope, temperatures)
        }
        Event::PrintFailed { reason } => detect_failure_patterns(envelope, reason),
        _ => vec![],
    }
}

fn detect_temperature_issues(
    envelope: &Envelope,
    temps: &[layermind_shared::types::Temperature],
) -> Vec<Detection> {
    let mut detections = Vec::new();

    for t in temps {
        let delta = (t.current - t.target).abs();
        if delta > 5.0 && t.power.unwrap_or(0.0) > 90.0 {
            detections.push(Detection {
                printer_id: envelope.printer_id.clone(),
                category: DetectionCategory::TemperatureInstability,
                severity: Severity::Warning,
                evidence: vec![format!(
                    "{} oscillating: target={} actual={} delta={}",
                    t.sensor, t.target, t.current, delta
                )],
            });
        }
    }

    detections
}

fn detect_failure_patterns(envelope: &Envelope, _reason: &Option<String>) -> Vec<Detection> {
    vec![]
}
