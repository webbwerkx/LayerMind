//! Machine intelligence types — hardware identity, capabilities, and
//! confidence model.
//!
//! This module defines the canonical types for printer hardware
//! intelligence. Everything is deterministic, strongly typed, and
//! carries provenance metadata through [`Property<T>`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Property<T> — typed value with provenance ───────────────────────

/// A value with attached metadata about how certain LayerMind is about
/// it and where the information came from.
///
/// Every discovered property — from identity to capabilities — carries
/// a [`Property`], so the AI reasoning layer can distinguish observed
/// facts from inferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property<T> {
    pub value: T,
    pub source: InformationSource,
    /// 0.0 = complete guess, 1.0 = directly observed / manufacturer
    /// confirmed.
    pub confidence: f64,
}

impl<T> Property<T> {
    /// Create a new property with explicit source and confidence.
    pub fn new(value: T, source: InformationSource, confidence: f64) -> Self {
        Self {
            value,
            source,
            confidence,
        }
    }

    /// Create an observed property (directly from Moonraker objects).
    pub fn observed(value: T) -> Self {
        Self {
            value,
            source: InformationSource::Moonraker,
            confidence: 1.0,
        }
    }

    /// Create an inferred property (derived by the Capability Engine).
    pub fn inferred(value: T, _rules: &str) -> Self {
        Self {
            value,
            source: InformationSource::DeterministicInference,
            confidence: 0.85,
        }
    }

    /// Create a default-guess property (low confidence).
    pub fn assumed(value: T) -> Self {
        Self {
            value,
            source: InformationSource::FactoryDefaults,
            confidence: 0.3,
        }
    }
}

impl<T: Default> Default for Property<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            source: InformationSource::Unknown,
            confidence: 0.0,
        }
    }
}

// ── Information Source ──────────────────────────────────────────────

/// Where a piece of hardware information came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InformationSource {
    /// Directly from a Moonraker API object (e.g. `printer.info`).
    Moonraker,
    /// Parsed from the printer's `printer.cfg`.
    PrinterConfig,
    /// Parsed from an included configuration file.
    IncludedConfig,
    /// Matched against a known hardware profile in the component library.
    HardwareProfile,
    /// Loaded from the LayerMind database (previously persisted).
    Database,
    /// Explicitly provided by the user.
    User,
    /// Sensible default when nothing else is available.
    FactoryDefaults,
    /// Derived by deterministic rules in the Capability Engine.
    DeterministicInference,
    /// Source has not been determined.
    Unknown,
}

// ── Machine Identity ────────────────────────────────────────────────

/// Core printer identity — what is this machine?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineIdentity {
    /// Unique printer identifier (from the core telemetry pipeline).
    pub printer_id: String,
    /// User-assigned or Moonraker-reported nickname.
    pub nickname: Option<String>,
    /// Manufacturer name (e.g. "Creality", "Voron", "Prusa").
    pub manufacturer: Option<Property<String>>,
    /// Model name (e.g. "Ender 3 V2", "V2.4").
    pub model: Option<Property<String>>,
    /// True if this is a custom-built printer.
    pub custom_build: Property<bool>,
    /// The kinematic / motion architecture.
    pub machine_type: Property<MachineType>,
    /// Serial number from firmware or sticker.
    pub serial_number: Option<String>,
    /// Firmware information summary.
    pub firmware: Option<FirmwareInfo>,
    /// When this identity was first constructed.
    pub discovered_at: DateTime<Utc>,
    /// When this identity was last updated.
    pub updated_at: DateTime<Utc>,
}

// ── Firmware ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareInfo {
    pub klipper_version: Option<String>,
    pub moonraker_version: Option<String>,
    pub config_hash: Option<String>,
    pub config_timestamp: Option<DateTime<Utc>>,
    pub loaded_modules: Vec<String>,
    pub mcu_count: Property<u32>,
}

// ── Machine Type ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MachineType {
    Cartesian,
    CoreXY,
    CoreXZ,
    Delta,
    Scara,
    /// Used for toolchangers, hybrid architectures, etc.
    Custom,
    Unknown,
}

// ── Motion System ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionSystem {
    pub kind: Property<MachineType>,
    pub axes: Vec<Axis>,
    pub build_volume: Option<BuildVolume>,
    pub max_velocity: Option<Property<f64>>,
    pub max_acceleration: Option<Property<f64>>,
    pub max_z_velocity: Option<Property<f64>>,
    pub square_corner_velocity: Option<Property<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axis {
    /// "x", "y", "z", "a", "b", "c"
    pub name: String,
    pub rotation_distance: Option<Property<f64>>,
    pub microsteps: Option<Property<u32>>,
    pub driver: Option<Driver>,
    pub endstop: Option<EndstopType>,
    pub rails: Option<RailType>,
    pub max_position: Option<Property<f64>>,
    pub min_position: Option<Property<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildVolume {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// ── Drivers ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Driver {
    pub chip: Property<DriverChip>,
    pub mode: Property<DriverMode>,
    pub current: Option<Property<f64>>,
    pub address: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverChip {
    TMC2208,
    TMC2209,
    TMC2225,
    TMC2226,
    TMC2130,
    TMC5160,
    TMC5161,
    TMC2240,
    A4988,
    DRV8825,
    LV8729,
    Other,
    Unknown,
}

impl DriverChip {
    pub fn uart_capable(&self) -> bool {
        matches!(
            self,
            DriverChip::TMC2209
                | DriverChip::TMC2225
                | DriverChip::TMC2226
                | DriverChip::TMC2130
                | DriverChip::TMC5160
                | DriverChip::TMC5161
                | DriverChip::TMC2240
        )
    }

    pub fn spi_capable(&self) -> bool {
        matches!(
            self,
            DriverChip::TMC2130 | DriverChip::TMC5160 | DriverChip::TMC5161 | DriverChip::TMC2240
        )
    }

    pub fn sensorless_homing_capable(&self) -> bool {
        matches!(
            self,
            DriverChip::TMC2209
                | DriverChip::TMC2226
                | DriverChip::TMC2130
                | DriverChip::TMC5160
                | DriverChip::TMC5161
                | DriverChip::TMC2240
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverMode {
    StealthChop,
    SpreadCycle,
    Standstill,
    Unknown,
}

// ── Endstop & Rails ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndstopType {
    Mechanical,
    Optical,
    HallEffect,
    /// Sensorless homing via driver stall detection.
    Sensorless,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RailType {
    LinearRail,
    LinearRod,
    VSlot,
    Unknown,
}

// ── Hardware Components ─────────────────────────────────────────────

/// Generic wrapper for any hardware component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component<T> {
    /// Unique identifier within the printer (e.g. "extruder_0").
    pub id: String,
    /// Human-readable name (e.g. "Primary Extruder").
    pub name: String,
    pub details: T,
    /// Matched hardware library profile name, if any.
    pub known_profile: Option<String>,
    pub installed: Option<DateTime<Utc>>,
    pub replaced: Option<DateTime<Utc>>,
}

/// Control board (main MCU board).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlBoard {
    pub manufacturer: Option<String>,
    pub board_name: Option<String>,
    pub mcu: Option<String>,
    pub cpu: Option<String>,
    pub driver_chips: Vec<DriverChip>,
    pub stepper_slots: u32,
    pub heater_slots: u32,
    pub thermistor_slots: u32,
    pub fan_slots: u32,
    pub has_canbus: bool,
    pub has_wifi: bool,
    pub has_ethernet: bool,
    pub has_usb: bool,
    pub voltage: Option<f64>,
}

/// Extruder motor + mechanical assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtruderSpec {
    pub extruder_type: Property<ExtruderType>,
    pub gear_ratio: Option<Property<f64>>,
    pub max_flow_rate: Option<Property<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtruderType {
    DirectDrive,
    Bowden,
    Orbiter,
    Sherpa,
    Clockwork,
    Galileo,
    LGX,
    BMG,
    Other,
    Unknown,
}

/// Hotend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotendSpec {
    pub hotend_type: Property<HotendType>,
    pub max_temperature: Property<f64>,
    pub heater_power: Option<Property<f64>>,
    pub nozzle_diameter: Option<Property<f64>>,
    pub pt1000: Property<bool>,       // high-temp RTD sensor
    pub thermocouple: Property<bool>, // K-type, etc.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotendType {
    Standard,
    Volcano,
    Rapido,
    Dragon,
    DragonHF,
    Revo,
    CHC,
    CHT,
    Mosquito,
    Copperhead,
    Other,
    Unknown,
}

/// Heater element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaterSpec {
    pub power: Option<Property<f64>>,
    pub heater_type: Property<HeaterType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeaterType {
    Cartridge,
    PCB,
    SiliconePad,
    Other,
    Unknown,
}

/// Thermistor or temperature sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermistorSpec {
    pub sensor_type: Property<SensorType>,
    pub pull_up: Option<Property<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorType {
    NTC100K,
    NTC3950,
    PT100,
    PT1000,
    AD8495,
    MAX31865,
    MAX31855,
    MAX31856,
    Other,
    Unknown,
}

/// Bed platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedSpec {
    pub bed_type: Property<BedType>,
    pub heater_power: Option<Property<f64>>,
    pub max_temperature: Option<Property<f64>>,
    pub surface_material: Option<String>,
    pub size_x: Option<f64>,
    pub size_y: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BedType {
    PCB,
    Aluminum,
    /// AC silicone heater
    AC,
    Fixed,
    Removable,
    Unknown,
}

/// Probe (Z endstop or bed mesh sensor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSpec {
    pub probe_type: Property<ProbeType>,
    pub uses_endstop_pin: bool,
    pub dockable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    BlTouch,
    CrTouch,
    MicroProbe,
    Inductive,
    /// "Poor man's probe" — nozzle + switch.
    Manual,
    Klicky,
    Euclid,
    Beacon,
    Cartographer,
    EddyCurrent,
    Piezo,
    Other,
    Unknown,
}

/// Filament sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilamentSensorSpec {
    pub sensor_type: Property<FilamentSensorType>,
    pub detects_runout: bool,
    pub detects_jam: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilamentSensorType {
    MechanicalSwitch,
    Optical,
    Encoder,
    Other,
    Unknown,
}

/// Accelerometer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccelerometerSpec {
    pub chip: Property<AccelChip>,
    pub axes: u8,            // 1, 2, or 3
    pub mounted_on: String,  // "toolhead", "bed", etc.
    pub bus: Option<String>, // SPI / I2C bus
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccelChip {
    ADXL345,
    LIS2DW,
    LIS3DH,
    MPU6500,
    MPU9250,
    ICM20948,
    ADCAccel,
    Other,
    Unknown,
}

/// Load cell or strain gauge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadCellSpec {
    pub resolution: Option<String>,
    pub amplifier_type: Option<String>,
}

/// Network interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceSpec {
    pub interface_type: Property<NetworkType>,
    pub mac_address: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkType {
    Ethernet,
    WiFi,
    CANBus,
    USB,
    Other,
    Unknown,
}

/// Display / screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySpec {
    pub display_type: Property<DisplayType>,
    pub encoder: bool, // rotary encoder present
    pub touchscreen: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayType {
    LCD12864,
    OLED,
    TFT,
    HDMI,
    Nextion,
    Other,
    Unknown,
}

/// CAN bus device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanDeviceSpec {
    pub uuid: Option<String>,
    pub device_type: Property<CanDeviceType>,
    pub bus_speed: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanDeviceType {
    ToolheadBoard,
    ExpansionBoard,
    Other,
    Unknown,
}

/// Fan / cooling device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanSpec {
    pub fan_type: Property<FanType>,
    pub pwm: bool,
    pub tachometer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FanType {
    PartCooling,
    Hotend,
    Controller,
    Chamber,
    Filter,
    Other,
    Unknown,
}

/// Enclosure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnclosureSpec {
    pub has_heater: bool,
    pub has_filter: bool,
    pub has_thermistor: bool,
    pub max_temperature: Option<f64>,
}

/// Toolhead (complete assembly of hotend + extruder + probe + fan).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolheadSpec {
    pub hotend_id: Option<String>,
    pub extruder_id: Option<String>,
    pub probe_id: Option<String>,
    pub fan_ids: Vec<String>,
    pub accelerometer_id: Option<String>,
    pub can_address: Option<String>,
}

/// Tool changer mechanism.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChangerSpec {
    pub tool_count: Property<u32>,
    pub dock_positions: Vec<f64>,
    pub auto_calibration: bool,
}

/// MCU (microcontroller unit — could be main board or auxiliary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McuSpec {
    pub mcu_name: String,
    pub cpu: Option<String>,
    pub clock_frequency: Option<u64>,
    pub is_primary: bool,
}

// ── Machine Hardware (container) ────────────────────────────────────

/// Complete hardware inventory for a printer.
///
/// Every component is optional — not all printers have all hardware.
/// Use [`Component<T>`] to wrap detailed specs with provenance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MachineHardware {
    pub control_board: Option<Component<ControlBoard>>,
    pub mcus: Vec<Component<McuSpec>>,
    pub motion_system: Option<MotionSystem>,
    pub extruders: Vec<Component<ExtruderSpec>>,
    pub hotends: Vec<Component<HotendSpec>>,
    pub heaters: Vec<Component<HeaterSpec>>,
    pub thermistors: Vec<Component<ThermistorSpec>>,
    pub bed: Option<Component<BedSpec>>,
    pub probes: Vec<Component<ProbeSpec>>,
    pub filament_sensors: Vec<Component<FilamentSensorSpec>>,
    pub accelerometers: Vec<Component<AccelerometerSpec>>,
    pub load_cells: Vec<Component<LoadCellSpec>>,
    pub networking: Vec<Component<NetworkInterfaceSpec>>,
    pub displays: Vec<Component<DisplaySpec>>,
    pub can_devices: Vec<Component<CanDeviceSpec>>,
    pub cooling: Vec<Component<FanSpec>>,
    pub enclosure: Option<Component<EnclosureSpec>>,
    pub toolheads: Vec<Component<ToolheadSpec>>,
    pub tool_changers: Vec<Component<ToolChangerSpec>>,
}

// ── Capability Set ──────────────────────────────────────────────────

/// Derived capabilities — what this printer can actually do.
///
/// Every capability is a [`Property<bool>`] so the AI can distinguish
/// "definitely supports this" from "probably supports this" from
/// "definitely cannot do this."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    // ── Input shaping ──────
    pub supports_input_shaping: Property<bool>,
    pub supports_adxl: Property<bool>,
    pub supports_lis2dw: Property<bool>,

    // ── Advanced extrusion ─
    pub supports_pressure_advance: Property<bool>,
    pub supports_multi_material: Property<bool>,
    pub supports_toolchanger: Property<bool>,

    // ── Homing & probing ──
    pub supports_sensorless_homing: Property<bool>,
    pub supports_bltouch: Property<bool>,
    pub supports_beacon: Property<bool>,
    pub supports_cartographer: Property<bool>,
    pub supports_eddy_current: Property<bool>,
    pub supports_load_cell: Property<bool>,

    // ── Connectivity ───────
    pub supports_canbus: Property<bool>,
    pub supports_wifi: Property<bool>,
    pub supports_ethernet: Property<bool>,
    pub supports_remote_updates: Property<bool>,

    // ── Monitoring ─────────
    pub supports_filament_sensor: Property<bool>,
    pub supports_camera: Property<bool>,
    pub supports_enclosure: Property<bool>,

    // ── Performance limits ─
    pub maximum_temperature: Property<f64>,
    pub maximum_velocity: Property<f64>,
    pub maximum_acceleration: Property<f64>,
    pub maximum_steppers: Property<u32>,
    pub maximum_heaters: Property<u32>,

    // ── Motion ─────────────
    pub supported_motion_systems: Property<Vec<MachineType>>,
    pub supported_driver_modes: Property<Vec<DriverMode>>,
    pub supports_multiple_mcus: Property<bool>,
    pub supports_high_temperature: Property<bool>,

    // ── Enclosure ──────────
    pub supports_chamber_heating: Property<bool>,
}

impl CapabilitySet {
    /// Build an empty (all-false, zero-confidence) capability set as a
    /// starting point before the Capability Engine derives answers.
    pub fn empty() -> Self {
        Self {
            supports_input_shaping: Property::default(),
            supports_adxl: Property::default(),
            supports_lis2dw: Property::default(),
            supports_pressure_advance: Property::default(),
            supports_multi_material: Property::default(),
            supports_toolchanger: Property::default(),
            supports_sensorless_homing: Property::default(),
            supports_bltouch: Property::default(),
            supports_beacon: Property::default(),
            supports_cartographer: Property::default(),
            supports_eddy_current: Property::default(),
            supports_load_cell: Property::default(),
            supports_canbus: Property::default(),
            supports_wifi: Property::default(),
            supports_ethernet: Property::default(),
            supports_remote_updates: Property::default(),
            supports_filament_sensor: Property::default(),
            supports_camera: Property::default(),
            supports_enclosure: Property::default(),
            maximum_temperature: Property::default(),
            maximum_velocity: Property::default(),
            maximum_acceleration: Property::default(),
            maximum_steppers: Property::default(),
            maximum_heaters: Property::default(),
            supported_motion_systems: Property::default(),
            supported_driver_modes: Property::default(),
            supports_multiple_mcus: Property::default(),
            supports_high_temperature: Property::default(),
            supports_chamber_heating: Property::default(),
        }
    }
}

// ── Machine Profile (top-level) ─────────────────────────────────────

/// The complete machine intelligence snapshot for a printer.
///
/// Built by the `machine` crate; consumed by `context` and `reasoning`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineProfile {
    pub identity: MachineIdentity,
    pub hardware: MachineHardware,
    pub capabilities: CapabilitySet,
    /// When this profile was built.
    pub generated_at: DateTime<Utc>,
}

// ── Hardware Profile Library ────────────────────────────────────────

/// A known hardware profile from the component library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub name: String,
    pub manufacturer: String,
    pub category: HardwareCategory,
    /// Keywords to match against Moonraker config / hardware reports.
    pub match_keywords: Vec<String>,
    pub known_capabilities: Vec<CapabilityHint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareCategory {
    ControlBoard,
    DriverChip,
    Extruder,
    Hotend,
    Probe,
    Accelerometer,
    FilamentSensor,
    Mcu,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityHint {
    /// Capability name as it appears in CapabilitySet.
    pub capability: String,
    /// Whether this hardware profile is known to support it.
    pub supported: bool,
    /// Confidence when this profile matches.
    pub confidence: f64,
}

// ── Hardware History ────────────────────────────────────────────────

/// A hardware change event for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareChange {
    pub component_id: String,
    pub component_type: String,
    pub change_kind: HardwareChangeKind,
    pub previous_value: Option<String>,
    pub new_value: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareChangeKind {
    Installed,
    Replaced,
    Removed,
    Upgraded,
    Configured,
}

// ── Configuration Snapshot ──────────────────────────────────────────

/// A point-in-time snapshot of the printer's full configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationSnapshot {
    pub printer_id: String,
    pub timestamp: DateTime<Utc>,
    pub config_hash: String,
    pub profile_snapshot: Option<MachineProfile>,
}
