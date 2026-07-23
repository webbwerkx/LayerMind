//! Hardware discovery — parses Moonraker objects and printer
//! configuration to build a structured inventory.
//!
//! This module translates raw Moonraker data (printer.info,
//! configfile, system_info, etc.) into typed hardware components.
//! It does not derive capabilities — that belongs to the
//! [`CapabilityEngine`](super::capability).

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
    /// `firmware_retraction` objects.
    pub fn discover_identity(
        printer_id: &str,
        _printer_info: Option<&serde_json::Value>,
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

        if let Some(info) = system_info {
            firmware.klipper_version = info
                .get("klipper_path")
                .and_then(|v| v.as_str())
                .map(String::from);
            firmware.moonraker_version = info
                .get("moonraker_version")
                .and_then(|v| v.as_str())
                .map(String::from);

            if let Some(mcus) = info.get("available_mcus").and_then(|v| v.as_array()) {
                firmware.mcu_count = Property::observed(mcus.len() as u32);
            }
        }

        MachineIdentity {
            printer_id: printer_id.to_string(),
            nickname: None,
            manufacturer: None,
            model: None,
            custom_build: Property::observed(false),
            machine_type: Property::assumed(MachineType::Unknown),
            serial_number: None,
            firmware: Some(firmware),
            discovered_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Discover hardware components from a `printer.objects` query
    /// response.
    pub fn discover_hardware(_printer_objects: Option<&serde_json::Value>) -> MachineHardware {
        // Start with an empty hardware snapshot. Real discovery will
        // populate this from Moonraker's `printer.objects.query` API.
        MachineHardware::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_basic_identity() {
        let identity = HardwareDiscovery::discover_identity("test-printer", None, None, None);
        assert_eq!(identity.printer_id, "test-printer");
        assert!(identity.firmware.is_some());
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
        let hw = HardwareDiscovery::discover_hardware(None);
        assert!(hw.motion_system.is_none());
        assert!(hw.bed.is_none());
        assert!(hw.extruders.is_empty());
    }
}
