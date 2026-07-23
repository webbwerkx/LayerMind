//! LayerMind shared types and contracts.
//!
//! This crate defines the canonical data types, error enums, and trait
//! interfaces used across all LayerMind crates. It contains zero business
//! logic and minimal dependencies. Every crate depends on this one;
//! this crate depends on nothing internal.

pub mod context;
pub mod error;
pub mod event;
pub mod history;
pub mod knowledge;
pub mod machine;
pub mod observation;
pub mod printer;
pub mod recommendation;
pub mod sink;
pub mod types;
