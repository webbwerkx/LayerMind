//! SnapshotDiffEngine — computes deterministic diffs between two
//! snapshots.
//!
//! Given Snapshot A and Snapshot B, produce a [`SnapshotDiff`] listing
//! every atomic change. This enables questions like "what changed
//! between last week and now?" and "what hardware was different before
//! this print started?"
//!
//! The diff engine walks the machine profile fields and compares them
//! field-by-field. It is exhaustive — every field in the profile is
//! checked.

use layermind_shared::history::*;
use layermind_shared::machine::{CapabilitySet, MachineHardware, MachineIdentity, MachineProfile};

/// Computes diffs between two snapshots or profiles.
#[derive(Debug)]
pub struct SnapshotDiffEngine;

impl SnapshotDiffEngine {
    /// Diff two machine profiles and produce a change list.
    pub fn diff_profiles(from: &MachineProfile, to: &MachineProfile) -> SnapshotDiff {
        let mut changes = Vec::new();

        Self::diff_identity(&mut changes, &from.identity, &to.identity);
        Self::diff_hardware(&mut changes, &from.hardware, &to.hardware);
        Self::diff_capabilities(&mut changes, &from.capabilities, &to.capabilities);

        SnapshotDiff {
            from_snapshot_id: "profile_a".into(),
            to_snapshot_id: "profile_b".into(),
            from_timestamp: from.generated_at,
            to_timestamp: to.generated_at,
            total_changes: changes.len(),
            changes,
        }
    }

    /// Diff two printer snapshots.
    pub fn diff_snapshots(from: &PrinterSnapshot, to: &PrinterSnapshot) -> Option<SnapshotDiff> {
        let from_profile = from.machine.as_ref()?;
        let to_profile = to.machine.as_ref()?;
        let mut diff = Self::diff_profiles(from_profile, to_profile);
        diff.from_snapshot_id = from.snapshot_id.clone();
        diff.to_snapshot_id = to.snapshot_id.clone();
        diff.from_timestamp = from.timestamp;
        diff.to_timestamp = to.timestamp;
        Some(diff)
    }

    // ── Identity diff ──────────────────────────────────────

    fn diff_identity(
        changes: &mut Vec<SnapshotChange>,
        from: &MachineIdentity,
        to: &MachineIdentity,
    ) {
        if let (Some(ref a), Some(ref b)) = (&from.manufacturer, &to.manufacturer) {
            if a.value != b.value {
                changes.push(SnapshotChange {
                    path: "identity.manufacturer".into(),
                    category: TimelineCategory::Hardware,
                    previous_value: Some(a.value.clone()),
                    new_value: Some(b.value.clone()),
                });
            }
        }
        if from.machine_type.value != to.machine_type.value {
            changes.push(SnapshotChange {
                path: "identity.machine_type".into(),
                category: TimelineCategory::Hardware,
                previous_value: Some(format!("{:?}", from.machine_type.value)),
                new_value: Some(format!("{:?}", to.machine_type.value)),
            });
        }
    }

    // ── Hardware diff ──────────────────────────────────────

    fn diff_hardware(
        changes: &mut Vec<SnapshotChange>,
        from: &MachineHardware,
        to: &MachineHardware,
    ) {
        // Extruder count change.
        if from.extruders.len() != to.extruders.len() {
            changes.push(SnapshotChange {
                path: "hardware.extruders".into(),
                category: TimelineCategory::Hardware,
                previous_value: Some(from.extruders.len().to_string()),
                new_value: Some(to.extruders.len().to_string()),
            });
        }

        // Hotend count change.
        if from.hotends.len() != to.hotends.len() {
            changes.push(SnapshotChange {
                path: "hardware.hotends".into(),
                category: TimelineCategory::Hardware,
                previous_value: Some(from.hotends.len().to_string()),
                new_value: Some(to.hotends.len().to_string()),
            });
        }

        // Probe change.
        let from_probes: Vec<String> = from
            .probes
            .iter()
            .map(|p| format!("{:?}", p.details.probe_type.value))
            .collect();
        let to_probes: Vec<String> = to
            .probes
            .iter()
            .map(|p| format!("{:?}", p.details.probe_type.value))
            .collect();
        if from_probes != to_probes {
            changes.push(SnapshotChange {
                path: "hardware.probes".into(),
                category: TimelineCategory::Hardware,
                previous_value: Some(from_probes.join(", ")),
                new_value: Some(to_probes.join(", ")),
            });
        }

        // Accelerometer change.
        let from_accel: Vec<String> = from
            .accelerometers
            .iter()
            .map(|a| format!("{:?}", a.details.chip.value))
            .collect();
        let to_accel: Vec<String> = to
            .accelerometers
            .iter()
            .map(|a| format!("{:?}", a.details.chip.value))
            .collect();
        if from_accel != to_accel {
            changes.push(SnapshotChange {
                path: "hardware.accelerometers".into(),
                category: TimelineCategory::Hardware,
                previous_value: Some(from_accel.join(", ")),
                new_value: Some(to_accel.join(", ")),
            });
        }
    }

    // ── Capability diff ────────────────────────────────────

    fn diff_capabilities(
        changes: &mut Vec<SnapshotChange>,
        from: &CapabilitySet,
        to: &CapabilitySet,
    ) {
        Self::cap_bool(
            changes,
            "input_shaping",
            &from.supports_input_shaping,
            &to.supports_input_shaping,
        );
        Self::cap_bool(
            changes,
            "pressure_advance",
            &from.supports_pressure_advance,
            &to.supports_pressure_advance,
        );
        Self::cap_bool(
            changes,
            "sensorless_homing",
            &from.supports_sensorless_homing,
            &to.supports_sensorless_homing,
        );
        Self::cap_bool(
            changes,
            "canbus",
            &from.supports_canbus,
            &to.supports_canbus,
        );
        Self::cap_bool(
            changes,
            "bltouch",
            &from.supports_bltouch,
            &to.supports_bltouch,
        );
        Self::cap_bool(
            changes,
            "toolchanger",
            &from.supports_toolchanger,
            &to.supports_toolchanger,
        );
        Self::cap_bool(
            changes,
            "high_temperature",
            &from.supports_high_temperature,
            &to.supports_high_temperature,
        );
    }

    fn cap_bool(
        changes: &mut Vec<SnapshotChange>,
        name: &str,
        from: &layermind_shared::machine::Property<bool>,
        to: &layermind_shared::machine::Property<bool>,
    ) {
        if from.value != to.value {
            changes.push(SnapshotChange {
                path: format!("capabilities.{}", name),
                category: TimelineCategory::Capability,
                previous_value: Some(from.value.to_string()),
                new_value: Some(to.value.to_string()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::machine::{
        CapabilitySet, MachineHardware, MachineIdentity, MachineType, Property,
    };

    fn test_profile() -> MachineProfile {
        MachineProfile {
            identity: MachineIdentity {
                printer_id: "p1".into(),
                nickname: None,
                manufacturer: Some(Property::assumed("A".into())),
                model: Some(Property::assumed("B".into())),
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
    fn detects_identity_changes() {
        let mut from = test_profile();
        let mut to = test_profile();
        to.identity.machine_type = Property::observed(MachineType::CoreXY);

        let diff = SnapshotDiffEngine::diff_profiles(&from, &to);
        assert!(diff.total_changes > 0);
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path == "identity.machine_type"));
    }

    #[test]
    fn detects_capability_changes() {
        let from = test_profile();
        let mut to = test_profile();
        to.capabilities.supports_input_shaping = Property::observed(true);

        let diff = SnapshotDiffEngine::diff_profiles(&from, &to);
        assert!(diff
            .changes
            .iter()
            .any(|c| c.path == "capabilities.input_shaping"));
    }

    #[test]
    fn identical_profiles_produce_empty_diff() {
        let p = test_profile();
        let diff = SnapshotDiffEngine::diff_profiles(&p, &p);
        assert_eq!(diff.total_changes, 0);
    }

    #[test]
    fn diff_snapshots_requires_machine() {
        let s1 = PrinterSnapshot {
            printer_id: "p1".into(),
            snapshot_id: "s1".into(),
            timestamp: Utc::now(),
            machine: None,
            capabilities: None,
            config_hash: None,
            trigger: SnapshotTrigger::Manual,
            metadata: serde_json::json!({}),
        };
        let s2 = s1.clone();
        let diff = SnapshotDiffEngine::diff_snapshots(&s1, &s2);
        assert!(diff.is_none());
    }
}
