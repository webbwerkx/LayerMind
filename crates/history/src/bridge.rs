//! Bridge between telemetry events and the timeline store.
//!
//! Maps canonical `Envelope` events from the printer pipeline into
//! `TimelineEvent` records suitable for the historical timeline.
//! High-frequency telemetry (temperature, position, speed, fan, progress)
//! is intentionally excluded to avoid flooding the timeline.

use layermind_shared::event::{Envelope, Event};
use layermind_shared::history::*;

/// Map an envelope to zero or more timeline events.
///
/// Returns an empty vec for high-frequency or non-significant events.
/// Only lifecycle-significant events become timeline records.
pub fn envelope_to_timeline(envelope: &Envelope) -> Vec<TimelineEvent> {
    let kind = match &envelope.payload {
        // --- Print lifecycle ---
        Event::PrintStarted {
            filename,
            estimated_time: _,
        } => TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Started,
            print_job_id: None,
            filename: Some(filename.clone()),
            duration_secs: None,
            failure_reason: None,
        }),
        Event::PrintPaused { reason } => TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Paused,
            print_job_id: None,
            filename: None,
            duration_secs: None,
            failure_reason: reason.clone(),
        }),
        Event::PrintResumed => TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Resumed,
            print_job_id: None,
            filename: None,
            duration_secs: None,
            failure_reason: None,
        }),
        Event::PrintCompleted {
            total_time,
            filament_used: _,
        } => TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Completed,
            print_job_id: None,
            filename: None,
            duration_secs: Some(*total_time),
            failure_reason: None,
        }),
        Event::PrintFailed { reason } => TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Failed,
            print_job_id: None,
            filename: None,
            duration_secs: None,
            failure_reason: reason.clone(),
        }),
        Event::PrintCancelled => TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Cancelled,
            print_job_id: None,
            filename: None,
            duration_secs: None,
            failure_reason: None,
        }),

        // --- Anomalies ---
        Event::HeaterFault { heater, message } => TimelineEventKind::Anomaly(AnomalyEvent {
            severity: AnomalySeverity::Error,
            anomaly_type: "heater_fault".into(),
            description: format!("Heater fault on {}: {}", heater, message),
            print_job_id: None,
        }),
        Event::Error { code, message } => TimelineEventKind::Anomaly(AnomalyEvent {
            severity: AnomalySeverity::Error,
            anomaly_type: code.clone().unwrap_or_else(|| "error".into()),
            description: message.clone(),
            print_job_id: None,
        }),
        Event::Warning { message } => TimelineEventKind::Anomaly(AnomalyEvent {
            severity: AnomalySeverity::Warning,
            anomaly_type: "warning".into(),
            description: message.clone(),
            print_job_id: None,
        }),

        // --- Lifecycle events ---
        Event::PrinterReady => TimelineEventKind::Configuration(ConfigurationEvent {
            action: ConfigAction::Reloaded,
            config_key: "printer.state".into(),
            previous_value: None,
            new_value: Some("ready".into()),
        }),
        Event::Connected => TimelineEventKind::Configuration(ConfigurationEvent {
            action: ConfigAction::Reloaded,
            config_key: "printer.state".into(),
            previous_value: None,
            new_value: Some("connected".into()),
        }),
        Event::StateChanged { state } => TimelineEventKind::Configuration(ConfigurationEvent {
            action: ConfigAction::Reloaded,
            config_key: "printer.state".into(),
            previous_value: None,
            new_value: Some(format!("{:?}", state)),
        }),
        Event::Disconnected { reason } => TimelineEventKind::Anomaly(AnomalyEvent {
            severity: AnomalySeverity::Warning,
            anomaly_type: "disconnected".into(),
            description: format!("Printer disconnected: {}", reason),
            print_job_id: None,
        }),

        // --- Skip: high-frequency telemetry ---
        Event::TemperatureUpdate { .. }
        | Event::PositionUpdate { .. }
        | Event::SpeedUpdate { .. }
        | Event::FanUpdate { .. }
        | Event::PrintProgress { .. }
        | Event::GcodeResponse { .. }
        | Event::Raw { .. } => return vec![],
    };

    let source = match &envelope.payload {
        Event::Error { .. } | Event::Warning { .. } | Event::HeaterFault { .. } => {
            TimelineEventSource::Automatic
        }
        _ => TimelineEventSource::Moonraker,
    };

    let confidence = match &envelope.payload {
        Event::Error { .. } | Event::Warning { .. } | Event::HeaterFault { .. } => 0.8,
        _ => 1.0,
    };

    vec![TimelineEvent {
        id: envelope.event_id.to_string(),
        printer_id: envelope.printer_id.clone(),
        timestamp: envelope.timestamp,
        kind,
        source,
        confidence,
        metadata: serde_json::json!({}),
    }]
}

/// Format a human-readable summary from a timeline event kind.
pub fn format_timeline_summary(kind: &TimelineEventKind) -> String {
    match kind {
        TimelineEventKind::PrintHistory(pe) => match pe.action {
            PrintAction::Started => format!(
                "Print started: {}",
                pe.filename.as_deref().unwrap_or("unknown")
            ),
            PrintAction::Completed => "Print completed".into(),
            PrintAction::Failed => format!(
                "Print failed: {}",
                pe.failure_reason.as_deref().unwrap_or("unknown")
            ),
            PrintAction::Cancelled => "Print cancelled".into(),
            PrintAction::Paused => "Print paused".into(),
            PrintAction::Resumed => "Print resumed".into(),
            _ => "Print event".into(),
        },
        TimelineEventKind::Anomaly(ae) => format!("[{}] {}", ae.anomaly_type, ae.description),
        TimelineEventKind::Configuration(ce) => format!("Config changed: {}", ce.config_key),
        _ => "Event recorded".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use layermind_shared::printer::PrinterState;
    use uuid::Uuid;

    fn envelope(event: Event) -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: "p1".into(),
            timestamp: Utc::now(),
            payload: event,
        }
    }

    #[test]
    fn print_started_maps_to_print_history() {
        let events = envelope_to_timeline(&envelope(Event::PrintStarted {
            filename: "benchy.gcode".into(),
            estimated_time: Some(3600.0),
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::PrintHistory(pe) => {
                assert_eq!(pe.action, PrintAction::Started);
                assert_eq!(pe.filename.as_deref(), Some("benchy.gcode"));
            }
            _ => panic!("expected PrintHistory event"),
        }
    }

    #[test]
    fn print_completed_maps_with_duration() {
        let events = envelope_to_timeline(&envelope(Event::PrintCompleted {
            total_time: 3540.0,
            filament_used: Some(5.2),
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::PrintHistory(pe) => {
                assert_eq!(pe.action, PrintAction::Completed);
                assert_eq!(pe.duration_secs, Some(3540.0));
            }
            _ => panic!("expected PrintHistory event"),
        }
    }

    #[test]
    fn print_failed_maps_with_reason() {
        let events = envelope_to_timeline(&envelope(Event::PrintFailed {
            reason: Some("thermal runaway".into()),
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::PrintHistory(pe) => {
                assert_eq!(pe.action, PrintAction::Failed);
                assert_eq!(pe.failure_reason.as_deref(), Some("thermal runaway"));
            }
            _ => panic!("expected PrintHistory event"),
        }
    }

    #[test]
    fn heater_fault_maps_to_anomaly() {
        let events = envelope_to_timeline(&envelope(Event::HeaterFault {
            heater: "extruder".into(),
            message: "temperature deviation".into(),
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::Anomaly(ae) => {
                assert_eq!(ae.severity, AnomalySeverity::Error);
                assert_eq!(ae.anomaly_type, "heater_fault");
            }
            _ => panic!("expected Anomaly event"),
        }
        assert_eq!(events[0].source, TimelineEventSource::Automatic);
        assert_eq!(events[0].confidence, 0.8);
    }

    #[test]
    fn temperature_update_is_skipped() {
        let events = envelope_to_timeline(&envelope(Event::TemperatureUpdate {
            temperatures: vec![],
        }));
        assert!(events.is_empty());
    }

    #[test]
    fn position_update_is_skipped() {
        let events = envelope_to_timeline(&envelope(Event::PositionUpdate {
            x: 100.0,
            y: 100.0,
            z: 10.0,
        }));
        assert!(events.is_empty());
    }

    #[test]
    fn fan_update_is_skipped() {
        let events = envelope_to_timeline(&envelope(Event::FanUpdate {
            name: "fan".into(),
            speed: 0.5,
            rpm: None,
        }));
        assert!(events.is_empty());
    }

    #[test]
    fn print_progress_is_skipped() {
        let events = envelope_to_timeline(&envelope(Event::PrintProgress {
            progress: 0.5,
            elapsed: 1800.0,
            remaining: Some(1800.0),
            current_layer: None,
            total_layers: None,
        }));
        assert!(events.is_empty());
    }

    #[test]
    fn error_maps_to_anomaly() {
        let events = envelope_to_timeline(&envelope(Event::Error {
            code: Some("MINTEMP".into()),
            message: "Extruder temp below minimum".into(),
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::Anomaly(ae) => {
                assert_eq!(ae.severity, AnomalySeverity::Error);
                assert_eq!(ae.anomaly_type, "MINTEMP");
            }
            _ => panic!("expected Anomaly event"),
        }
    }

    #[test]
    fn warning_maps_to_anomaly() {
        let events = envelope_to_timeline(&envelope(Event::Warning {
            message: "PID deviation detected".into(),
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::Anomaly(ae) => {
                assert_eq!(ae.severity, AnomalySeverity::Warning);
                assert_eq!(ae.anomaly_type, "warning");
            }
            _ => panic!("expected Anomaly event"),
        }
    }

    #[test]
    fn connected_maps_to_configuration() {
        let events = envelope_to_timeline(&envelope(Event::Connected));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::Configuration(ce) => {
                assert_eq!(ce.new_value.as_deref(), Some("connected"));
            }
            _ => panic!("expected Configuration event"),
        }
    }

    #[test]
    fn state_changed_maps_to_configuration() {
        let events = envelope_to_timeline(&envelope(Event::StateChanged {
            state: PrinterState::Printing,
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::Configuration(ce) => {
                assert_eq!(ce.new_value.as_deref(), Some("Printing"));
            }
            _ => panic!("expected Configuration event"),
        }
    }

    #[test]
    fn disconnected_maps_to_anomaly() {
        let events = envelope_to_timeline(&envelope(Event::Disconnected {
            reason: "WebSocket closed".into(),
        }));
        assert_eq!(events.len(), 1);
        match &events[0].kind {
            TimelineEventKind::Anomaly(ae) => {
                assert_eq!(ae.severity, AnomalySeverity::Warning);
                assert_eq!(ae.anomaly_type, "disconnected");
            }
            _ => panic!("expected Anomaly event"),
        }
    }

    #[test]
    fn format_summary_print_started() {
        let kind = TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Started,
            print_job_id: None,
            filename: Some("benchy.gcode".into()),
            duration_secs: None,
            failure_reason: None,
        });
        assert_eq!(format_timeline_summary(&kind), "Print started: benchy.gcode");
    }

    #[test]
    fn format_summary_print_completed() {
        let kind = TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Completed,
            print_job_id: None,
            filename: None,
            duration_secs: Some(3540.0),
            failure_reason: None,
        });
        assert_eq!(format_timeline_summary(&kind), "Print completed");
    }

    #[test]
    fn format_summary_print_failed() {
        let kind = TimelineEventKind::PrintHistory(PrintHistoryEvent {
            action: PrintAction::Failed,
            print_job_id: None,
            filename: None,
            duration_secs: None,
            failure_reason: Some("thermal runaway".into()),
        });
        assert_eq!(
            format_timeline_summary(&kind),
            "Print failed: thermal runaway"
        );
    }

    #[test]
    fn format_summary_anomaly() {
        let kind = TimelineEventKind::Anomaly(AnomalyEvent {
            severity: AnomalySeverity::Warning,
            anomaly_type: "thermal".into(),
            description: "PID oscillation".into(),
            print_job_id: None,
        });
        assert_eq!(
            format_timeline_summary(&kind),
            "[thermal] PID oscillation"
        );
    }

    #[test]
    fn format_summary_configuration() {
        let kind = TimelineEventKind::Configuration(ConfigurationEvent {
            action: ConfigAction::Reloaded,
            config_key: "printer.state".into(),
            previous_value: None,
            new_value: Some("ready".into()),
        });
        assert_eq!(format_timeline_summary(&kind), "Config changed: printer.state");
    }
}
