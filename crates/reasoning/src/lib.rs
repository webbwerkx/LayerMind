//! LayerMind Reasoning Engine.
//!
//! AI-powered diagnostic pipeline that converts PrinterContext into
//! trustworthy, evidence-backed recommendations.
//!
//! Architecture:
//!   PrinterContext → PromptBuilder → AiProvider → ResponseParser
//!     → TrustValidator → ValidatedRecommendation
//!
//! The pipeline is provider-agnostic via the AiProvider trait. First
//! implementation is OpenAI-compatible (covers OpenAI, OpenRouter,
//! local llama.cpp/Ollama servers).

pub mod diagnostic;
pub mod parser;
pub mod prompt;
pub mod provider;
pub mod trust;

pub use diagnostic::PrintDoctor;
pub use provider::{AiProvider, AiRequest, AiResponse, TokenUsage};
