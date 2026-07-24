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

    if args.len() == 1 || args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return Ok(());
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("LayerMind v{}", env!("CARGO_PKG_VERSION"));
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
            let rt = application::bootstrap(&config).await?;
            commands::cmd_printer_test(&rt, &printer_id).await?;
        }
        "monitor" => {
            let rt = application::bootstrap(&config).await?;
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
    println!("LayerMind v{} — AI-powered OS for additive manufacturing", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    layermind <command> [args]");
    println!();
    println!("COMMANDS:");
    println!("    printer test [id]    Connect to printer and show hardware/capabilities");
    println!("    monitor [id]         Show live printer context (requires layermind run)");
    println!("    diagnose [id]        Run AI diagnostic (requires layermind run)");
    println!("    run                  Start the full daemon pipeline");
    println!();
    println!("FLAGS:");
    println!("    -h, --help           Show this help");
    println!("    -V, --version        Show version");
    println!();
    println!("EXAMPLES:");
    println!("    MOONRAKER_URL=ws://voron.local:7125/websocket layermind printer test");
    println!("    layermind run");
    println!("    layermind monitor");
    println!("    layermind diagnose");
    println!();
    println!("ENVIRONMENT:");
    println!("    MOONRAKER_URL                  Moonraker WebSocket URL (default: ws://localhost:7125/websocket)");
    println!("    LAYERMIND_MOONRAKER_API_KEY    Moonraker API key (optional)");
    println!("    LAYERMIND_PROVIDER             AI provider: openai, openrouter, ollama, anthropic, gemini, custom");
    println!("    LAYERMIND_MODEL                Model name (e.g., deepseek/deepseek-chat, gpt-4o, llama3.3)");
    println!("    LAYERMIND_PROVIDER_ENDPOINT    Custom provider endpoint URL");
    println!("    DATABASE_URL                   PostgreSQL connection string (optional, in-memory fallback)");
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
