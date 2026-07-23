//! Capability Engine — deterministic derivation of printer capabilities
//! from hardware inventory.
//!
//! Every capability in [`CapabilitySet`] is computed mechanically from
//! the [`MachineHardware`] components. No AI involved.

use layermind_shared::machine::*;

/// Derives a [`CapabilitySet`] from a hardware inventory.
///
/// Rules are additive: start from [`CapabilitySet::empty()`] (all
/// false, zero confidence) and flip to true with high confidence when
/// hardware evidence is found.
#[derive(Debug)]
pub struct CapabilityEngine;

impl CapabilityEngine {
    /// Derive all capabilities from the given hardware snapshot.
    pub fn derive(hardware: &MachineHardware) -> CapabilitySet {
        let mut caps = CapabilitySet::empty();

        // ── Input shaping ──────────────────────────────────
        let has_adxl = hardware
            .accelerometers
            .iter()
            .any(|a| matches!(a.details.chip.value, AccelChip::ADXL345));
        let has_lis = hardware
            .accelerometers
            .iter()
            .any(|a| matches!(a.details.chip.value, AccelChip::LIS2DW));

        if has_adxl || has_lis {
            caps.supports_input_shaping = Property::inferred(true, "accelerometer present");
        }
        if has_adxl {
            caps.supports_adxl = Property::observed(true);
        }
        if has_lis {
            caps.supports_lis2dw = Property::observed(true);
        }

        // ── Pressure advance ───────────────────────────────
        // All Klipper printers running UART/SPI drivers support it.
        let has_uart_spi = hardware
            .motion_system
            .as_ref()
            .map(|m| {
                m.axes.iter().any(|ax| {
                    ax.driver
                        .as_ref()
                        .map(|d| d.chip.value.uart_capable() || d.chip.value.spi_capable())
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if has_uart_spi {
            caps.supports_pressure_advance = Property::inferred(true, "UART/SPI driver present");
        }

        // ── Multi-material / toolchangers ──────────────────
        caps.supports_multi_material =
            Property::observed(hardware.extruders.len() > 1 || hardware.hotends.len() > 1);
        caps.supports_toolchanger = Property::observed(!hardware.tool_changers.is_empty());

        // ── Sensorless homing ──────────────────────────────
        let sensorless = hardware
            .motion_system
            .as_ref()
            .map(|m| {
                m.axes.iter().any(|ax| match &ax.endstop {
                    Some(EndstopType::Sensorless) => true,
                    _ => ax
                        .driver
                        .as_ref()
                        .map(|d| d.chip.value.sensorless_homing_capable())
                        .unwrap_or(false),
                })
            })
            .unwrap_or(false);
        caps.supports_sensorless_homing =
            Property::inferred(sensorless, "sensorless homing capable driver");

        // ── Probes ─────────────────────────────────────────
        let probe_types: Vec<ProbeType> = hardware
            .probes
            .iter()
            .map(|p| p.details.probe_type.value)
            .collect();

        caps.supports_bltouch = Property::observed(matches_any(
            &probe_types,
            &[ProbeType::BlTouch, ProbeType::CrTouch],
        ));
        caps.supports_beacon = Property::observed(probe_types.contains(&ProbeType::Beacon));
        caps.supports_cartographer =
            Property::observed(probe_types.contains(&ProbeType::Cartographer));
        caps.supports_eddy_current =
            Property::observed(probe_types.contains(&ProbeType::EddyCurrent));
        caps.supports_load_cell = Property::observed(!hardware.load_cells.is_empty());

        // ── Connectivity ───────────────────────────────────
        if let Some(ref board) = hardware.control_board {
            caps.supports_canbus =
                Property::observed(board.details.has_canbus || !hardware.can_devices.is_empty());
            caps.supports_wifi = Property::observed(board.details.has_wifi);
            caps.supports_ethernet = Property::observed(board.details.has_ethernet);
        }

        caps.supports_remote_updates = Property::inferred(
            true,
            "all Klipper/Moonraker printers support remote updates",
        );

        // ── Monitoring ─────────────────────────────────────
        caps.supports_filament_sensor = Property::observed(!hardware.filament_sensors.is_empty());
        caps.supports_encosure = Property::observed(hardware.enclosure.is_some());

        // ── Multiple MCUs ──────────────────────────────────
        caps.supports_multiple_mcus = Property::observed(hardware.mcus.len() > 1);

        // ── Performance limits ─────────────────────────────
        // Collect max temperatures across all hotends and bed.
        let max_temp: f64 = hardware
            .hotends
            .iter()
            .map(|h| h.details.max_temperature.value)
            .chain(
                hardware
                    .bed
                    .iter()
                    .filter_map(|b| b.details.max_temperature.as_ref().map(|t| t.value)),
            )
            .fold(0.0_f64, f64::max);

        caps.maximum_temperature = Property::inferred(max_temp, "hardware specifications");

        // Collect velocity/acceleration from motion system.
        if let Some(ref motion) = hardware.motion_system {
            if let Some(ref v) = motion.max_velocity {
                caps.maximum_velocity = v.clone();
            }
            if let Some(ref a) = motion.max_acceleration {
                caps.maximum_acceleration = a.clone();
            }
        }

        // Stepper / heater slots from control board.
        if let Some(ref board) = hardware.control_board {
            caps.maximum_steppers = Property::observed(board.details.stepper_slots);
            caps.maximum_heaters = Property::observed(board.details.heater_slots);
        }

        // ── High temperature ───────────────────────────────
        caps.supports_high_temperature =
            Property::inferred(max_temp > 300.0, "max temperature exceeds 300°C");

        // ── Chamber heating ────────────────────────────────
        caps.supports_chamber_heating = Property::observed(
            hardware
                .enclosure
                .as_ref()
                .map(|e| e.details.has_heater)
                .unwrap_or(false),
        );

        // ── Motion system capabilities ─────────────────────
        if let Some(ref motion) = hardware.motion_system {
            caps.supported_motion_systems = Property::observed(vec![motion.kind.value]);
        }

        // Collect all driver modes.
        let modes: Vec<DriverMode> = hardware
            .motion_system
            .iter()
            .flat_map(|m| m.axes.iter())
            .filter_map(|ax| ax.driver.as_ref().map(|d| d.mode.value))
            .collect();
        if !modes.is_empty() {
            caps.supported_driver_modes = Property::observed(modes);
        }

        caps
    }
}

fn matches_any<T: PartialEq>(items: &[T], candidates: &[T]) -> bool {
    items.iter().any(|i| candidates.contains(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_hardware() -> MachineHardware {
        MachineHardware {
            motion_system: Some(MotionSystem {
                kind: Property::observed(MachineType::CoreXY),
                axes: vec![Axis {
                    name: "x".into(),
                    rotation_distance: None,
                    microsteps: None,
                    driver: Some(Driver {
                        chip: Property::observed(DriverChip::TMC2209),
                        mode: Property::observed(DriverMode::StealthChop),
                        current: None,
                        address: None,
                    }),
                    endstop: None,
                    rails: None,
                    max_position: None,
                    min_position: None,
                }],
                build_volume: Some(BuildVolume {
                    x: 250.0,
                    y: 250.0,
                    z: 250.0,
                }),
                max_velocity: Some(Property::observed(300.0)),
                max_acceleration: Some(Property::observed(3000.0)),
                max_z_velocity: None,
                square_corner_velocity: None,
            }),
            hotends: vec![Component {
                id: "h0".into(),
                name: "Hotend".into(),
                details: HotendSpec {
                    hotend_type: Property::observed(HotendType::Standard),
                    max_temperature: Property::observed(260.0),
                    heater_power: None,
                    nozzle_diameter: None,
                    pt1000: Property::observed(false),
                    thermocouple: Property::observed(false),
                },
                known_profile: None,
                installed: None,
                replaced: None,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn empty_hardware_all_false() {
        let caps = CapabilityEngine::derive(&MachineHardware::default());
        assert!(!caps.supports_input_shaping.value);
        assert!(!caps.supports_pressure_advance.value);
        assert!(!caps.supports_sensorless_homing.value);
        assert!(!caps.supports_bltouch.value);
        assert!(!caps.supports_toolchanger.value);
        assert_eq!(caps.supports_canbus.confidence, 0.0);
    }

    #[test]
    fn derives_pressure_advance_from_tmc2209() {
        let caps = CapabilityEngine::derive(&minimal_hardware());
        assert!(caps.supports_pressure_advance.value);
        assert_eq!(
            caps.supports_pressure_advance.source,
            InformationSource::DeterministicInference
        );
    }

    #[test]
    fn derives_motion_capabilities() {
        let caps = CapabilityEngine::derive(&minimal_hardware());
        assert!(caps
            .supported_motion_systems
            .value
            .contains(&MachineType::CoreXY));
        assert_eq!(caps.maximum_velocity.value, 300.0);
        assert_eq!(caps.maximum_acceleration.value, 3000.0);
    }

    #[test]
    fn derives_sensorless_homing_from_tmc2209() {
        // TMC2209 supports sensorless homing; no explicit endstop needed.
        let caps = CapabilityEngine::derive(&minimal_hardware());
        assert!(caps.supports_sensorless_homing.value);
    }

    #[test]
    fn detects_bltouch_probe() {
        let mut hw = minimal_hardware();
        hw.probes = vec![Component {
            id: "p0".into(),
            name: "BLTouch".into(),
            details: ProbeSpec {
                probe_type: Property::observed(ProbeType::BlTouch),
                uses_endstop_pin: true,
                dockable: false,
            },
            known_profile: None,
            installed: None,
            replaced: None,
        }];
        let caps = CapabilityEngine::derive(&hw);
        assert!(caps.supports_bltouch.value);
        assert!(!caps.supports_beacon.value);
    }

    #[test]
    fn beacon_probe_and_bed_mesh() {
        let mut hw = minimal_hardware();
        hw.probes = vec![Component {
            id: "b1".into(),
            name: "Beacon".into(),
            details: ProbeSpec {
                probe_type: Property::observed(ProbeType::Beacon),
                uses_endstop_pin: false,
                dockable: false,
            },
            known_profile: None,
            installed: None,
            replaced: None,
        }];
        let caps = CapabilityEngine::derive(&hw);
        assert!(caps.supports_beacon.value);
    }

    #[test]
    fn high_temperature_from_hotend() {
        let mut hw = minimal_hardware();
        hw.hotends[0].details.max_temperature = Property::observed(350.0);
        let caps = CapabilityEngine::derive(&hw);
        assert!(caps.supports_high_temperature.value);
        assert_eq!(caps.maximum_temperature.value, 350.0);
    }
}
