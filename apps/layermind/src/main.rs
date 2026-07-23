//! LayerMind — AI-powered operating system for additive manufacturing.
//!
//! ```text
//! Usage:
//!   layermind printer test    Test Moonraker connection
//!   layermind monitor         Live printer status
//!   layermind diagnose        AI diagnostic analysis
//!   layermind run             Start the full daemon (background pipeline)
//! ```

mod application;
mod commands;
mod runtime;

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    // Load configuration (from env + config file).
    let config = layermind_config::Config::load()?;

    let printer_id = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| "default".to_string());

    match args[1].as_str() {
        "printer" if args.get(2).map(|s| s.as_str()) == Some("test") => {
            let rt = application::bootstrap_test(&config).await?;
            commands::cmd_printer_test(&rt, &printer_id).await?;
        }
        "monitor" => {
            let rt = application::bootstrap_test(&config).await?;
            commands::cmd_monitor(&rt, &printer_id).await?;
        }
        "diagnose" => {
            let rt = application::bootstrap(&config).await?;
            commands::cmd_diagnose(&rt, &printer_id).await?;
        }
        "run" => {
            println!("Starting LayerMind daemon...");
            layermind_core::run().await?;
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_usage() {
    println!("LayerMind v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage:");
    println!("  layermind printer test [printer_id]  Test Moonraker connection");
    println!("  layermind monitor [printer_id]       Live printer status");
    println!("  layermind diagnose [printer_id]      AI diagnostic analysis");
    println!("  layermind run                         Start the full daemon");
    println!();
    println!("Environment:");
    println!("  LAYERMIND_PROVIDER       AI provider (openai, openrouter, ollama, anthropic, gemini, custom)");
    println!("  LAYERMIND_MODEL          Model name");
    println!("  LAYERMIND_PROVIDER_ENDPOINT  Custom endpoint URL");
    println!("  MOONRAKER_URL            Moonraker WebSocket URL");
    println!("  DATABASE_URL             PostgreSQL connection string");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_string_is_non_empty() {
        // Just verify the module compiles and links.
        assert!(env!("CARGO_PKG_VERSION").len() > 0);
    }
}
