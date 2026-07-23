//! SnapshotBuilder — captures current printer state as immutable
//! snapshots.
//!
//! Snapshots are periodic full-state captures that support diff
//! generation and historical queries. The builder takes the current
//! known state (from the machine crate, knowledge crate, etc.) and
//! serializes it into a snapshot record.

use chrono::Utc;
use layermind_shared::history::*;
use layermind_shared::machine::MachineProfile;
use uuid::Uuid;

/// Builds snapshots from current state.
#[derive(Debug)]
pub struct SnapshotBuilder;

impl SnapshotBuilder {
    /// Create a full printer snapshot from a machine profile.
    pub fn printer_snapshot(
        printer_id: &str,
        machine: Option<&MachineProfile>,
        trigger: SnapshotTrigger,
    ) -> PrinterSnapshot {
        PrinterSnapshot {
            printer_id: printer_id.to_string(),
            snapshot_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            machine: machine.cloned(),
            capabilities: machine.map(|m| m.capabilities.clone()),
            config_hash: machine
                .as_ref()
                .and_then(|m| m.identity.firmware.as_ref())
                .and_then(|f| f.config_hash.clone()),
            trigger,
            metadata: serde_json::json!({}),
        }
    }

    /// Create a machine-only snapshot.
    pub fn machine_snapshot(printer_id: &str, profile: &MachineProfile) -> MachineSnapshot {
        MachineSnapshot {
            printer_id: printer_id.to_string(),
            timestamp: Utc::now(),
            identity: Some(profile.identity.clone()),
            hardware: profile.hardware.clone(),
        }
    }

    /// Create a capability-only snapshot.
    pub fn capability_snapshot(printer_id: &str, profile: &MachineProfile) -> CapabilitySnapshot {
        CapabilitySnapshot {
            printer_id: printer_id.to_string(),
            timestamp: Utc::now(),
            capabilities: profile.capabilities.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_shared::machine::{
        CapabilitySet, MachineHardware, MachineIdentity, MachineType, Property,
    };

    fn test_profile(printer_id: &str) -> MachineProfile {
        MachineProfile {
            identity: MachineIdentity {
                printer_id: printer_id.into(),
                nickname: None,
                manufacturer: Some(Property::assumed("Test".into())),
                model: Some(Property::assumed("T1".into())),
                custom_build: Property::observed(false),
                machine_type: Property::observed(MachineType::Cartesian),
                serial_number: None,
                firmware: None,
                discovered_at: Utc::now(),
                updated_at: Utc::now(),
            },
            hardware: MachineHardware::default(),
            capabilities: CapabilitySet::empty(),
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn builds_printer_snapshot() {
        let profile = test_profile("p1");
        let snap = SnapshotBuilder::printer_snapshot("p1", Some(&profile), SnapshotTrigger::Manual);
        assert_eq!(snap.printer_id, "p1");
        assert!(snap.machine.is_some());
        assert!(snap.capabilities.is_some());
        assert_eq!(snap.trigger, SnapshotTrigger::Manual);
    }

    #[test]
    fn builds_machine_snapshot() {
        let profile = test_profile("p2");
        let snap = SnapshotBuilder::machine_snapshot("p2", &profile);
        assert!(snap.identity.is_some());
    }

    #[test]
    fn builds_capability_snapshot() {
        let profile = test_profile("p3");
        let snap = SnapshotBuilder::capability_snapshot("p3", &profile);
        assert!(!snap.capabilities.supports_input_shaping.value);
    }

    #[test]
    fn snapshot_ids_are_unique() {
        let profile = test_profile("p4");
        let s1 =
            SnapshotBuilder::printer_snapshot("p4", Some(&profile), SnapshotTrigger::Scheduled);
        let s2 =
            SnapshotBuilder::printer_snapshot("p4", Some(&profile), SnapshotTrigger::Scheduled);
        assert_ne!(s1.snapshot_id, s2.snapshot_id);
    }
}
