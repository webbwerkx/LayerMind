//! CLI command implementations.
//!
//! Each command is a function that takes a [`Runtime`] and optional
//! arguments, performs the operation, and prints results to stdout.

use std::sync::Arc;

use layermind_machine::MachineProfileBuilder;

use crate::runtime::Runtime;

/// Test the connection to a printer via Moonraker.
pub async fn cmd_printer_test(_rt: &Runtime, printer_id: &str) -> anyhow::Result<()> {
    println!("Moonraker Connection\n");
    println!("  Printer: {printer_id}");
    println!();

    let profile = MachineProfileBuilder::unknown_profile(printer_id);

    println!("Machine:");
    println!("  Motion: {:?}", profile.identity.machine_type.value);
    if let Some(ref fw) = profile.identity.firmware {
        if let Some(ref v) = fw.klipper_version {
            println!("  Klipper: {v}");
        }
        if let Some(ref v) = fw.moonraker_version {
            println!("  Moonraker: {v}");
        }
        println!("  MCUs: {}", fw.mcu_count.value);
    }

    if let Some(ref mfr) = profile.identity.manufacturer {
        println!("  Manufacturer: {} [{:?}]", mfr.value, mfr.source);
    }
    if let Some(ref model) = profile.identity.model {
        println!("  Model: {} [{:?}]", model.value, model.source);
    }

    println!();
    println!("Capabilities:");

    let caps = &profile.capabilities;
    let cap = |name: &str, supported: bool| {
        println!(
            "  {name}: {}",
            if supported {
                "supported"
            } else {
                "not detected"
            }
        )
    };
    cap("Input shaping", caps.supports_input_shaping.value);
    cap("Pressure advance", caps.supports_pressure_advance.value);
    cap("Sensorless homing", caps.supports_sensorless_homing.value);
    cap("CAN bus", caps.supports_canbus.value);
    cap("BLTouch/CRTouch", caps.supports_bltouch.value);
    cap("Beacon probe", caps.supports_beacon.value);
    cap("High temperature", caps.supports_high_temperature.value);
    if caps.maximum_temperature.value > 0.0 {
        println!("  Max temperature: {:.0}°C", caps.maximum_temperature.value);
    }

    Ok(())
}

/// Display a live status monitor for the printer.
pub async fn cmd_monitor(rt: &Runtime, printer_id: &str) -> anyhow::Result<()> {
    println!("LayerMind Runtime Status\n");
    println!("  Runtime started: {}", rt.started_at);
    println!(
        "  AI provider: {} ({})",
        rt.provider.name(),
        rt.provider.model()
    );
    println!();

    if let Some(ctx) = rt.context_store.context(printer_id) {
        println!("Printer: {printer_id}");
        println!();
        println!("  Name: {}", ctx.summary.name);
        if let Some(ref model) = ctx.summary.model {
            println!("  Model: {model}");
        }
        if let Some(ref fw) = ctx.summary.firmware {
            println!("  Firmware: {fw}");
        }
        println!();
        println!(
            "  Status: {}",
            if ctx.current_state.is_printing {
                "PRINTING"
            } else {
                "IDLE"
            }
        );

        if let Some(ref filename) = ctx.current_state.active_print_filename {
            println!("  Active print: {filename}");
        }

        println!();
        println!("  Health:");
        if let Some(stability) = ctx.health.temperature_stability {
            println!("    Temperature stability: {stability:.2}");
        }
        if let Some(rate) = ctx.health.success_rate {
            println!("    Success rate: {rate:.2}");
        }
        println!("    Total prints: {}", ctx.print_history.total_prints);
        println!("    Failed prints: {}", ctx.print_history.failed_prints);

        if let Some(ref machine) = ctx.machine {
            println!();
            println!("  Machine Intelligence:");
            println!("    Motion: {:?}", machine.identity.machine_type.value);
            println!("    Extruders: {}", machine.hardware.extruders.len());
            println!("    Hotends: {}", machine.hardware.hotends.len());
            if !machine.hardware.probes.is_empty() {
                let probes: Vec<String> = machine
                    .hardware
                    .probes
                    .iter()
                    .map(|p| format!("{:?}", p.details.probe_type.value))
                    .collect();
                println!("    Probes: {}", probes.join(", "));
            }
        }

        println!();
        println!("  Recent History:");
        if let Some(last_hw) = ctx.history.last_hardware_change {
            println!("    Last hardware change: {last_hw}");
        }
        if let Some(last_fw) = ctx.history.last_firmware_update {
            println!("    Last firmware update: {last_fw}");
        }
        if let Some(last_cfg) = ctx.history.last_config_change {
            println!("    Last config change: {last_cfg}");
        }
        if let Some(last_cal) = ctx.history.last_calibration {
            println!("    Last calibration: {last_cal}");
        }
        for change in &ctx.history.recent_changes {
            println!("    - {} [{:?}]", change.summary, change.category);
        }
    } else {
        println!("Printer: {printer_id}");
        println!();
        println!("  No context data available yet.");
        println!("  Start telemetry collection to build context.");
    }

    Ok(())
}

/// Run an AI diagnostic on the printer.
pub async fn cmd_diagnose(rt: &Runtime, printer_id: &str) -> anyhow::Result<()> {
    use layermind_reasoning::DiagnosticOrchestrator;

    println!("AI Diagnostic\n");

    let ctx = match rt.context_store.context(printer_id) {
        Some(c) => c,
        None => {
            println!("  No context available for printer '{printer_id}'.");
            println!(
                "  The printer must be connected and sending telemetry before diagnostics can run."
            );
            return Ok(());
        }
    };

    println!("  Printer: {}", ctx.summary.name);
    println!(
        "  Status: {}",
        if ctx.current_state.is_printing {
            "PRINTING"
        } else {
            "IDLE"
        }
    );
    println!("  Using: {} ({})", rt.provider.name(), rt.provider.model());
    println!();

    let result = DiagnosticOrchestrator::diagnose(&ctx, Arc::clone(&rt.provider)).await;

    match result {
        Ok(validated) => {
            println!("  Summary: {}", validated.recommendation.summary);
            println!("  Confidence: {:.2}", validated.recommendation.confidence);
            println!();
            println!("  Actions:");
            for (i, action) in validated.recommendation.actions.iter().enumerate() {
                println!("    {}. {}", i + 1, action.description);
                println!("       Priority: {}", action.priority);
                if let Some(ref cmd) = action.suggested_command {
                    println!("       Command: {cmd}");
                }
                println!("       Expected: {}", action.expected_outcome);
            }

            if !validated.disclaimers.is_empty() {
                println!();
                println!("  Disclaimers:");
                for d in &validated.disclaimers {
                    println!("    - {d}");
                }
            }

            let usage = &validated.recommendation.usage;
            println!();
            println!(
                "  Tokens: {} prompt + {} completion | Cost: ${:.6}",
                usage.prompt_tokens, usage.completion_tokens, usage.estimated_cost_usd,
            );
            println!("  Provider: {} / {}", usage.provider, usage.model,);
        }
        Err(e) => {
            println!("  Diagnostic failed: {e}");
        }
    }

    Ok(())
}
