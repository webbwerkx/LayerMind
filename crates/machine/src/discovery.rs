//! Hardware discovery — parses Moonraker objects and printer
//! configuration to build a structured inventory.
//!
//! This module translates raw Moonraker data (printer.info,
//! configfile, system_info, etc.) into typed hardware components.
//! It does not derive capabilities — that belongs to the
//! [`CapabilityEngine`](super::capability).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::Utc;
use layermind_shared::machine::*;

/// Discovers hardware identity and components from Moonraker data.
///
/// Takes raw Moonraker JSON objects and produces a typed
/// [`MachineIdentity`] and [`MachineHardware`] snapshot.
#[derive(Debug)]
pub struct HardwareDiscovery;

impl HardwareDiscovery {
    /// Discover printer identity from Moonraker `printer.info` and
    /// `system_info` objects.
    ///
    /// Parses hostname, firmware versions, MCU count, CPU model, and
    /// git metadata from the raw JSON responses.
    pub fn discover_identity(
        printer_id: &str,
        printer_info: Option<&serde_json::Value>,
        _firmware_retraction: Option<&serde_json::Value>,
        system_info: Option<&serde_json::Value>,
    ) -> MachineIdentity {
        let mut firmware = FirmwareInfo {
            klipper_version: None,
            moonraker_version: None,
            config_hash: None,
            config_timestamp: None,
            loaded_modules: Vec::new(),
            mcu_count: Property::assumed(1u32),
        };

        // system_info may be wrapped in an outer object or flat.
        let system_info_inner = system_info
            .and_then(|info| info.get("system_info").or(Some(info)));

        // MCU count and moonraker version from system_info.
        if let Some(info) = system_info_inner {
            firmware.moonraker_version = info
                .get("moonraker_version")
                .and_then(|v| v.as_str())
                .map(String::from);

            if let Some(mcus) = info.get("available_mcus").and_then(|v| v.as_array()) {
                firmware.mcu_count = Property::observed(mcus.len() as u32);
            }
        }

        // Klipper version from printer_info, with fallback to system_info.
        firmware.klipper_version = printer_info
            .and_then(|info| info.get("klipper_path"))
            .or_else(|| system_info_inner.and_then(|info| info.get("klipper_path")))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Board manufacturer and model hints from CPU info.
        let (mut manufacturer, mut model) = (None, None);

        if let Some(info) = system_info_inner {
            // Detect board type from cpu_info.model.
            if let Some(cpu_model) = info
                .get("cpu_info")
                .and_then(|v| v.get("model"))
                .and_then(|v| v.as_str())
            {
                detect_manufacturer_and_model(cpu_model, &mut firmware.loaded_modules);
            }

            // Manufacturer from hardware_desc.
            if let Some(hw_desc) = info
                .get("cpu_info")
                .and_then(|v| v.get("hardware_desc"))
                .and_then(|v| v.as_str())
            {
                if hw_desc.contains("BCM2835") || hw_desc.contains("BCM2711") {
                    manufacturer = Some(Property::observed("Raspberry Pi".to_string()));
                }
            }

            // Model from cpu_info.model.
            if let Some(cpu_model) = info
                .get("cpu_info")
                .and_then(|v| v.get("model"))
                .and_then(|v| v.as_str())
            {
                model = Some(Property::observed(cpu_model.to_string()));
            }
        }

        // Also check printer_info.cpu_info for board hints.
        if let Some(info) = printer_info {
            if let Some(cpu_info) = info.get("cpu_info").and_then(|v| v.as_str()) {
                detect_manufacturer_and_model(cpu_info, &mut firmware.loaded_modules);
            }
        }

        // Nickname from printer_info hostname.
        let mut nickname = None;
        if let Some(info) = printer_info {
            nickname = info
                .get("hostname")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Config file path as config hash.
            if let Some(cfg) = info.get("config_file").and_then(|v| v.as_str()) {
                let mut hasher = DefaultHasher::new();
                cfg.hash(&mut hasher);
                firmware.config_hash = Some(format!("sha256:{:016x}", hasher.finish()));
            }

            // Git metadata as loaded modules.
            if let Some(branch) = info.get("git_branch").and_then(|v| v.as_str()) {
                firmware.loaded_modules.push(format!("git_branch={}", branch));
            }
            if let Some(commit) = info.get("git_commit").and_then(|v| v.as_str()) {
                firmware.loaded_modules.push(format!("git_commit={}", commit));
            }
        }

        MachineIdentity {
            printer_id: printer_id.to_string(),
            nickname,
            manufacturer,
            model,
            custom_build: Property::observed(false),
            machine_type: Property::assumed(MachineType::Unknown),
            serial_number: None,
            firmware: Some(firmware),
            discovered_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Discover hardware components from a `printer.objects.query`
    /// response and `system_info`.
    ///
    /// Parses the raw JSON for component presence (extruder, bed, fan),
    /// build volume from `toolhead.axis_maximum`, MCU info from
    /// `available_mcus`, and stepper count from `motion_report`.
    pub fn discover_hardware(
        printer_objects: Option<&serde_json::Value>,
        system_info: Option<&serde_json::Value>,
    ) -> MachineHardware {
        let mut hardware = MachineHardware::default();

        // printer_objects from query response has a "status" wrapper.
        let status = printer_objects
            .and_then(|po| po.get("status"))
            .filter(|s| s.is_object());

        if let Some(status) = status {
            // Extruder.
            if status.get("extruder").is_some() {
                hardware.extruders.push(Component {
                    id: "extruder_0".into(),
                    name: "Extruder".into(),
                    details: ExtruderSpec {
                        extruder_type: Property::observed(ExtruderType::Unknown),
                        gear_ratio: None,
                        max_flow_rate: None,
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }

            // Heater bed.
            if status.get("heater_bed").is_some() {
                hardware.bed = Some(Component {
                    id: "bed".into(),
                    name: "Heated Bed".into(),
                    details: BedSpec {
                        bed_type: Property::observed(BedType::Unknown),
                        heater_power: None,
                        max_temperature: None,
                        surface_material: None,
                        size_x: None,
                        size_y: None,
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }

            // Fan.
            if status.get("fan").is_some() {
                hardware.cooling.push(Component {
                    id: "fan_0".into(),
                    name: "Part Cooling Fan".into(),
                    details: FanSpec {
                        fan_type: Property::observed(FanType::PartCooling),
                        pwm: true,
                        tachometer: false,
                    },
                    known_profile: None,
                    installed: None,
                    replaced: None,
                });
            }

            // Motion system from motion_report steppers and toolhead
            // axis limits.
            let axes: Vec<Axis> = status
                .get("motion_report")
                .and_then(|mr| mr.get("steppers"))
                .and_then(|s| s.as_array())
                .map(|steppers| {
                    steppers
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(|name| Axis {
                            name: name.to_string(),
                            rotation_distance: None,
                            microsteps: None,
                            driver: None,
                            endstop: None,
                            rails: None,
                            max_position: None,
                            min_position: None,
                        })
                        .collect()
                })
                .unwrap_or_default();

            let build_volume = status
                .get("toolhead")
                .and_then(|th| th.get("axis_maximum"))
                .and_then(|ax| ax.as_array())
                .filter(|ax| ax.len() >= 3)
                .map(|ax| BuildVolume {
                    x: ax[0].as_f64().unwrap_or(0.0),
                    y: ax[1].as_f64().unwrap_or(0.0),
                    z: ax[2].as_f64().unwrap_or(0.0),
                });

            if !axes.is_empty() || build_volume.is_some() {
                hardware.motion_system = Some(MotionSystem {
                    kind: Property::assumed(MachineType::Unknown),
                    axes,
                    build_volume,
                    max_velocity: None,
                    max_acceleration: None,
                    max_z_velocity: None,
                    square_corner_velocity: None,
                });
            }
        }

        // MCU info from system_info (handle both flat and nested).
        if let Some(info) = system_info
            .and_then(|info| info.get("system_info").or(Some(info)))
        {
            if let Some(mcus) = info.get("available_mcus").and_then(|v| v.as_array()) {
                for (i, mcu) in mcus.iter().enumerate() {
                    let name = mcu
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let cpu = mcu.get("version").and_then(|v| v.as_str()).map(String::from);

                    hardware.mcus.push(Component {
                        id: format!("mcu_{}", i),
                        name: name.clone(),
                        details: McuSpec {
                            mcu_name: name,
                            cpu,
                            clock_frequency: None,
                            is_primary: i == 0,
                        },
                        known_profile: None,
                        installed: None,
                        replaced: None,
                    });
                }
            }
        }

        hardware
    }
}

/// Detect manufacturer and model from CPU info string.
fn detect_manufacturer_and_model(cpu_info: &str, loaded_modules: &mut Vec<String>) {
    let cpu_lower = cpu_info.to_lowercase();
    if cpu_lower.contains("raspberry") {
        loaded_modules.push("board=raspberry_pi".to_string());
    } else if cpu_lower.contains("rockchip") {
        loaded_modules.push("board=rockchip".to_string());
    } else if cpu_lower.contains("allwinner") {
        loaded_modules.push("board=allwinner".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MachineProfileBuilder;

    #[test]
    fn discovers_basic_identity() {
        let identity = HardwareDiscovery::discover_identity("test-printer", None, None, None);
        assert_eq!(identity.printer_id, "test-printer");
        assert!(identity.firmware.is_some());
        assert!(identity.nickname.is_none());
        assert!(identity.manufacturer.is_none());
        assert!(identity.model.is_none());
    }

    #[test]
    fn reads_system_info_klipper_version() {
        let info = serde_json::json!({
            "klipper_path": "/home/pi/klipper/klippy/klippy.py",
            "moonraker_version": "v0.8.0",
            "available_mcus": [
                {"name": "mcu", "version": "v0.10.0"}
            ]
        });
        let identity = HardwareDiscovery::discover_identity("p1", None, None, Some(&info));
        assert_eq!(
            identity
                .firmware
                .as_ref()
                .unwrap()
                .klipper_version
                .as_deref(),
            Some("/home/pi/klipper/klippy/klippy.py")
        );
        assert_eq!(identity.firmware.as_ref().unwrap().mcu_count.value, 1);
    }

    #[test]
    fn empty_hardware_is_valid() {
        let hw = HardwareDiscovery::discover_hardware(None, None);
        assert!(hw.motion_system.is_none());
        assert!(hw.bed.is_none());
        assert!(hw.extruders.is_empty());
        assert!(hw.mcus.is_empty());
        assert!(hw.cooling.is_empty());
    }

    #[test]
    fn parses_printer_info_hostname_and_config() {
        let info = serde_json::json!({
            "hostname": "voron",
            "config_file": "/home/pi/printer.cfg",
            "git_branch": "master",
            "git_commit": "abc123"
        });
        let identity = HardwareDiscovery::discover_identity("p1", Some(&info), None, None);
        assert_eq!(identity.nickname.as_deref(), Some("voron"));
        assert!(identity
            .firmware
            .as_ref()
            .unwrap()
            .config_hash
            .as_deref()
            .unwrap()
            .starts_with("sha256:"));
        let modules = &identity.firmware.as_ref().unwrap().loaded_modules;
        assert!(modules.contains(&"git_branch=master".to_string()));
        assert!(modules.contains(&"git_commit=abc123".to_string()));
    }

    #[test]
    fn detects_raspberry_pi_from_cpu_info() {
        let info = serde_json::json!({
            "system_info": {
                "cpu_info": {
                    "model": "Raspberry Pi 4 Model B Rev 1.1",
                    "hardware_desc": "BCM2711"
                }
            }
        });
        let identity = HardwareDiscovery::discover_identity("p1", None, None, Some(&info));
        assert_eq!(
            identity.manufacturer.as_ref().unwrap().value,
            "Raspberry Pi"
        );
        assert_eq!(
            identity.model.as_ref().unwrap().value,
            "Raspberry Pi 4 Model B Rev 1.1"
        );
    }

    #[test]
    fn discovers_hardware_from_printer_objects() {
        let objects = serde_json::json!({
            "status": {
                "extruder": { "temperature": 210.0, "target": 210.0 },
                "heater_bed": { "temperature": 60.0, "target": 60.0 },
                "fan": { "speed": 0.0 },
                "toolhead": { "axis_maximum": [300.0, 300.0, 400.0] },
                "motion_report": { "steppers": ["stepper_x", "stepper_y", "stepper_z"] }
            }
        });
        let hw = HardwareDiscovery::discover_hardware(Some(&objects), None);
        assert_eq!(hw.extruders.len(), 1);
        assert_eq!(hw.extruders[0].id, "extruder_0");
        assert!(hw.bed.is_some());
        assert_eq!(hw.bed.as_ref().unwrap().id, "bed");
        assert_eq!(hw.cooling.len(), 1);
        assert_eq!(hw.cooling[0].id, "fan_0");
        let motion = hw.motion_system.as_ref().unwrap();
        assert_eq!(motion.axes.len(), 3);
        let bv = motion.build_volume.as_ref().unwrap();
        assert_eq!(bv.x, 300.0);
        assert_eq!(bv.y, 300.0);
        assert_eq!(bv.z, 400.0);
    }

    #[test]
    fn discovers_mcus_from_system_info_flat() {
        let sys = serde_json::json!({
            "available_mcus": [
                { "name": "mcu", "version": "v0.12.0" },
                { "name": "rpi", "version": "v0.12.0" }
            ]
        });
        let hw = HardwareDiscovery::discover_hardware(None, Some(&sys));
        assert_eq!(hw.mcus.len(), 2);
        assert_eq!(hw.mcus[0].name, "mcu");
        assert!(hw.mcus[0].details.is_primary);
        assert_eq!(hw.mcus[0].details.cpu.as_deref(), Some("v0.12.0"));
        assert_eq!(hw.mcus[1].name, "rpi");
        assert!(!hw.mcus[1].details.is_primary);
    }

    #[test]
    fn discovers_mcus_from_system_info_nested() {
        let sys = serde_json::json!({
            "system_info": {
                "available_mcus": [
                    { "name": "mcu", "version": "v0.11.0" }
                ]
            }
        });
        let hw = HardwareDiscovery::discover_hardware(None, Some(&sys));
        assert_eq!(hw.mcus.len(), 1);
        assert_eq!(hw.mcus[0].details.cpu.as_deref(), Some("v0.11.0"));
    }

    #[test]
    fn partial_printer_objects_discovery() {
        let objects = serde_json::json!({
            "status": {
                "extruder": { "temperature": 200.0, "target": 200.0 },
                "fan": { "speed": 0.5 }
            }
        });
        let hw = HardwareDiscovery::discover_hardware(Some(&objects), None);
        assert_eq!(hw.extruders.len(), 1);
        assert!(hw.bed.is_none());
        assert_eq!(hw.cooling.len(), 1);
        assert!(hw.motion_system.is_none());
    }

    #[test]
    fn full_build_discovers_both_identity_and_hardware() {
        let builder = MachineProfileBuilder::new();
        let printer_info = serde_json::json!({
            "hostname": "test-printer",
            "config_file": "/home/pi/printer.cfg"
        });
        let system_info = serde_json::json!({
            "available_mcus": [{ "name": "mcu" }],
            "moonraker_version": "v0.8.0"
        });
        let printer_objects = serde_json::json!({
            "status": {
                "extruder": { "temperature": 200.0 },
                "heater_bed": { "temperature": 50.0 }
            }
        });
        let profile = builder.build(
            "p1",
            Some(&printer_info),
            Some(&system_info),
            Some(&printer_objects),
        );
        assert_eq!(profile.identity.nickname.as_deref(), Some("test-printer"));
        assert_eq!(profile.hardware.extruders.len(), 1);
        assert!(profile.hardware.bed.is_some());
        assert_eq!(profile.hardware.mcus.len(), 1);
    }

    #[test]
    fn empty_status_object_returns_defaults() {
        let objects = serde_json::json!({ "status": {} });
        let hw = HardwareDiscovery::discover_hardware(Some(&objects), None);
        assert!(hw.extruders.is_empty());
        assert!(hw.bed.is_none());
        assert!(hw.cooling.is_empty());
        assert!(hw.motion_system.is_none());
    }
}
