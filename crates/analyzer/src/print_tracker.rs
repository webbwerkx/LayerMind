//! Print lifecycle tracker.
//!
//! Tracks a single printer's print lifecycle: when a print starts,
//! how it progresses, whether it completes or fails. Produces
//! PrintStarted, PrintCompleted/PrintFailed, and PrintSummary
//! observations.

use chrono::{DateTime, Utc};
use layermind_shared::event::{Envelope, Event};
use layermind_shared::observation::ObservationKind;

/// Tracks the lifecycle of a print on one printer.
#[derive(Debug, Default)]
pub struct PrintTracker {
    /// Currently active print, if any.
    current_print: Option<ActivePrint>,
    /// Number of consecutive failed prints.
    consecutive_failures: u32,
    /// Number of successful prints.
    successful_prints: u32,
}

#[derive(Debug)]
struct ActivePrint {
    filename: String,
    started_at: DateTime<Utc>,
    last_progress: f64,
    last_layer: Option<u32>,
    total_layers: Option<u32>,
    #[allow(dead_code)]
    filament_used: Option<f64>,
    paused: bool,
    pause_count: u32,
}

impl PrintTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an envelope. Returns an observation if a significant
    /// print lifecycle transition occurred.
    pub fn process(&mut self, envelope: &Envelope) -> Option<ObservationKind> {
        match &envelope.payload {
            Event::PrintStarted { filename, .. } => {
                self.current_print = Some(ActivePrint {
                    filename: filename.clone(),
                    started_at: envelope.timestamp,
                    last_progress: 0.0,
                    last_layer: None,
                    total_layers: None,
                    filament_used: None,
                    paused: false,
                    pause_count: 0,
                });
                Some(ObservationKind::PrintStarted {
                    filename: filename.clone(),
                })
            }

            Event::PrintProgress {
                progress,
                current_layer,
                total_layers,
                ..
            } => {
                if let Some(ref mut p) = self.current_print {
                    p.last_progress = *progress;
                    p.last_layer = *current_layer;
                    p.total_layers = *total_layers;
                }
                None
            }

            Event::PrintPaused { .. } => {
                if let Some(ref mut p) = self.current_print {
                    p.paused = true;
                    p.pause_count += 1;
                }
                None
            }

            Event::PrintResumed => {
                if let Some(ref mut p) = self.current_print {
                    p.paused = false;
                }
                None
            }

            Event::PrintCompleted {
                total_time,
                filament_used,
            } => {
                let summary = self.finish_print(Some(*total_time), *filament_used, None);
                Some(summary)
            }

            Event::PrintFailed { reason } => {
                let duration = self
                    .current_print
                    .as_ref()
                    .map(|p| (envelope.timestamp - p.started_at).num_milliseconds() as f64 / 1000.0)
                    .unwrap_or(0.0);

                let summary = self.finish_print(Some(duration), None, reason.clone());
                Some(summary)
            }

            Event::PrintCancelled => {
                let summary = self.finish_print(None, None, Some("cancelled".into()));
                Some(summary)
            }

            _ => None,
        }
    }

    fn finish_print(
        &mut self,
        duration_secs: Option<f64>,
        filament_used: Option<f64>,
        failure_reason: Option<String>,
    ) -> ObservationKind {
        let print = self.current_print.take();

        let filename = print
            .as_ref()
            .map(|p| p.filename.clone())
            .unwrap_or_default();
        let success = failure_reason.is_none();
        let duration = duration_secs.unwrap_or(0.0);
        let total_layers = print.as_ref().and_then(|p| p.total_layers);

        if success {
            self.successful_prints += 1;
            self.consecutive_failures = 0;
        } else {
            self.consecutive_failures += 1;
        }

        let highlights = if let Some(ref p) = print {
            let mut h = Vec::new();
            if p.pause_count > 0 {
                h.push(format!("paused {} time(s)", p.pause_count));
            }
            if self.consecutive_failures > 1 {
                h.push(format!(
                    "{} consecutive failure(s)",
                    self.consecutive_failures
                ));
            }
            h
        } else {
            vec![]
        };

        ObservationKind::PrintSummary {
            filename,
            success,
            duration_secs: duration,
            filament_used_mm: filament_used,
            total_layers,
            failure_reason,
            highlights,
        }
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    pub fn is_printing(&self) -> bool {
        self.current_print.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn full_print_lifecycle() {
        let mut tracker = PrintTracker::new();

        let start = tracker.process(&envelope(Event::PrintStarted {
            filename: "test.gcode".into(),
            estimated_time: Some(3600.0),
        }));
        assert!(matches!(start, Some(ObservationKind::PrintStarted { .. })));

        let progress = tracker.process(&envelope(Event::PrintProgress {
            progress: 0.5,
            elapsed: 1800.0,
            remaining: Some(1800.0),
            current_layer: Some(50),
            total_layers: Some(100),
        }));
        assert!(progress.is_none());

        let complete = tracker.process(&envelope(Event::PrintCompleted {
            total_time: 3600.0,
            filament_used: Some(5000.0),
        }));
        assert!(matches!(
            complete,
            Some(ObservationKind::PrintSummary { success: true, .. })
        ));
    }

    #[test]
    fn tracks_consecutive_failures() {
        let mut tracker = PrintTracker::new();

        tracker.process(&envelope(Event::PrintStarted {
            filename: "fail1.gcode".into(),
            estimated_time: None,
        }));
        tracker.process(&envelope(Event::PrintFailed {
            reason: Some("error".into()),
        }));
        assert_eq!(tracker.consecutive_failures(), 1);

        tracker.process(&envelope(Event::PrintStarted {
            filename: "fail2.gcode".into(),
            estimated_time: None,
        }));
        tracker.process(&envelope(Event::PrintFailed {
            reason: Some("error".into()),
        }));
        assert_eq!(tracker.consecutive_failures(), 2);

        tracker.process(&envelope(Event::PrintStarted {
            filename: "success.gcode".into(),
            estimated_time: None,
        }));
        tracker.process(&envelope(Event::PrintCompleted {
            total_time: 100.0,
            filament_used: None,
        }));
        assert_eq!(tracker.consecutive_failures(), 0);
    }
}
