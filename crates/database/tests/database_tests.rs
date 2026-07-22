//! Tests for the database layer.
//!
//! Unit tests exercise query construction, event type mapping, and
//! model serialization without requiring a database connection.
//!
//! Integration tests (gated behind `DATABASE_URL` env var) run against
//! a real PostgreSQL instance with migrations.

#[cfg(test)]
mod unit {
    use layermind_database::event_type_name;
    use layermind_shared::event::Event;

    #[test]
    fn event_type_names_are_snake_case() {
        assert_eq!(
            event_type_name(&Event::TemperatureUpdate {
                temperatures: vec![]
            }),
            "temperature_update"
        );
        assert_eq!(
            event_type_name(&Event::PrintStarted {
                filename: "test".into(),
                estimated_time: None
            }),
            "print_started"
        );
        assert_eq!(
            event_type_name(&Event::PrintCompleted {
                total_time: 100.0,
                filament_used: None
            }),
            "print_completed"
        );
        assert_eq!(event_type_name(&Event::Connected), "connected");
        assert_eq!(
            event_type_name(&Event::Disconnected {
                reason: "test".into()
            }),
            "disconnected"
        );
        assert_eq!(event_type_name(&Event::PrintCancelled), "print_cancelled");
        assert_eq!(
            event_type_name(&Event::PositionUpdate {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }),
            "position_update"
        );
    }

    #[test]
    fn all_event_variants_have_type_names() {
        let events = vec![
            Event::Connected,
            Event::Disconnected { reason: "".into() },
            Event::PrinterReady,
            Event::StateChanged {
                state: layermind_shared::printer::PrinterState::Idle,
            },
            Event::TemperatureUpdate {
                temperatures: vec![],
            },
            Event::HeaterFault {
                heater: "".into(),
                message: "".into(),
            },
            Event::PositionUpdate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Event::SpeedUpdate { speed: 0.0 },
            Event::FanUpdate {
                name: "".into(),
                speed: 0.0,
                rpm: None,
            },
            Event::PrintStarted {
                filename: "".into(),
                estimated_time: None,
            },
            Event::PrintProgress {
                progress: 0.0,
                elapsed: 0.0,
                remaining: None,
                current_layer: None,
                total_layers: None,
            },
            Event::PrintPaused { reason: None },
            Event::PrintResumed,
            Event::PrintCompleted {
                total_time: 0.0,
                filament_used: None,
            },
            Event::PrintFailed { reason: None },
            Event::PrintCancelled,
            Event::GcodeResponse {
                command: "".into(),
                response: "".into(),
            },
            Event::Error {
                code: None,
                message: "".into(),
            },
            Event::Warning { message: "".into() },
            Event::Raw {
                namespace: "".into(),
                key: None,
                value: serde_json::Value::Null,
            },
        ];

        for event in &events {
            let name = event_type_name(event);
            assert!(!name.is_empty(), "event type name must not be empty");
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "type name must be snake_case: {name}"
            );
        }
    }

    #[test]
    fn event_type_names_are_unique() {
        use std::collections::HashSet;

        let events = vec![
            Event::Connected,
            Event::Disconnected { reason: "".into() },
            Event::PrinterReady,
            Event::StateChanged {
                state: layermind_shared::printer::PrinterState::Idle,
            },
            Event::TemperatureUpdate {
                temperatures: vec![],
            },
            Event::HeaterFault {
                heater: "".into(),
                message: "".into(),
            },
            Event::PositionUpdate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Event::SpeedUpdate { speed: 0.0 },
            Event::FanUpdate {
                name: "".into(),
                speed: 0.0,
                rpm: None,
            },
            Event::PrintStarted {
                filename: "".into(),
                estimated_time: None,
            },
            Event::PrintProgress {
                progress: 0.0,
                elapsed: 0.0,
                remaining: None,
                current_layer: None,
                total_layers: None,
            },
            Event::PrintPaused { reason: None },
            Event::PrintResumed,
            Event::PrintCompleted {
                total_time: 0.0,
                filament_used: None,
            },
            Event::PrintFailed { reason: None },
            Event::PrintCancelled,
            Event::GcodeResponse {
                command: "".into(),
                response: "".into(),
            },
            Event::Error {
                code: None,
                message: "".into(),
            },
            Event::Warning { message: "".into() },
            Event::Raw {
                namespace: "".into(),
                key: None,
                value: serde_json::Value::Null,
            },
        ];

        let names: Vec<String> = events.iter().map(event_type_name).collect();
        let unique: HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len(), "event type names must be unique");
    }
}

#[cfg(test)]
mod integration {
    use chrono::Utc;
    use layermind_database::{Database, Repository};
    use layermind_shared::event::{Envelope, Event};
    use uuid::Uuid;

    /// Return the test database URL from the environment, or skip.
    fn db_url() -> Option<String> {
        std::env::var("LAYERMIND_TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .ok()
    }

    async fn setup_db() -> Option<Database> {
        let url = db_url()?;
        let config = layermind_config::DatabaseConfig {
            url,
            max_connections: 2,
            run_migrations: true,
        };
        Database::connect(&config).await.ok()
    }

    fn make_envelope(printer_id: &str, event: Event) -> Envelope {
        Envelope {
            event_id: Uuid::now_v7(),
            printer_id: printer_id.into(),
            timestamp: Utc::now(),
            payload: event,
        }
    }

    #[tokio::test]
    async fn auto_registers_printers() {
        let db = match setup_db().await {
            Some(db) => db,
            None => {
                eprintln!("skipping test: no DATABASE_URL set");
                return;
            }
        };

        let events = vec![
            make_envelope("550e8400-e29b-41d4-a716-446655440000", Event::Connected),
            make_envelope("550e8400-e29b-41d4-a716-446655440000", Event::PrinterReady),
        ];

        let sink = db.create_sink();
        sink.write_batch(&events).await.unwrap();

        let repo = Repository::new(db.pool().clone());
        let printers = repo.list_printers().await.unwrap();

        assert!(
            printers
                .iter()
                .any(|p| p.id.to_string() == "550e8400-e29b-41d4-a716-446655440000"),
            "printer should be auto-registered"
        );
    }

    #[tokio::test]
    async fn persists_telemetry_events() {
        let db = match setup_db().await {
            Some(db) => db,
            None => {
                eprintln!("skipping test: no DATABASE_URL set");
                return;
            }
        };

        let printer_id = "660e8400-e29b-41d4-a716-446655440001";

        let events = vec![
            make_envelope(
                printer_id,
                Event::TemperatureUpdate {
                    temperatures: vec![layermind_shared::types::Temperature {
                        sensor: "extruder_0".into(),
                        current: 210.0,
                        target: 210.0,
                        power: Some(0.5),
                    }],
                },
            ),
            make_envelope(
                printer_id,
                Event::PrintStarted {
                    filename: "benchy.gcode".into(),
                    estimated_time: Some(3600.0),
                },
            ),
            make_envelope(
                printer_id,
                Event::PrintProgress {
                    progress: 0.5,
                    elapsed: 1800.0,
                    remaining: Some(1800.0),
                    current_layer: Some(50),
                    total_layers: Some(100),
                },
            ),
        ];

        let sink = db.create_sink();
        sink.write_batch(&events).await.unwrap();

        let repo = Repository::new(db.pool().clone());
        let stored = repo
            .recent_events(Uuid::parse_str(printer_id).unwrap(), 100)
            .await
            .unwrap();

        assert_eq!(stored.len(), 3, "all 3 events should be persisted");
        assert_eq!(stored[0].event_type, "print_progress");
        assert_eq!(stored[1].event_type, "print_started");
        assert_eq!(stored[2].event_type, "temperature_update");
    }

    #[tokio::test]
    async fn sink_is_idempotent_for_same_printer() {
        let db = match setup_db().await {
            Some(db) => db,
            None => {
                eprintln!("skipping test: no DATABASE_URL set");
                return;
            }
        };

        let printer_id = "770e8400-e29b-41d4-a716-446655440002";

        // Write the same printer twice — should not error.
        let events = vec![make_envelope(printer_id, Event::Connected)];
        let sink = db.create_sink();
        sink.write_batch(&events).await.unwrap();
        sink.write_batch(&events).await.unwrap();

        let repo = Repository::new(db.pool().clone());
        let printers = repo.list_printers().await.unwrap();
        let count = printers
            .iter()
            .filter(|p| p.id.to_string() == printer_id)
            .count();
        assert_eq!(count, 1, "printer should only be registered once");
    }
}
