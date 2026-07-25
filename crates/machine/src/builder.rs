//! MachineProfileBuilder — orchestrates the hardware discovery,
//! capability derivation, confidence calibration, and profile
//! assembly pipeline.
//!
//! This is the public entry point for the machine crate.

use chrono::Utc;
use layermind_shared::machine::*;

use crate::capability::CapabilityEngine;
use crate::config_parser::ParsedConfig;
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

    pub fn enrich_from_config(&self, profile: &mut MachineProfile, config: &ParsedConfig) {
        // ── 1. Stepper drivers ──────────────────────────────────
        if let Some(ref mut motion) = profile.hardware.motion_system {
            for axis in &mut motion.axes {
                let axis_name = format!("stepper_{}", axis.name);
                if let Some(driver_name) = config.stepper_drivers.get(&axis_name) {
                    let chip = parse_driver_chip(driver_name);
                    axis.driver = Some(Driver {
                        chip: Property::new(chip, InformationSource::PrinterConfig, 0.95),
                        mode: Property::new(DriverMode::Unknown, InformationSource::PrinterConfig, 0.5),
                        current: None,
                        address: None,
                    });
                    if let Some(steps) = config.microsteps.get(&axis.name) {
                        axis.microsteps = Some(Property::new(*steps, InformationSource::PrinterConfig, 0.95));
                    }
                    if let Some(dist) = config.rotation_distance.get(&axis.name) {
                        axis.rotation_distance = Some(Property::new(*dist, InformationSource::PrinterConfig, 0.95));
                    }
                }
            }
        }

        // ── 2. Endstops ─────────────────────────────────────────
        if let Some(ref mut motion) = profile.hardware.motion_system {
            for axis in &mut motion.axes {
                if config.endstops.contains_key(&axis.name) {
                    axis.endstop = Some(EndstopType::Unknown);
                }
            }
        }

        // ── 3. Probe ────────────────────────────────────────────
        if let Some(ref probe_section) = config.probe {
            let exists = !profile.hardware.probes.is_empty();
            if !exists {
                let probe_type = match probe_section.as_str() {
                    "bltouch" => ProbeType::BlTouch,
                    "probe" => ProbeType::Unknown,
                    _ => ProbeType::Unknown,
                };
                profile.hardware.probes.push(Component {
                    id: "probe_0".into(),
                    name: probe_section.clone(),
                    details: ProbeSpec {
                        probe_type: Property::new(probe_type, InformationSource::PrinterConfig, 0.9),
                        uses_endstop_pin: true,
                        dockable: false,
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }
        }

        // ── 4. Accelerometer ────────────────────────────────────
        if let Some(ref accel_section) = config.accelerometer {
            let exists = !profile.hardware.accelerometers.is_empty();
            if !exists {
                let chip = match accel_section.as_str() {
                    "adxl345" => AccelChip::ADXL345,
                    "lis2dw" => AccelChip::LIS2DW,
                    "mpu9250" => AccelChip::MPU9250,
                    _ => AccelChip::Unknown,
                };
                profile.hardware.accelerometers.push(Component {
                    id: "accel_0".into(),
                    name: accel_section.clone(),
                    details: AccelerometerSpec {
                        chip: Property::new(chip, InformationSource::PrinterConfig, 0.9),
                        axes: 3,
                        mounted_on: "toolhead".into(),
                        bus: None,
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }
        }

        // ── 5. Thermistors ──────────────────────────────────────
        for (section, sensor) in &config.sensor_types {
            let sensor_type = parse_sensor_type(sensor);
            let name = match section.as_str() {
                "extruder" => "Extruder Thermistor",
                "heater_bed" => "Bed Thermistor",
                s => s,
            };
            let id = format!("thermistor_{}", section);
            let exists = profile.hardware.thermistors.iter().any(|t| t.id == id);
            if !exists {
                profile.hardware.thermistors.push(Component {
                    id,
                    name: name.to_string(),
                    details: ThermistorSpec {
                        sensor_type: Property::new(sensor_type, InformationSource::PrinterConfig, 0.9),
                        pull_up: None,
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }
        }

        // ── 6. Fans ─────────────────────────────────────────────
        for fan_section in &config.fans {
            let id = format!("fan_{}", fan_section);
            let exists = profile.hardware.cooling.iter().any(|f| f.id == id);
            if !exists {
                let fan_type = if fan_section.starts_with("heater_fan") {
                    FanType::Hotend
                } else if fan_section.starts_with("controller_fan") {
                    FanType::Controller
                } else if fan_section.starts_with("exhaust_fan") {
                    FanType::Filter
                } else {
                    FanType::PartCooling
                };
                profile.hardware.cooling.push(Component {
                    id,
                    name: fan_section.clone(),
                    details: FanSpec {
                        fan_type: Property::new(fan_type, InformationSource::PrinterConfig, 0.9),
                        pwm: true,
                        tachometer: false,
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }
        }

        // ── 7. Heater pins ──────────────────────────────────────
        for (section, _pin) in &config.heater_pins {
            let id = format!("heater_{}", section);
            let exists = profile.hardware.heaters.iter().any(|h| h.id == id);
            if !exists {
                profile.hardware.heaters.push(Component {
                    id,
                    name: section.clone(),
                    details: HeaterSpec {
                        power: None,
                        heater_type: Property::assumed(HeaterType::Unknown),
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }
        }

        // ── 8. Input shaper ─────────────────────────────────────
        if config.input_shaper.is_some() {
            profile.capabilities.supports_input_shaping = Property::new(
                true,
                InformationSource::PrinterConfig,
                0.95,
            );
        }

        // ── 9. Nozzle diameter ──────────────────────────────────
        if let Some(diameter) = config.nozzle_diameter {
            if profile.hardware.extruders.first_mut().is_some() {
                // The ExtruderSpec doesn't have nozzle info — only HotendSpec does.
                // We'll need to ensure a hotend exists.
                let hotend_exists = !profile.hardware.hotends.is_empty();
                if !hotend_exists {
                    profile.hardware.hotends.push(Component {
                        id: "hotend_0".into(),
                        name: "Extruder Hotend".into(),
                        details: HotendSpec {
                            hotend_type: Property::assumed(HotendType::Unknown),
                            max_temperature: Property::assumed(260.0),
                            heater_power: None,
                            nozzle_diameter: Some(Property::new(
                                diameter,
                                InformationSource::PrinterConfig,
                                0.95,
                            )),
                            pt1000: Property::assumed(false),
                            thermocouple: Property::assumed(false),
                        },
                        known_profile: None,
                        installed: None,
                        replaced: None,
                    });
                }
            }
        }

        // ── 10. PID settings (log only) ─────────────────────────
        for (heater, _pids) in &config.pid_settings {
            tracing::info!(heater = %heater, "PID settings found in printer config");
        }

        // Re-derive capabilities and apply library matches.
        profile.capabilities = CapabilityEngine::derive(&profile.hardware);
        self.apply_library_to_hardware(&mut profile.hardware);
        ConfidenceEngine::calibrate(profile);
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

fn parse_driver_chip(name: &str) -> DriverChip {
    let lower = name.to_lowercase();
    if lower.starts_with("tmc2209") { DriverChip::TMC2209 }
    else if lower.starts_with("tmc2208") { DriverChip::TMC2208 }
    else if lower.starts_with("tmc2225") { DriverChip::TMC2225 }
    else if lower.starts_with("tmc2226") { DriverChip::TMC2226 }
    else if lower.starts_with("tmc5160") { DriverChip::TMC5160 }
    else if lower.starts_with("tmc5161") { DriverChip::TMC5161 }
    else if lower.starts_with("tmc2130") { DriverChip::TMC2130 }
    else if lower.starts_with("tmc2240") { DriverChip::TMC2240 }
    else if lower.starts_with("a4988") { DriverChip::A4988 }
    else if lower.starts_with("drv8825") { DriverChip::DRV8825 }
    else if lower.starts_with("lv8729") { DriverChip::LV8729 }
    else { DriverChip::Unknown }
}

fn parse_sensor_type(name: &str) -> SensorType {
    let lower = name.to_lowercase();
    if lower.contains("104gt") || lower.contains("atc semitec") { SensorType::NTC100K }
    else if lower.contains("3950") { SensorType::NTC3950 }
    else if lower.contains("pt1000") { SensorType::PT1000 }
    else if lower.contains("pt100") { SensorType::PT100 }
    else if lower.contains("ad8495") { SensorType::AD8495 }
    else if lower.contains("max31865") { SensorType::MAX31865 }
    else if lower.contains("max31855") { SensorType::MAX31855 }
    else if lower.contains("max31856") { SensorType::MAX31856 }
    else { SensorType::Unknown }
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

    #[test]
    fn enrich_from_config_adds_stepper_driver() {
        let builder = MachineProfileBuilder::new();
        let mut profile = builder.build("p-cfg", None, None, None);

        // Give the profile a motion system with axes.
        profile.hardware.motion_system = Some(MotionSystem {
            kind: Property::assumed(MachineType::CoreXY),
            axes: vec![
                Axis { name: "x".into(), rotation_distance: None, microsteps: None, driver: None, endstop: None, rails: None, max_position: None, min_position: None },
                Axis { name: "y".into(), rotation_distance: None, microsteps: None, driver: None, endstop: None, rails: None, max_position: None, min_position: None },
                Axis { name: "z".into(), rotation_distance: None, microsteps: None, driver: None, endstop: None, rails: None, max_position: None, min_position: None },
            ],
            build_volume: None,
            max_velocity: None, max_acceleration: None, max_z_velocity: None, square_corner_velocity: None,
        });

        let text = "[tmc2209 stepper_x]\nuart_pin: PC14\n[tmc2209 stepper_y]\nuart_pin: PC15\n[stepper_x]\nendstop_pin: ^!PC0\n[extruder]\nsensor_type: ATC Semitec 104GT-2\nnozzle_diameter: 0.400\n[adxl345]\ncs_pin: PC13\n[probe]\npin: ^!PD3\n[input_shaper]\nshaper_type_x: mzv\n[fan]\npin: PA0\n[heater_fan hotend_fan]\npin: PA1\n[heater_bed]\nheater_pin: PA2\nsensor_type: Generic 3950";
        let config = crate::config_parser::parse_config(text);

        builder.enrich_from_config(&mut profile, &config);

        // Stepper drivers set.
        let axes = &profile.hardware.motion_system.as_ref().unwrap().axes;
        assert!(axes[0].driver.is_some());
        assert_eq!(axes[0].driver.as_ref().unwrap().chip.value, DriverChip::TMC2209);
        assert!(axes[1].driver.is_some());
        assert!(axes[2].driver.is_none());

        // Endstop set for x.
        assert!(axes[0].endstop.is_some());
        assert!(axes[1].endstop.is_none());

        // Probe added.
        assert_eq!(profile.hardware.probes.len(), 1);

        // Accelerometer added.
        assert_eq!(profile.hardware.accelerometers.len(), 1);

        // Input shaping capability.
        assert!(profile.capabilities.supports_input_shaping.value);

        // Fans added (2: fan + heater_fan).
        assert!(profile.hardware.cooling.len() >= 2);

        // Thermistors added.
        assert!(profile.hardware.thermistors.len() >= 2);

        // Heater added for heater_bed.
        assert!(profile.hardware.heaters.len() >= 1);
    }
}
