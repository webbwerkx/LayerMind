//! Application bootstrap — loads configuration, wires all crates,
//! and manages the runtime lifecycle.
//!
//! The application module is the glue between the CLI entry point and
//! the LayerMind backend crates. It owns no business logic — everything
//! is delegated to the appropriate crate.

use std::sync::Arc;

use chrono::Utc;
use layermind_config::Config;
use layermind_context::ContextStore;
use layermind_machine::MachineProfileBuilder;
use layermind_reasoning::diagnostic::{DiagnoseError, PrintDoctor};
use layermind_reasoning::AiProvider;
use layermind_shared::context::PrinterContext;
use layermind_shared::recommendation::ValidatedRecommendation;

use crate::runtime::Runtime;

/// Bootstrap the full LayerMind application from configuration.
///
/// Returns a fully wired [`Runtime`] ready for command execution.
pub async fn bootstrap(config: &Config) -> anyhow::Result<Runtime> {
    layermind_logging::init(&config.logging);
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "LayerMind bootstrapping"
    );

    // ── AI provider ──────────────────────────────────────────
    let provider: Arc<dyn AiProvider> = layermind_ai::create_provider(&config.provider)
        .map_err(|e| anyhow::anyhow!("failed to create AI provider: {}", e))?;

    tracing::info!(
        provider = %provider.name(),
        model = %provider.model(),
        "AI provider ready"
    );

    // ── Context store (shared across all consumers) ──────────
    let context_store = Arc::new(ContextStore::new());

    // ── Print doctor ─────────────────────────────────────────
    let print_doctor = Arc::new(PrintDoctor::new(Arc::clone(&provider)));

    Ok(Runtime {
        config: config.clone(),
        provider,
        machine_builder: Arc::new(MachineProfileBuilder::new()),
        context_store,
        print_doctor,
        started_at: Utc::now(),
    })
}

/// Bootstrap in "offline" mode with a mock AI provider (for testing).
/// Does NOT initialize logging (callers should do that once).
pub async fn bootstrap_test(config: &Config) -> anyhow::Result<Runtime> {
    let provider: Arc<dyn AiProvider> = Arc::new(layermind_reasoning::provider::MockProvider::new(
        "mock",
        "mock-model",
        r#"{"actions":[],"summary":"mock"}"#,
    ));

    Ok(Runtime {
        config: config.clone(),
        provider,
        machine_builder: Arc::new(MachineProfileBuilder::new()),
        context_store: Arc::new(ContextStore::new()),
        print_doctor: Arc::new(PrintDoctor::new(Arc::new(
            layermind_reasoning::provider::MockProvider::new(
                "mock-doc",
                "mock-model",
                r#"{"actions":[],"summary":"mock"}"#,
            ),
        ) as Arc<dyn AiProvider>)),
        started_at: Utc::now(),
    })
}

/// Run an AI diagnostic against a printer using the existing
/// PrintDoctor pipeline.
pub async fn diagnose_printer(
    store: &ContextStore,
    doctor: &PrintDoctor,
    printer_id: &str,
) -> Result<ValidatedRecommendation, DiagnoseError> {
    layermind_core::diagnose_printer(store, doctor, printer_id).await
}

/// Produce a printer context snapshot for display.
pub fn printer_context(store: &ContextStore, printer_id: &str) -> Option<PrinterContext> {
    store.context(printer_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use layermind_config::Config;

    #[tokio::test]
    async fn bootstrap_offline_succeeds() {
        let config = Config::default();
        let rt = bootstrap_test(&config).await;
        assert!(rt.is_ok());
    }

    #[tokio::test]
    async fn offline_runtime_has_provider() {
        let config = Config::default();
        let rt = bootstrap_test(&config).await.unwrap();
        assert_eq!(rt.provider.name(), "mock");
    }

    #[tokio::test]
    async fn offline_runtime_has_context_store() {
        let config = Config::default();
        let rt = bootstrap_test(&config).await.unwrap();
        assert!(rt.context_store.context("nonexistent").is_none());
    }

    #[tokio::test]
    async fn diagnose_on_empty_store_returns_error() {
        let config = Config::default();
        let rt = bootstrap_test(&config).await.unwrap();
        let result = diagnose_printer(&rt.context_store, &rt.print_doctor, "ghost").await;
        assert!(result.is_err());
    }
}
