//! MachineProfileBuilder — orchestrates the hardware discovery,
//! capability derivation, confidence calibration, and profile
//! assembly pipeline.
//!
//! This is the public entry point for the machine crate.

use chrono::Utc;
use layermind_shared::machine::*;

use crate::capability::CapabilityEngine;
use crate::confidence::ConfidenceEngine;
use crate::discovery::HardwareDiscovery;
use crate::library::HardwareLibrary;

/// Builds a complete [`MachineProfile`] from Moonraker data.
///
/// # Pipeline
///
/// ```text
/// 1. HardwareDiscovery → identity + hardware snapshot
/// 2. CapabilityEngine → derived capabilities
/// 3. ConfidenceEngine → calibrate confidence
/// 4. Assemble final MachineProfile
/// ```
#[derive(Debug)]
pub struct MachineProfileBuilder {
    library: HardwareLibrary,
}

impl MachineProfileBuilder {
    pub fn new() -> Self {
        Self {
            library: HardwareLibrary::new(),
        }
    }

    /// Build a full machine profile from raw Moonraker data.
    ///
    /// `printer_info` and `system_info` come from Moonraker API.
    /// `printer_objects` is the raw `printer.objects.query` response.
    pub fn build(
        &self,
        printer_id: &str,
        printer_info: Option<&serde_json::Value>,
        system_info: Option<&serde_json::Value>,
        printer_objects: Option<&serde_json::Value>,
    ) -> MachineProfile {
        let identity =
            HardwareDiscovery::discover_identity(printer_id, printer_info, None, system_info);

        let mut hardware = HardwareDiscovery::discover_hardware(printer_objects, system_info);

        // Apply library matches to boost hardware component confidence.
        self.apply_library_to_hardware(&mut hardware);

        let capabilities = CapabilityEngine::derive(&hardware);

        let mut profile = MachineProfile {
            identity,
            hardware,
            capabilities,
            generated_at: Utc::now(),
        };

        ConfidenceEngine::calibrate(&mut profile);
        profile
    }

    /// Build an empty (unknown) profile for a printer that has sent
    /// telemetry but no hardware data has been collected yet.
    pub fn unknown_profile(printer_id: &str) -> MachineProfile {
        MachineProfile {
            identity: MachineIdentity {
                printer_id: printer_id.into(),
                nickname: None,
                manufacturer: None,
                model: None,
                custom_build: Property::assumed(false),
                machine_type: Property::assumed(MachineType::Unknown),
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

    pub(crate) fn apply_library_to_hardware(&self, hardware: &mut MachineHardware) {
        // Walk every component and match against the hardware library.
        for comp in hardware.mcus.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.extruders.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.hotends.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.probes.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.accelerometers.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.cooling.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.filament_sensors.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.heaters.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.thermistors.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.can_devices.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.displays.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.networking.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.load_cells.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.toolheads.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        for comp in hardware.tool_changers.iter_mut() {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }

        // Optional components.
        if let Some(ref mut comp) = hardware.control_board {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        if let Some(ref mut comp) = hardware.bed {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
        if let Some(ref mut comp) = hardware.enclosure {
            self.library.match_component(&comp.name, &mut comp.known_profile);
        }
    }
}

impl Default for MachineProfileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_profile_with_minimal_data() {
        let builder = MachineProfileBuilder::new();
        let profile = builder.build("p1", None, None, None);
        assert_eq!(profile.identity.printer_id, "p1");
        assert!(!profile.capabilities.supports_input_shaping.value);
    }

    #[test]
    fn builds_profile_with_system_info() {
        let builder = MachineProfileBuilder::new();
        let info = serde_json::json!({
            "klipper_path": "/home/pi/klipper/klippy/klippy.py",
            "available_mcus": [{"name": "mcu"}]
        });
        let profile = builder.build("p2", None, Some(&info), None);
        assert_eq!(
            profile.identity.firmware.as_ref().unwrap().mcu_count.value,
            1
        );
    }

    #[test]
    fn unknown_profile_is_valid() {
        let profile = MachineProfileBuilder::unknown_profile("ghost");
        assert_eq!(profile.identity.printer_id, "ghost");
        assert!(profile.identity.firmware.is_none());
        assert_eq!(profile.capabilities.supports_input_shaping.confidence, 0.0);
    }

    #[test]
    fn hardware_with_adxl_enables_input_shaping() {
        let builder = MachineProfileBuilder::new();
        let mut profile = builder.build("p-adxl", None, None, None);
        profile.hardware.accelerometers = vec![Component {
            id: "a0".into(),
            name: "ADXL".into(),
            details: AccelerometerSpec {
                chip: Property::observed(AccelChip::ADXL345),
                axes: 3,
                mounted_on: "toolhead".into(),
                bus: None,
            },
            known_profile: None,
            installed: None,
            replaced: None,
        }];

        // Re-derive capabilities (normally done in build, but we
        // modified hardware after build).
        let caps = CapabilityEngine::derive(&profile.hardware);
        profile.capabilities = caps;

        assert!(profile.capabilities.supports_input_shaping.value);
        assert!(profile.capabilities.supports_adxl.value);
    }

    #[test]
    fn library_matches_probe_component() {
        let builder = MachineProfileBuilder::new();
        let mut profile = builder.build("p-probe", None, None, None);

        // Add a probe with a name that should match a library profile.
        profile.hardware.probes = vec![Component {
            id: "probe_0".into(),
            name: "BLTouch v3.1".into(),
            details: ProbeSpec {
                probe_type: Property::observed(ProbeType::BlTouch),
                uses_endstop_pin: true,
                dockable: false,
            },
            known_profile: None,
            installed: None,
            replaced: None,
        }];

        builder.apply_library_to_hardware(&mut profile.hardware);

        assert_eq!(
            profile.hardware.probes[0].known_profile.as_deref(),
            Some("BLTouch")
        );
    }

    #[test]
    fn library_does_not_match_unknown_component() {
        let builder = MachineProfileBuilder::new();
        let mut profile = builder.build("p-unknown", None, None, None);

        profile.hardware.probes = vec![Component {
            id: "probe_0".into(),
            name: "Inductive Probe 5mm".into(),
            details: ProbeSpec {
                probe_type: Property::observed(ProbeType::MicroProbe),
                uses_endstop_pin: true,
                dockable: false,
            },
            known_profile: None,
            installed: None,
            replaced: None,
        }];

        builder.apply_library_to_hardware(&mut profile.hardware);

        assert!(profile.hardware.probes[0].known_profile.is_none());
    }
}
