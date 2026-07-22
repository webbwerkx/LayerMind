//! LayerMind Context Engine.
//!
//! Synthesizes knowledge into AI-consumable printer context documents.
//! The Context Engine subscribes to Knowledge records from the Knowledge
//! Engine, caches state per printer in a `ContextStore`, and produces
//! structured `PrinterContext` briefings designed for LLM prompt injection.
//!
//! Architecture:
//!   KnowledgeEngine → broadcast(Knowledge) → ContextEngine (ingestion)
//!                                                    │
//!                                              ContextStore (Arc)
//!                                                    │
//!                    ┌───────────────────────────────┼────────────────┐
//!                    │                               │                │
//!              PrintDoctor                         CLI          REST / Web
//!              (AI consumer)                   (future)         (future)
//!
//! Future specialized views (TroubleshootingContext, CalibrationContext,
//! MaintenanceContext) will be additional methods on `ContextStore`,
//! projecting the same cached state through different lenses.

mod engine;
mod store;

pub use engine::ContextEngine;
pub use store::ContextStore;
