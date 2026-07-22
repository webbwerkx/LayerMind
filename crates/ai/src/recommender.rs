//! Recommendation engine — generates actionable suggestions from detections.
//!
//! Maps detected conditions to user-facing recommendations with
//! confidence scores and supporting evidence.

use crate::detector::{Detection, DetectionCategory};

/// A user-facing recommendation.
#[derive(Debug, Clone)]
pub struct Recommendation {
    pub printer_id: String,
    pub category: String,
    pub message: String,
    pub confidence: f64,
    pub action_items: Vec<String>,
}

/// Generate recommendations from a batch of detections.
pub fn recommend(detections: &[Detection]) -> Vec<Recommendation> {
    detections
        .iter()
        .map(|d| match &d.category {
            DetectionCategory::TemperatureInstability => Recommendation {
                printer_id: d.printer_id.clone(),
                category: "thermal".into(),
                message:
                    "Your heater PID may need tuning. Temperature is oscillating significantly."
                        .into(),
                confidence: 0.75,
                action_items: vec![
                    "Run PID_CALIBRATE HEATER=extruder TARGET=220".into(),
                    "Run PID_CALIBRATE HEATER=heater_bed TARGET=60".into(),
                    "Save with SAVE_CONFIG".into(),
                ],
            },
            DetectionCategory::RingingGhosting => Recommendation {
                printer_id: d.printer_id.clone(),
                category: "mechanical".into(),
                message: "Your acceleration appears too aggressive for this machine.".into(),
                confidence: 0.7,
                action_items: vec![
                    "Reduce max acceleration by 20%".into(),
                    "Check belt tension".into(),
                    "Consider input shaping calibration".into(),
                ],
            },
            DetectionCategory::FirstLayerFailure => Recommendation {
                printer_id: d.printer_id.clone(),
                category: "calibration".into(),
                message: "Your Z offset may be incorrect. Repeated first layer failures detected."
                    .into(),
                confidence: 0.65,
                action_items: vec![
                    "Run PROBE_CALIBRATE".into(),
                    "Adjust Z offset in smaller increments".into(),
                    "Clean and re-level bed".into(),
                ],
            },
            _ => Recommendation {
                printer_id: d.printer_id.clone(),
                category: "general".into(),
                message: "An issue was detected. Review the printer logs for more detail.".into(),
                confidence: 0.5,
                action_items: vec!["Review recent print history".into()],
            },
        })
        .collect()
}
