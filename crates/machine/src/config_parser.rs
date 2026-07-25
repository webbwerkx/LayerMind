//! Klipper printer config file parser.
//!
//! Parses `printer.cfg` (INI-like format with sections) to extract
//! hardware details that Moonraker's runtime API doesn't expose:
//! stepper drivers, sensor types, thermistors, probe models, etc.
//!
//! # Format
//!
//! ```text
//! [section_name]
//! key: value
//! key = value
//! ```
//!
//! Lines starting with `#` or `;` are comments. `[include path]`
//! directives reference other files.

use std::collections::HashMap;

/// Parsed printer configuration with extracted hardware details.
#[derive(Debug, Clone, Default)]
pub struct ParsedConfig {
    /// Raw section → key → value map (all sections, all keys).
    pub raw: HashMap<String, HashMap<String, String>>,

    // ── Extracted hardware details ──────────────────────────────

    /// Stepper drivers per axis (e.g., "stepper_x" → "tmc2209").
    pub stepper_drivers: HashMap<String, String>,

    /// Endstop pins per axis.
    pub endstops: HashMap<String, String>,

    /// Thermistor/sensor type for extruder and bed.
    pub sensor_types: HashMap<String, String>,

    /// Probe model (from [probe] or [bltouch] section).
    pub probe: Option<String>,

    /// Accelerometer model (from [adxl345] or [lis2dw]).
    pub accelerometer: Option<String>,

    /// Input shaper configuration.
    pub input_shaper: Option<String>,

    /// Microstepping values per axis.
    pub microsteps: HashMap<String, u32>,

    /// Rotation distance per axis.
    pub rotation_distance: HashMap<String, f64>,

    /// Max position per axis.
    pub position_max: HashMap<String, f64>,

    /// Nozzle diameter.
    pub nozzle_diameter: Option<f64>,

    /// Filament diameter.
    pub filament_diameter: Option<f64>,

    /// Heater pin assignments.
    pub heater_pins: HashMap<String, String>,

    /// Fan sections detected.
    pub fans: Vec<String>,

    /// Included config files.
    pub includes: Vec<String>,

    /// PID settings per heater.
    pub pid_settings: HashMap<String, HashMap<String, f64>>,
}

/// Parse the text content of a Klipper config file.
pub fn parse_config(text: &str) -> ParsedConfig {
    let mut config = ParsedConfig::default();
    parse_into(&mut config, text);
    config
}

/// Merge multiple parsed configs into a single combined config.
/// Later files overwrite earlier files for duplicate keys.
pub fn merge_configs(configs: impl IntoIterator<Item = ParsedConfig>) -> ParsedConfig {
    let mut merged = ParsedConfig::default();
    for config in configs {
        for (section, kv) in config.raw {
            let entry = merged.raw.entry(section).or_default();
            for (k, v) in kv {
                entry.insert(k, v);
            }
        }
        merged.stepper_drivers.extend(config.stepper_drivers);
        merged.endstops.extend(config.endstops);
        merged.sensor_types.extend(config.sensor_types);
        if config.probe.is_some() {
            merged.probe = config.probe;
        }
        if config.accelerometer.is_some() {
            merged.accelerometer = config.accelerometer;
        }
        if config.input_shaper.is_some() {
            merged.input_shaper = config.input_shaper;
        }
        merged.microsteps.extend(config.microsteps);
        merged.rotation_distance.extend(config.rotation_distance);
        merged.position_max.extend(config.position_max);
        if config.nozzle_diameter.is_some() {
            merged.nozzle_diameter = config.nozzle_diameter;
        }
        if config.filament_diameter.is_some() {
            merged.filament_diameter = config.filament_diameter;
        }
        merged.heater_pins.extend(config.heater_pins);
        for fan in config.fans {
            if !merged.fans.contains(&fan) {
                merged.fans.push(fan);
            }
        }
        merged.includes.extend(config.includes);
        for (heater, pids) in config.pid_settings {
            merged.pid_settings.entry(heater).or_default().extend(pids);
        }
    }
    merged
}

fn parse_into(config: &mut ParsedConfig, text: &str) {
    let mut current_section: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        // Include directive (must come before section header check).
        if let Some(rest) = trimmed.strip_prefix("[include ") {
            if let Some(end) = rest.find(']') {
                config.includes.push(rest[..end].trim().to_string());
            }
            continue;
        }

        // Section header.
        if trimmed.starts_with('[') {
            if let Some(end) = trimmed.find(']') {
                let name = trimmed[1..end].trim().to_string();
                current_section = Some(name.clone());
                config.raw.entry(name).or_default();
            }
            continue;
        }

        let section = match &current_section {
            Some(s) => s.clone(),
            None => continue,
        };

        // Key-value pair.
        if let Some((key, value)) = trimmed.split_once(&[':', '='][..]) {
            let key = key.trim().to_string();
            let value = value.trim().to_string();

            config.raw.entry(section.clone()).or_default().insert(key.clone(), value.clone());

            // Extract known hardware details.
            extract_hardware_detail(config, &section, &key, &value);
        }
    }
}

fn extract_hardware_detail(
    config: &mut ParsedConfig,
    section: &str,
    key: &str,
    value: &str,
) {
    match section {
        s if s.starts_with("stepper_") && key == "endstop_pin" => {
            config.endstops.insert(s.replace("stepper_", ""), value.to_string());
        }
        s if s.starts_with("tmc2209") || s.starts_with("tmc2225") || s.starts_with("tmc5160") || s.starts_with("tmc2130") || s.starts_with("a4988") || s.starts_with("drv8825") || s.starts_with("lv8729") || s.starts_with("st820") || s.starts_with("tmc2208") || s.starts_with("tmc2660") => {
            if key == "uart_pin" || key == "step_pin" || key == "cs_pin" {
                let driver_type = section.split_whitespace().next().unwrap_or(section);
                let axis = section.split_whitespace().nth(1).unwrap_or("");
                if !axis.is_empty() {
                    config.stepper_drivers.insert(axis.to_string(), driver_type.to_string());
                }
            }
        }
        s if s == "extruder" || s == "heater_bed" => {
            match key {
                "sensor_type" => { config.sensor_types.insert(s.to_string(), value.to_string()); }
                "heater_pin" => { config.heater_pins.insert(s.to_string(), value.to_string()); }
                "nozzle_diameter" => { config.nozzle_diameter = value.parse().ok(); }
                "filament_diameter" => { config.filament_diameter = value.parse().ok(); }
                "microsteps" => { config.microsteps.insert(s.to_string(), value.parse().unwrap_or(16)); }
                "rotation_distance" => { config.rotation_distance.insert(s.to_string(), value.parse().unwrap_or(0.0)); }
                "pid_Kp" | "pid_Ki" | "pid_Kd" => {
                    config.pid_settings.entry(s.to_string()).or_default().insert(key.to_string(), value.parse().unwrap_or(0.0));
                }
                _ => {}
            }
        }
        s if s == "probe" || s == "bltouch" => {
            if key == "pin" || key == "sensor_pin" {
                config.probe = Some(s.to_string());
            }
        }
        s if s == "adxl345" || s == "lis2dw" || s == "mpu9250" => {
            config.accelerometer = Some(s.to_string());
        }
        s if s == "input_shaper" && key == "shaper_type_x" => {
            config.input_shaper = Some(value.to_string());
        }
        s if s.starts_with("fan") || s.starts_with("heater_fan") || s.starts_with("controller_fan") || s.starts_with("exhaust_fan") => {
            if !config.fans.contains(&section.to_string()) {
                config.fans.push(section.to_string());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_config() {
        let text = r#"
[stepper_x]
step_pin: PB7
dir_pin: PC5
enable_pin: !PC6
microsteps: 16
rotation_distance: 40
endstop_pin: ^!PC0
position_endstop: 0
position_max: 300

[extruder]
step_pin: PB4
microsteps: 32
rotation_distance: 33
nozzle_diameter: 0.400
filament_diameter: 1.750
sensor_type: ATC Semitec 104GT-2
heater_pin: PA2
control: pid
pid_Kp: 22.2
pid_Ki: 1.08
pid_Kd: 114

[heater_bed]
heater_pin: PA1
sensor_type: Generic 3950
pid_Kp: 67.6
pid_Ki: 1.76
pid_Kd: 645

[tmc2209 stepper_x]
uart_pin: PC14
run_current: 0.800

[probe]
pin: ^!PD3
x_offset: -25.0

[adxl345]
cs_pin: PC13
spi_bus: spi1

[input_shaper]
shaper_type_x: mzv
shaper_type_y: mzv
"#;
        let config = parse_config(text);

        assert_eq!(config.stepper_drivers.get("stepper_x").unwrap(), "tmc2209");
        assert_eq!(config.endstops.get("x").unwrap(), "^!PC0");
        assert_eq!(config.sensor_types.get("extruder").unwrap(), "ATC Semitec 104GT-2");
        assert_eq!(config.sensor_types.get("heater_bed").unwrap(), "Generic 3950");
        assert_eq!(config.probe.as_deref(), Some("probe"));
        assert_eq!(config.accelerometer.as_deref(), Some("adxl345"));
        assert_eq!(config.input_shaper.as_deref(), Some("mzv"));
        assert_eq!(config.microsteps.get("extruder").unwrap(), &32);
        assert!((config.rotation_distance.get("extruder").unwrap() - 33.0).abs() < 0.01);
        assert!((config.nozzle_diameter.unwrap() - 0.4).abs() < 0.001);
        assert!((config.filament_diameter.unwrap() - 1.75).abs() < 0.001);
        assert!(config.pid_settings.contains_key("extruder"));
        let extruder_pid = &config.pid_settings["extruder"];
        assert!((extruder_pid["pid_Kp"] - 22.2).abs() < 0.01);
        assert_eq!(config.includes.len(), 0);
    }

    #[test]
    fn handles_empty_config() {
        let config = parse_config("");
        assert!(config.raw.is_empty());
        assert!(config.stepper_drivers.is_empty());
    }

    #[test]
    fn handles_comments_and_blank_lines() {
        let text = "# comment\n\n; another comment\n[stepper_x]\nstep_pin: PB7\n";
        let config = parse_config(text);
        assert_eq!(config.raw.get("stepper_x").unwrap().get("step_pin").unwrap(), "PB7");
    }

    #[test]
    fn detects_include_directives() {
        let text = "[include mainsail.cfg]\n[include macros/*.cfg]\n[stepper_x]\nstep_pin: PB7\n";
        let config = parse_config(text);
        assert_eq!(config.includes.len(), 2);
        assert!(config.includes.contains(&"mainsail.cfg".to_string()));
    }

    #[test]
    fn detects_fan_sections() {
        let text = "[fan]\npin: PA0\n[heater_fan my_fan]\npin: PA1\n";
        let config = parse_config(text);
        assert!(config.fans.contains(&"fan".to_string()));
        assert!(config.fans.contains(&"heater_fan my_fan".to_string()));
    }

    #[test]
    fn handles_bltouch_section() {
        let text = "[bltouch]\nsensor_pin: ^!PD3\n[stepper_z]\nendstop_pin: probe:z_virtual_endstop\n";
        let config = parse_config(text);
        assert_eq!(config.probe.as_deref(), Some("bltouch"));
    }

    #[test]
    fn parses_equals_sign_too() {
        let text = "[stepper_x]\nstep_pin = PB7\nmicrosteps: 16\n";
        let config = parse_config(text);
        assert_eq!(config.raw.get("stepper_x").unwrap().get("step_pin").unwrap(), "PB7");
        assert_eq!(config.raw.get("stepper_x").unwrap().get("microsteps").unwrap(), "16");
    }
}