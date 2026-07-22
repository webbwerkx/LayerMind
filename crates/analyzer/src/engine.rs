//! Analyzer engine — event loop and state management.
//!
//! Subscribes to printer envelopes, maintains per-printer state
//! (print tracker, health metrics, detection windows), and produces
//! observations on significant events.

use std::collections::HashMap;

use layermind_shared::event::{Envelope, Event};
use layermind_shared::observation::{Observation, ObservationKind};
use tokio::sync::broadcast;
use tracing;

use crate::metrics::HealthMetrics;
use crate::print_tracker::PrintTracker;
use crate::rules::DetectionEngine;

/// Default window for health metric snapshots (events).
const HEALTH_SNAPSHOT_INTERVAL: u64 = 100;

/// Max events to keep per printer for detection windows.
const MAX_EVENT_WINDOW: usize = 1000;

/// Runs the analyzer — consumes envelopes and produces observations.
pub struct AnalyzerEngine {
    rx: broadcast::Receiver<Envelope>,
    tx: broadcast::Sender<Observation>,
    printers: HashMap<String, PrinterAnalyzer>,
    event_count: u64,
}

struct PrinterAnalyzer {
    print_tracker: PrintTracker,
    health: HealthMetrics,
    detection: DetectionEngine,
    event_window: Vec<Envelope>,
    connected_since: Option<chrono::DateTime<chrono::Utc>>,
}

impl AnalyzerEngine {
    pub fn new(rx: broadcast::Receiver<Envelope>) -> (Self, broadcast::Receiver<Observation>) {
        let (tx, obs_rx) = broadcast::channel(256);
        (
            Self {
                rx,
                tx,
                printers: HashMap::new(),
                event_count: 0,
            },
            obs_rx,
        )
    }

    pub fn sender(&self) -> broadcast::Sender<Observation> {
        self.tx.clone()
    }

    /// Run the analyzer loop until the broadcast sender is dropped.
    pub async fn run(mut self) {
        tracing::info!("analyzer engine starting");

        loop {
            match self.rx.recv().await {
                Ok(envelope) => {
                    self.process(envelope);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "analyzer lagging behind printer events");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        tracing::info!("analyzer engine stopped");
    }

    fn process(&mut self, envelope: Envelope) {
        let printer_id = envelope.printer_id.clone();
        self.event_count += 1;

        // Collect emissions inside a block to limit the mutable borrow.
        let emissions = {
            let pa = self
                .printers
                .entry(printer_id.clone())
                .or_insert_with(|| PrinterAnalyzer {
                    print_tracker: PrintTracker::new(),
                    health: HealthMetrics::new(),
                    detection: DetectionEngine::new(),
                    event_window: Vec::new(),
                    connected_since: None,
                });

            // Track connection lifecycle.
            match &envelope.payload {
                Event::Connected => {
                    pa.connected_since = Some(envelope.timestamp);
                    pa.health.record_connect();
                }
                Event::Disconnected { .. } => {
                    pa.connected_since = None;
                }
                _ => {}
            }

            // Feed to print tracker and health metrics.
            let print_obs = pa.print_tracker.process(&envelope);
            pa.health.process(&envelope);

            // Maintain event window.
            pa.event_window.push(envelope.clone());
            if pa.event_window.len() > MAX_EVENT_WINDOW {
                pa.event_window.remove(0);
            }

            // Run detection rules.
            let detections = pa.detection.analyze(&pa.event_window);

            // Build emissions list.
            let mut ems: Vec<ObservationKind> = Vec::new();
            if let Some(obs) = print_obs {
                ems.push(obs);
            }
            for det in detections {
                ems.push(ObservationKind::AnomalyDetected {
                    category: det.category,
                    severity: det.severity,
                    message: det.message,
                    evidence: det.evidence,
                });
            }
            ems
        }; // `pa` borrow ends here.

        for kind in emissions {
            self.emit(printer_id.clone(), kind);
        }

        // Periodic health snapshot.
        if self.event_count % HEALTH_SNAPSHOT_INTERVAL == 0 {
            for (pid, pa) in self.printers.iter() {
                let uptime = pa
                    .connected_since
                    .map(|since| (chrono::Utc::now() - since).num_seconds().max(0) as f64)
                    .unwrap_or(0.0);

                self.emit(
                    pid.clone(),
                    ObservationKind::HealthSnapshot {
                        temperature_stability: pa.health.temperature_stability(),
                        success_rate: pa.health.success_rate(),
                        recent_error_count: pa.health.error_count(),
                        recent_warning_count: pa.health.warning_count(),
                        seconds_since_calibration: None,
                        uptime_secs: uptime,
                    },
                );
            }
        }
    }

    fn emit(&self, printer_id: String, kind: ObservationKind) {
        let obs = Observation::new(printer_id, kind);
        let _ = self.tx.send(obs);
    }
}
