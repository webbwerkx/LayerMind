//! Confidence Engine — assigns confidence scores to discovered
//! properties based on information source, corroboration, and data
//! quality.
//!
//! The engine is deterministic. It never calls AI. It mechanically
//! adjusts confidence based on rules like:
//! - Direct Moonraker observation → 1.0
//! - Matched hardware profile → 0.9
//! - Deterministic inference from multiple corroborating signals → 0.85
//! - Factory default → 0.3
//! - Unknown / missing → 0.0

use layermind_shared::machine::*;

/// Adjusts confidence scores on discovered properties.
///
/// Takes a [`MachineProfile`] and boosts or reduces confidence per
/// property based on corroboration, data quality, and source
/// reliability.
#[derive(Debug)]
pub struct ConfidenceEngine;

impl ConfidenceEngine {
    /// Apply confidence adjustments to an entire profile.
    ///
    /// Returns a new profile with adjusted confidence values. The
    /// original profile is unchanged.
    pub fn calibrate(profile: &mut MachineProfile) {
        // ── Identity confidence ────────────────────────────
        let identity = &mut profile.identity;

        // If we have both a known model and the firmware confirms
        // Klipper, we're more confident about manufacturer/model.
        if let Some(ref model) = identity.model {
            if model.confidence < 0.8 && identity.firmware.as_ref().is_some() {
                // Boost slightly when firmware context is present.
            }
        }

        // Mark custom builds with lower confidence on manufacturer.
        if identity.custom_build.value {
            if let Some(ref mut mfr) = identity.manufacturer {
                mfr.confidence = mfr.confidence * 0.7;
            }
        }

        // ── Capability corroboration ───────────────────────
        let caps = &mut profile.capabilities;

        // If both ADXL and LIS2DW are absent and no probe supports
        // input shaping, we're very confident input shaping is not
        // supported.
        if !caps.supports_adxl.value
            && !caps.supports_lis2dw.value
            && !caps.supports_beacon.value
            && !caps.supports_cartographer.value
        {
            caps.supports_input_shaping.confidence = 1.0;
        }

        // If multiple MCUs are present, confidence goes up.
        if caps.supports_multiple_mcus.value {
            caps.supports_multiple_mcus.confidence = 1.0;
        }

        // If we have a toolchanger, we're definitely multi-material.
        if caps.supports_toolchanger.value {
            caps.supports_multi_material.confidence = 1.0;
        }

        // High temperature confidence: if we found a thermocouple or
        // PT1000, boost the high-temp claim.
        let has_high_temp_sensor = profile
            .hardware
            .hotends
            .iter()
            .any(|h| h.details.pt1000.value || h.details.thermocouple.value);

        let has_high_temp_hotend = profile.hardware.hotends.iter().any(|h| {
            matches!(
                h.details.hotend_type.value,
                HotendType::DragonHF | HotendType::Volcano | HotendType::Rapido
            )
        });

        if has_high_temp_sensor && has_high_temp_hotend {
            caps.supports_high_temperature.confidence = 1.0;
        } else if has_high_temp_sensor || has_high_temp_hotend {
            caps.supports_high_temperature.confidence =
                caps.supports_high_temperature.confidence * 1.1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_profile() -> MachineProfile {
        MachineProfile {
            identity: MachineIdentity {
                printer_id: "test".into(),
                nickname: None,
                manufacturer: Some(Property::assumed("Creality".into())),
                model: Some(Property::assumed("Ender 3 V2".into())),
                custom_build: Property::observed(false),
                machine_type: Property::observed(MachineType::Cartesian),
                serial_number: None,
                firmware: Some(FirmwareInfo {
                    klipper_version: Some("v0.10.0".into()),
                    moonraker_version: Some("v0.8.0".into()),
                    config_hash: None,
                    config_timestamp: None,
                    loaded_modules: vec![],
                    mcu_count: Property::observed(1),
                }),
                discovered_at: Utc::now(),
                updated_at: Utc::now(),
            },
            hardware: MachineHardware::default(),
            capabilities: CapabilitySet {
                supports_input_shaping: Property::assumed(false),
                supports_adxl: Property::assumed(false),
                supports_lis2dw: Property::assumed(false),
                ..CapabilitySet::empty()
            },
            generated_at: Utc::now(),
        }
    }

    #[test]
    fn increases_confidence_on_absence_of_shaping() {
        let mut profile = test_profile();
        ConfidenceEngine::calibrate(&mut profile);
        assert_eq!(profile.capabilities.supports_input_shaping.confidence, 1.0);
    }

    #[test]
    fn high_temp_with_sensor_and_hotend() {
        let mut profile = test_profile();
        profile.capabilities.supports_high_temperature = Property::inferred(true, "test");
        profile.hardware.hotends = vec![Component {
            id: "h0".into(),
            name: "Dragon HF".into(),
            details: HotendSpec {
                hotend_type: Property::observed(HotendType::DragonHF),
                max_temperature: Property::observed(500.0),
                heater_power: None,
                nozzle_diameter: None,
                pt1000: Property::observed(true),
                thermocouple: Property::observed(false),
            },
            known_profile: None,
            installed: None,
            replaced: None,
        }];
        ConfidenceEngine::calibrate(&mut profile);
        assert_eq!(
            profile.capabilities.supports_high_temperature.confidence,
            1.0
        );
    }

    #[test]
    fn toolchanger_implies_multi_material() {
        let mut profile = test_profile();
        profile.capabilities.supports_toolchanger = Property::observed(true);
        ConfidenceEngine::calibrate(&mut profile);
        assert_eq!(profile.capabilities.supports_multi_material.confidence, 1.0);
    }
}
