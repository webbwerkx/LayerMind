//! Runtime state — holds all wired application components.
//!
//! The runtime is the central state object passed to every command
//! handler. It owns references to all configured services.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use layermind_config::Config;
use layermind_context::ContextStore;
use layermind_machine::MachineProfileBuilder;
use layermind_reasoning::diagnostic::PrintDoctor;
use layermind_reasoning::AiProvider;

/// The fully bootstrapped LayerMind runtime.
///
/// All fields are available to command handlers. Commands consume the
/// runtime to perform operations against live or configured services.
pub struct Runtime {
    /// Loaded configuration (environment + config file).
    pub config: Config,
    /// Configured AI provider.
    pub provider: Arc<dyn AiProvider>,
    /// Machine profile builder for hardware discovery.
    pub machine_builder: Arc<MachineProfileBuilder>,
    /// Shared printer context store.
    pub context_store: Arc<ContextStore>,
    /// AI print doctor for diagnostic commands.
    pub print_doctor: Arc<PrintDoctor>,
    /// When the runtime was started.
    pub started_at: DateTime<Utc>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("provider", &self.provider.name())
            .field("model", &self.provider.model())
            .field("started_at", &self.started_at)
            .finish()
    }
}
