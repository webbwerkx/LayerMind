//! Health metric calculations.
//!
//! Accumulates statistics from raw events and computes derived health
//! indicators. Designed to be lightweight — O(1) per event, O(1) snapshot.

use layermind_shared::event::{Envelope, Event};

/// Rolling health metrics for a single printer.
#[derive(Debug, Default)]
pub struct HealthMetrics {
    /// Count of events processed.
    event_count: u64,
    /// Count of TemperatureUpdate events.
    temp_update_count: u64,
    /// Rolling average of temperature deviation (|current - target|).
    temp_deviation_sum: f64,
    /// Count of Error events.
    error_count: u64,
    /// Count of Warning events.
    warning_count: u64,
    /// Count of completed prints.
    completed_count: u64,
    /// Count of failed prints.
    failed_count: u64,
    /// Connection count (times connected).
    connect_count: u64,
    /// Rolling average of print durations.
    print_duration_sum: f64,
    print_duration_count: u64,
}

impl HealthMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a single event into the metrics accumulator.
    pub fn process(&mut self, envelope: &Envelope) {
        self.event_count += 1;

        match &envelope.payload {
            Event::TemperatureUpdate { temperatures } => {
                self.temp_update_count += 1;
                for t in temperatures {
                    self.temp_deviation_sum += (t.current - t.target).abs();
                }
            }
            Event::Error { .. } => {
                self.error_count += 1;
            }
            Event::Warning { .. } => {
                self.warning_count += 1;
            }
            Event::PrintCompleted { total_time, .. } => {
                self.completed_count += 1;
                self.print_duration_sum += total_time;
                self.print_duration_count += 1;
            }
            Event::PrintFailed { .. } => {
                self.failed_count += 1;
            }
            _ => {}
        }
    }

    /// Record a printer connection event.
    pub fn record_connect(&mut self) {
        self.connect_count += 1;
    }

    /// Temperature stability score: 1.0 = perfect, 0.0 = very unstable.
    ///
    /// Based on average absolute deviation from target. At 0°C deviation
    /// the score is 1.0. At 5°C average deviation, score drops to ~0.5.
    /// At 10°C+, score approaches 0.
    pub fn temperature_stability(&self) -> f64 {
        if self.temp_update_count == 0 {
            return 1.0;
        }
        let avg_deviation = self.temp_deviation_sum / self.temp_update_count as f64;
        // Exponential decay: e^(-deviation / scale)
        (-avg_deviation / 3.0).exp().clamp(0.0, 1.0)
    }

    /// Print success rate: successful / (successful + failed).
    /// Returns None if no prints have completed.
    pub fn success_rate(&self) -> Option<f64> {
        let total = self.completed_count + self.failed_count;
        if total == 0 {
            return None;
        }
        Some(self.completed_count as f64 / total as f64)
    }

    pub fn error_count(&self) -> u64 {
        self.error_count
    }

    pub fn warning_count(&self) -> u64 {
        self.warning_count
    }

    pub fn avg_print_duration(&self) -> Option<f64> {
        if self.print_duration_count == 0 {
            return None;
        }
        Some(self.print_duration_sum / self.print_duration_count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::types::Temperature;
    use uuid::Uuid;

    fn envelope(event: Event) -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "test".into(),
            timestamp: Utc::now(),
            payload: event,
        }
    }

    #[test]
    fn perfect_temperature_stability() {
        let mut m = HealthMetrics::new();
        let e = envelope(Event::TemperatureUpdate {
            temperatures: vec![Temperature {
                sensor: "e0".into(),
                current: 210.0,
                target: 210.0,
                power: Some(0.5),
            }],
        });
        m.process(&e);
        assert!(m.temperature_stability() > 0.95);
    }

    #[test]
    fn poor_temperature_stability() {
        let mut m = HealthMetrics::new();
        let e = envelope(Event::TemperatureUpdate {
            temperatures: vec![Temperature {
                sensor: "e0".into(),
                current: 220.0,
                target: 210.0,
                power: Some(0.9),
            }],
        });
        m.process(&e);
        assert!(m.temperature_stability() < 0.1);
    }

    #[test]
    fn success_rate_with_mixed_results() {
        let mut m = HealthMetrics::new();
        m.process(&envelope(Event::PrintCompleted {
            total_time: 100.0,
            filament_used: None,
        }));
        m.process(&envelope(Event::PrintCompleted {
            total_time: 200.0,
            filament_used: None,
        }));
        m.process(&envelope(Event::PrintFailed { reason: None }));

        let rate = m.success_rate().unwrap();
        assert!((rate - 2.0 / 3.0).abs() < 0.01);
    }
}
