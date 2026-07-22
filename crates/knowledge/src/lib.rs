//! LayerMind Knowledge Engine.
//!
//! Transforms observations into persistent, structured knowledge about
//! printers. The Knowledge Engine sits between the Analyzer and the
//! future AI Engine — it organizes observations into profiles, tracks
//! lifecycle, builds timelines, and scores importance.
//!
//! Architecture:
//!   Observations → KnowledgeEngine → Knowledge → (AI, DB, UI)
//!
//! The engine is pure in-memory state. Persistence is handled
//! externally via a KnowledgeSink (database crate).

mod engine;
mod profiler;
mod scoring;
mod timeline;
mod tracker;

pub use engine::KnowledgeEngine;
pub use profiler::PrinterProfiler;
pub use scoring::KnowledgeScorer;
pub use timeline::TimelineBuilder;
pub use tracker::ObservationTracker;
