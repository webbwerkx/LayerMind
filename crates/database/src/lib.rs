//! LayerMind database layer.
//!
//! Manages PostgreSQL connections, schema migrations, and provides typed
//! query interfaces for all domain entities. Will use sqlx when wired up.
//!
//! Key entities:
//! - Printer
//! - PrintJob
//! - TelemetryEvent
//! - Filament
//! - Profile
//! - Calibration
//! - Failure
//! - Recommendation
//! - MaintenanceEvent
//! - AiObservation

use layermind_config::DatabaseConfig;
use layermind_shared::error::Result;

mod migrations;
mod models;

/// Database connection pool and operations.
pub struct Database {
    config: DatabaseConfig,
}

impl Database {
    pub async fn connect(config: DatabaseConfig) -> Result<Self> {
        tracing::info!("connecting to database");
        // TODO: sqlx::PgPool::connect(&config.url)
        Ok(Self { config })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        tracing::info!("running database migrations");
        // TODO: sqlx::migrate!().run(&pool)
        Ok(())
    }
}
