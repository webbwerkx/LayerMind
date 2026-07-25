//! Machine intelligence crate — hardware discovery, capability
//! derivation, and confidence scoring for 3D printer hardware.
//!
//! # Architecture
//!
//! ```text
//! Moonraker objects + printer.cfg
//!         │
//!   HardwareDiscovery    ─── discovers identity + components
//!         │
//!   CapabilityEngine     ─── derives capabilities from hardware
//!         │
//!   ConfidenceEngine     ─── assigns confidence per property
//!         │
//!   MachineProfileBuilder ─── assembles the final MachineProfile
//! ```
//!
//! All stages are deterministic. No AI, no networking, no database
//! access except shared models.

pub mod builder;
pub mod capability;
pub mod config_parser;
pub mod confidence;
pub mod discovery;
pub mod library;

pub use builder::MachineProfileBuilder;
pub use capability::CapabilityEngine;
pub use config_parser::{merge_configs, parse_config, ParsedConfig};
pub use confidence::ConfidenceEngine;
pub use discovery::HardwareDiscovery;
pub use library::HardwareLibrary;
