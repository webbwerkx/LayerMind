//! Service wiring and dependency injection.

use layermind_config::Config;
use layermind_shared::error::Result;

/// Builds and wires together all LayerMind services.
pub struct ServiceGraph;

impl ServiceGraph {
    pub async fn build(config: &Config) -> Result<()> {
        tracing::info!("building service graph");

        // Database connection (optional — system runs without it)
        let db = layermind_database::Database::connect(config.database.clone()).await;
        match &db {
            Ok(_) => tracing::info!("database connected"),
            Err(e) => {
                tracing::warn!(error = %e, "database unavailable, running without persistence")
            }
        }

        Ok(())
    }
}
