//! Hardware Profile Library — compiled-in database of known
//! printer components and their capabilities.
//!
//! Each profile defines match keywords, known capabilities, and
//! limits. The discovery engine uses these profiles to identify
//! hardware by name match.
//!
//! No networking. Profiles are compiled into the binary. Future
//! phases may support downloadable profile packs.

use layermind_shared::machine::*;

/// A static library of known hardware profiles.
#[derive(Debug)]
pub struct HardwareLibrary {
    profiles: Vec<HardwareProfile>,
}

impl HardwareLibrary {
    /// Build the library with all bundled profiles.
    pub fn new() -> Self {
        Self {
            profiles: Self::bundled_profiles(),
        }
    }

    /// Search for profiles matching the given name.
    pub fn find(&self, name: &str) -> Vec<&HardwareProfile> {
        self.profiles
            .iter()
            .filter(|p| {
                p.match_keywords
                    .iter()
                    .any(|k| name.to_lowercase().contains(&k.to_lowercase()))
            })
            .collect()
    }

    /// Try to match a component name and set its `known_profile`.
    /// Returns the matched profile name if any.
    pub fn match_component<'a>(&self, name: &str, known_profile: &'a mut Option<String>) -> Option<&'a str> {
        if known_profile.is_some() {
            return known_profile.as_deref();
        }
        if let Some(profile) = self.find(name).first() {
            *known_profile = Some(profile.name.clone());
            known_profile.as_deref()
        } else {
            None
        }
    }

    /// Return all profiles for a given category.
    pub fn by_category(&self, category: HardwareCategory) -> Vec<&HardwareProfile> {
        self.profiles
            .iter()
            .filter(|p| p.category == category)
            .collect()
    }

    // ── Bundled profiles ───────────────────────────────────

    fn bundled_profiles() -> Vec<HardwareProfile> {
        vec![
            // ── Probe profiles ─────────────────────────────
            Self::probe(
                "BLTouch",
                "Antclabs",
                vec!["bltouch"],
                vec![
                    ("supports_bltouch", true, 1.0),
                    ("supports_input_shaping", false, 1.0),
                ],
            ),
            Self::probe(
                "CRTouch",
                "Creality",
                vec!["crtouch"],
                vec![("supports_bltouch", true, 1.0)],
            ),
            Self::probe(
                "Beacon",
                "Beacon3D",
                vec!["beacon"],
                vec![
                    ("supports_beacon", true, 1.0),
                    ("supports_input_shaping", true, 0.9),
                    ("supports_eddy_current", true, 1.0),
                ],
            ),
            Self::probe(
                "Cartographer",
                "Cartographer3D",
                vec!["cartographer"],
                vec![
                    ("supports_cartographer", true, 1.0),
                    ("supports_input_shaping", true, 0.9),
                ],
            ),
            // ── Driver profiles ────────────────────────────
            Self::driver(
                "TMC2209",
                "Trinamic",
                vec!["tmc2209", "2209"],
                vec![
                    ("supports_pressure_advance", true, 1.0),
                    ("supports_sensorless_homing", true, 1.0),
                    ("supports_stealthchop", true, 1.0),
                ],
            ),
            Self::driver(
                "TMC5160",
                "Trinamic",
                vec!["tmc5160", "5160"],
                vec![
                    ("supports_pressure_advance", true, 1.0),
                    ("supports_sensorless_homing", true, 1.0),
                    ("supports_spreadcycle", true, 1.0),
                ],
            ),
            // ── Hotend profiles ────────────────────────────
            Self::hotend(
                "Dragon HF",
                "Phaetus",
                vec!["dragon hf", "dragonhf", "dragon_hf"],
                vec![
                    ("supports_high_temperature", true, 1.0),
                    ("maximum_temperature_500", true, 1.0),
                ],
            ),
            Self::hotend(
                "Rapido",
                "Phaetus",
                vec!["rapido"],
                vec![
                    ("supports_high_temperature", true, 1.0),
                    ("maximum_temperature_350", true, 1.0),
                ],
            ),
            Self::hotend(
                "Revo",
                "E3D",
                vec!["revo"],
                vec![
                    ("supports_nozzle_swap", true, 1.0),
                    ("maximum_temperature_300", true, 1.0),
                ],
            ),
            // ── Extruder profiles ──────────────────────────
            Self::extruder(
                "Orbiter v2",
                "Orbiter Projects",
                vec!["orbiter"],
                vec![("max_flow_rate_30", true, 0.9)],
            ),
            // ── Accelerometer profiles ─────────────────────
            Self::accel(
                "ADXL345",
                "Analog Devices",
                vec!["adxl345", "adxl"],
                vec![("supports_input_shaping", true, 1.0)],
            ),
            Self::accel(
                "LIS2DW",
                "STMicroelectronics",
                vec!["lis2dw"],
                vec![("supports_input_shaping", true, 1.0)],
            ),
        ]
    }

    fn probe(
        name: &str,
        mfr: &str,
        keywords: Vec<&str>,
        caps: Vec<(&str, bool, f64)>,
    ) -> HardwareProfile {
        Self::profile(name, mfr, HardwareCategory::Probe, keywords, caps)
    }

    fn driver(
        name: &str,
        mfr: &str,
        keywords: Vec<&str>,
        caps: Vec<(&str, bool, f64)>,
    ) -> HardwareProfile {
        Self::profile(name, mfr, HardwareCategory::DriverChip, keywords, caps)
    }

    fn hotend(
        name: &str,
        mfr: &str,
        keywords: Vec<&str>,
        caps: Vec<(&str, bool, f64)>,
    ) -> HardwareProfile {
        Self::profile(name, mfr, HardwareCategory::Hotend, keywords, caps)
    }

    fn extruder(
        name: &str,
        mfr: &str,
        keywords: Vec<&str>,
        caps: Vec<(&str, bool, f64)>,
    ) -> HardwareProfile {
        Self::profile(name, mfr, HardwareCategory::Extruder, keywords, caps)
    }

    fn accel(
        name: &str,
        mfr: &str,
        keywords: Vec<&str>,
        caps: Vec<(&str, bool, f64)>,
    ) -> HardwareProfile {
        Self::profile(name, mfr, HardwareCategory::Accelerometer, keywords, caps)
    }

    fn profile(
        name: &str,
        mfr: &str,
        category: HardwareCategory,
        keywords: Vec<&str>,
        caps: Vec<(&str, bool, f64)>,
    ) -> HardwareProfile {
        HardwareProfile {
            name: name.into(),
            manufacturer: mfr.into(),
            category,
            match_keywords: keywords.into_iter().map(String::from).collect(),
            known_capabilities: caps
                .into_iter()
                .map(|(c, s, conf)| CapabilityHint {
                    capability: c.into(),
                    supported: s,
                    confidence: conf,
                })
                .collect(),
        }
    }
}

impl Default for HardwareLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bltouch_by_keyword() {
        let lib = HardwareLibrary::new();
        let matches = lib.find("Sensor: BLTouch v3.1");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].name, "BLTouch");
    }

    #[test]
    fn finds_tmc2209() {
        let lib = HardwareLibrary::new();
        let matches = lib.find("tmc2209 stepper_x");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].category, HardwareCategory::DriverChip);
    }

    #[test]
    fn finds_beacon() {
        let lib = HardwareLibrary::new();
        let matches = lib.find("beacon");
        assert!(!matches.is_empty());
        let caps = &matches[0].known_capabilities;
        assert!(caps.iter().any(|c| c.capability == "supports_beacon"));
    }

    #[test]
    fn no_match_for_unknown() {
        let lib = HardwareLibrary::new();
        let matches = lib.find("nonexistent_hardware_xyz");
        assert!(matches.is_empty());
    }

    #[test]
    fn filters_by_category() {
        let lib = HardwareLibrary::new();
        let probes = lib.by_category(HardwareCategory::Probe);
        assert!(probes.len() >= 3);
        assert!(probes.iter().all(|p| p.category == HardwareCategory::Probe));
    }
}
