use chrono::{DateTime, Utc};
use layermind_config::Config;
use layermind_shared::machine::MachineProfile;
use layermind_shared::recommendation::ValidatedRecommendation;

/// Snapshot of printer state from Moonraker polling.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct PrinterSnapshot {
    pub hostname: Option<String>,
    pub klipper_version: Option<String>,
    pub moonraker_version: Option<String>,
    pub state: String,
    pub print_filename: Option<String>,
    pub print_progress: f64,
    pub print_elapsed: f64,
    pub print_remaining: Option<f64>,
    pub current_layer: Option<u32>,
    pub total_layers: Option<u32>,
    pub extruder_temp: f64,
    pub extruder_target: f64,
    pub bed_temp: f64,
    pub bed_target: f64,
    pub fan_speed: f64,
    pub position: [f64; 4],
    pub speed: f64,
    pub homed_axes: String,
}

/// A single event shown in the event timeline panel.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub message: String,
    pub level: EventLevel,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum EventLevel {
    Info,
    Warning,
    Error,
}

/// Which panel has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Printer,
    Temps,
    Events,
    Recs,
}

/// Overall application state for the TUI.
#[allow(dead_code)]
pub struct AppState {
    pub config: Config,
    pub printer: PrinterSnapshot,
    pub events: Vec<TimelineEntry>,
    pub focus: Focus,
    pub event_scroll: usize,
    pub connected: bool,
    pub connecting: bool,
    pub connection_error: Option<String>,
    pub last_refresh: Option<DateTime<Utc>>,
    pub diagnostic_result: Option<ValidatedRecommendation>,
    pub machine_profile: Option<MachineProfile>,
    pub show_machine: bool,
    pub running_diagnostic: bool,
    pub diagnostic_error: Option<String>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            printer: PrinterSnapshot::default(),
            events: Vec::new(),
            focus: Focus::Printer,
            event_scroll: 0,
            connected: false,
            connecting: false,
            connection_error: None,
            last_refresh: None,
            diagnostic_result: None,
            machine_profile: None,
            show_machine: false,
            running_diagnostic: false,
            diagnostic_error: None,
        }
    }

    pub fn add_event(&mut self, message: impl Into<String>, level: EventLevel) {
        let entry = TimelineEntry {
            message: message.into(),
            level,
        };
        self.events.push(entry);
        if self.events.len() > 100 {
            self.events.remove(0);
        }
        self.event_scroll = self.events.len().saturating_sub(1);
    }

    pub fn status_text(&self) -> &str {
        if self.connecting {
            "connecting"
        } else if !self.connected {
            "disconnected"
        } else {
            &self.printer.state
        }
    }
}
