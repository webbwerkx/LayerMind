//! LayerMind Reasoning Engine.
//!
//! AI-powered diagnostic pipeline that converts PrinterContext into
//! trustworthy, evidence-backed recommendations.
//!
//! Architecture (Phase 2.3):
//!   PrinterContext → EvidenceRanker (top-N ranked facts)
//!     → ContradictionDetector (conflict detection)
//!     → PromptBuilder (optimized prompt with history + trends)
//!     → AiProvider (model)
//!     → ResponseParser (JSON extraction + fallback)
//!     → ConfidenceCalibrator (deterministic adjustment)
//!     → Prioritizer (deterministic action ordering)
//!     → TrustValidator (claim cross-reference)
//!     → ValidatedRecommendation
//!
//! Every step except AiProvider is deterministic. The pipeline is
//! provider-agnostic via the AiProvider trait.

pub mod confidence;
pub mod contradiction;
pub mod diagnostic;
pub mod evidence;
pub mod parser;
pub mod prioritization;
pub mod prompt;
pub mod provider;
pub mod trust;

pub use diagnostic::PrintDoctor;
pub use provider::{AiProvider, AiRequest, AiResponse, TokenUsage};
