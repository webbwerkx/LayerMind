//! LayerMind Context Engine.
//!
//! Synthesizes knowledge into AI-consumable printer context documents.
//! The Context Engine subscribes to Knowledge records from the Knowledge
//! Engine, caches state per printer, and produces structured PrinterContext
//! briefings designed for LLM prompt injection.
//!
//! Architecture:
//!   KnowledgeEngine → broadcast(Knowledge) → ContextEngine (cached state)
//!     → context(printer_id) → PrinterContext (AI-ready briefing)
//!
//! Future specialized views (TroubleshootingContext, CalibrationContext,
//! MaintenanceContext) will be additional methods on the same engine,
//! projecting the same cached state through different lenses.

mod engine;

pub use engine::ContextEngine;
