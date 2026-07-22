//! LayerMind Analyzer Engine.
//!
//! Deterministic analysis layer that converts raw telemetry events into
//! higher-level observations. This is NOT an AI/LLM system — it is a
//! rules engine that produces structured, testable insights.
//!
//! Architecture:
//!   Telemetry Events → AnalyzerEngine → Observations → (AI, DB, UI)
//!
//! The analyzer subscribes to the same printer broadcast channel as the
//! telemetry pipeline. It is a parallel, independent consumer.

mod engine;
mod metrics;
mod print_tracker;
pub mod rules;

pub use engine::AnalyzerEngine;
pub use metrics::HealthMetrics;
pub use print_tracker::PrintTracker;
pub use rules::DetectionEngine;
