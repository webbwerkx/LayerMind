//! Knowledge Engine — event loop and state management.
//!
//! Subscribes to Observations from the Analyzer, maintains per-printer
//! state (tracker, profiler, timeline), and produces Knowledge records.

use std::collections::HashMap;

use layermind_shared::knowledge::{Knowledge, KnowledgeKind};
use layermind_shared::observation::Observation;
use tokio::sync::broadcast;
use tracing;

use crate::profiler::PrinterProfiler;
use crate::timeline::TimelineBuilder;
use crate::tracker::ObservationTracker;

/// How often to emit a KnowledgeSnapshot (every N observations).
const SNAPSHOT_INTERVAL: u64 = 50;

/// Runs the knowledge engine — consumes observations, produces knowledge.
pub struct KnowledgeEngine {
    rx: broadcast::Receiver<Observation>,
    tx: broadcast::Sender<Knowledge>,
    printers: HashMap<String, PrinterKnowledge>,
    observation_count: u64,
}

struct PrinterKnowledge {
    tracker: ObservationTracker,
    profiler: PrinterProfiler,
    timeline: TimelineBuilder,
}

impl KnowledgeEngine {
    pub fn new(rx: broadcast::Receiver<Observation>) -> (Self, broadcast::Receiver<Knowledge>) {
        let (tx, knowledge_rx) = broadcast::channel(256);
        (
            Self {
                rx,
                tx,
                printers: HashMap::new(),
                observation_count: 0,
            },
            knowledge_rx,
        )
    }

    pub fn sender(&self) -> broadcast::Sender<Knowledge> {
        self.tx.clone()
    }

    /// Run the knowledge engine until the broadcast sender is dropped.
    pub async fn run(mut self) {
        tracing::info!("knowledge engine starting");

        loop {
            match self.rx.recv().await {
                Ok(observation) => {
                    self.process(observation);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "knowledge engine lagging");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }

        tracing::info!("knowledge engine stopped");
    }

    fn process(&mut self, observation: Observation) {
        let printer_id = observation.printer_id.clone();
        self.observation_count += 1;

        let pk = self
            .printers
            .entry(printer_id.clone())
            .or_insert_with(|| PrinterKnowledge {
                tracker: ObservationTracker::new(),
                profiler: PrinterProfiler::new(printer_id.clone()),
                timeline: TimelineBuilder::new(),
            });

        // Track the observation lifecycle.
        let (tracked_kind, _profile_suggestion) = pk.tracker.record(&observation);

        // Build/update the printer profile.
        let profile_updated = pk.profiler.process(&observation);

        // Add timeline entries for significant events.
        let timeline_kinds = pk.timeline.process(&observation);

        // Collect emissions (release borrow before calling emit).
        let mut emissions: Vec<KnowledgeKind> = Vec::new();
        if let Some(kind) = tracked_kind {
            emissions.push(kind);
        }
        if profile_updated {
            emissions.push(KnowledgeKind::ProfileUpdated {
                profile: pk.profiler.profile().clone(),
            });
        }
        emissions.extend(timeline_kinds);

        // Periodic knowledge snapshot.
        if self.observation_count % SNAPSHOT_INTERVAL == 0 {
            for pk in self.printers.values() {
                emissions.push(KnowledgeKind::KnowledgeSnapshot {
                    active_observation_count: pk.tracker.active_count(),
                    resolved_observation_count: pk.tracker.resolved_count(),
                    profile_age_secs: (chrono::Utc::now() - pk.profiler.profile().updated_at)
                        .num_seconds()
                        .max(0) as f64,
                    timeline_event_count: pk.timeline.len(),
                });
            }
        }

        for kind in emissions {
            self.emit(printer_id.clone(), kind);
        }
    }

    fn emit(&self, printer_id: String, kind: KnowledgeKind) {
        let record = Knowledge::new(printer_id, kind);
        let _ = self.tx.send(record);
    }
}
