use std::sync::Arc;

use layermind_context::ContextStore;
use layermind_machine::MachineProfileBuilder;
use layermind_shared::context::PrinterContext;
use layermind_shared::history::HistorySummary;
use tokio::sync::Mutex;

use crate::app::{AppState, EventLevel, PrinterSnapshot};

/// Run an AI diagnostic and store the result in the app state.
pub async fn run_diagnose(state: &Arc<Mutex<AppState>>) {
    let config = {
        let app = state.lock().await;
        app.config.clone()
    };

    // Build a temporary context store with minimal data.
    let _store = ContextStore::new();
    let snapshot: PrinterSnapshot;

    // Query Moonraker for current state.
    match layermind_moonraker::client::query_hardware_info(&config.moonraker).await {
        Ok((server_info, printer_info, printer_objects)) => {
            let builder = MachineProfileBuilder::new();
            let _profile = builder.build("printer", Some(&printer_info), Some(&server_info), Some(&printer_objects));

            // Parse current state from objects.
            snapshot = crate::client::parse_printer_objects(&printer_objects);
            let state_str = snapshot.state.to_uppercase();
            let hostname = snapshot.hostname.clone().unwrap_or_else(|| "printer".into());

            // Build a minimal PrinterContext.
            let ctx = PrinterContext {
                printer_id: hostname.clone(),
                generated_at: chrono::Utc::now(),
                summary: layermind_shared::context::PrinterSummary {
                    name: hostname.clone(),
                    model: None,
                    firmware: None,
                    first_seen: None,
                    last_seen: None,
                    reliability_score: None,
                    total_observations: 0,
                    total_prints: 0,
                },
                print_history: layermind_shared::context::PrintHistorySummary {
                    total_prints: 0,
                    successful_prints: 0,
                    failed_prints: 0,
                    success_rate: None,
                    avg_duration_secs: None,
                    recent_failures: vec![],
                    common_failure_pattern: None,
                },
                health: layermind_shared::context::HealthSummary {
                    temperature_stability: None,
                    success_rate: None,
                    uptime_secs: 0.0,
                    recent_error_count: 0,
                    recent_warning_count: 0,
                    reliability_score: None,
                },
                current_state: layermind_shared::context::CurrentState {
                    is_printing: state_str == "PRINTING",
                    active_print_filename: snapshot.print_filename.clone(),
                    active_observations: vec![],
                    pending_warnings: vec![],
                    recent_events: vec![],
                },
                known_issues: vec![],
                historical_patterns: vec![],
                recent_evidence: vec![],
                machine: None,
                history: HistorySummary {
                    last_hardware_change: None,
                    last_firmware_update: None,
                    last_config_change: None,
                    last_calibration: None,
                    last_maintenance: None,
                    recent_changes: vec![],
                    total_events: 0,
                    config_age_days: None,
                    hardware_age_days: None,
                },
                learning: None,
            };

            // Create AI provider.
            match layermind_ai::create_provider(&config.provider) {
                Ok(provider) => {
                    let provider = Arc::new(provider);
                    let doctor = layermind_reasoning::diagnostic::PrintDoctor::new(Arc::clone(&provider));

                    let mut app = state.lock().await;
                    app.running_diagnostic = true;
                    app.diagnostic_error = None;
                    app.add_event("Running AI diagnostic...", EventLevel::Info);
                    drop(app);

                    match doctor.diagnose(&ctx).await {
                        Ok(result) => {
                            let mut app = state.lock().await;
                            app.diagnostic_result = Some(result);
                            app.running_diagnostic = false;
                            app.add_event("Diagnostic complete", EventLevel::Info);
                        }
                        Err(e) => {
                            let mut app = state.lock().await;
                            app.diagnostic_error = Some(e.to_string());
                            app.running_diagnostic = false;
                            app.add_event(format!("Diagnostic failed: {e}"), EventLevel::Error);
                        }
                    }
                }
                Err(e) => {
                    let mut app = state.lock().await;
                    app.diagnostic_error = Some(format!("Failed to create AI provider: {e}"));
                    app.add_event(format!("AI provider error: {e}"), EventLevel::Error);
                }
            }
        }
        Err(e) => {
            let mut app = state.lock().await;
            app.diagnostic_error = Some(format!("Moonraker query failed: {e}"));
            app.add_event(format!("Moonraker error: {e}"), EventLevel::Error);
        }
    }
}

/// Query Moonraker and build a machine profile.
pub async fn show_machine(state: &Arc<Mutex<AppState>>) {
    let config = {
        let app = state.lock().await;
        app.config.clone()
    };

    match layermind_moonraker::client::query_hardware_info(&config.moonraker).await {
        Ok((server_info, printer_info, printer_objects)) => {
            let builder = MachineProfileBuilder::new();
            let profile = builder.build("printer", Some(&printer_info), Some(&server_info), Some(&printer_objects));

            let mut app = state.lock().await;
            app.machine_profile = Some(profile);
            app.show_machine = true;
            app.add_event("Machine info loaded", EventLevel::Info);
        }
        Err(e) => {
            let mut app = state.lock().await;
            app.add_event(format!("Machine info failed: {e}"), EventLevel::Error);
        }
    }
}
