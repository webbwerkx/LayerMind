//! LayerMind database layer.
//!
//! Manages PostgreSQL connections, schema migrations, and provides typed
//! query interfaces for all domain entities.
//!
//! ## Storage Backend
//!
//! Currently PostgreSQL via `sqlx`. The schema is designed for future
//! migration to TimescaleDB for high-volume telemetry (hypertables,
//! compression, retention policies).
//!
//! ## Architecture
//!
//! Database-specific code is confined to this crate. The rest of LayerMind
//! interacts through the `Sink` trait (defined in `layermind_shared`).
//! This means the telemetry pipeline has zero knowledge of PostgreSQL.
//!
//! ## Key Entities
//!
//! - Printer
//! - PrintJob
//! - TelemetryEvent (primary time-series table)
//! - CalibrationEvent
//! - AiObservation

use std::sync::Arc;

use async_trait::async_trait;
use layermind_config::DatabaseConfig;
use layermind_shared::error::{Error, Result};
use layermind_shared::event::Envelope;
use layermind_shared::sink::Sink;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing;
use uuid::Uuid;

pub mod models;

// ── Database ────────────────────────────────────────────────────────

/// Connection pool and migration manager.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Connect to PostgreSQL and run pending migrations.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        tracing::info!(url = %mask_url(&config.url), "connecting to database");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&config.url)
            .await
            .map_err(|e| Error::Database(format!("connection failed: {e}")))?;

        let db = Self { pool };

        if config.run_migrations {
            db.run_migrations().await?;
        }

        tracing::info!("database ready");
        Ok(db)
    }

    /// Run pending sqlx migrations.
    pub async fn run_migrations(&self) -> Result<()> {
        tracing::info!("running database migrations");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| Error::Database(format!("migration failed: {e}")))?;
        Ok(())
    }

    /// Return the pool for direct queries.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Create a sink that writes telemetry events to this database.
    pub fn create_sink(&self) -> Arc<dyn Sink> {
        Arc::new(DatabaseSink::new(self.pool.clone()))
    }
}

// ── DatabaseSink ────────────────────────────────────────────────────

/// Implements the `Sink` trait for PostgreSQL.
///
/// Each `write_batch` is a single atomic INSERT using UNNEST for
/// maximum throughput. Printer auto-registration happens inline —
/// unknown printer IDs are inserted before event rows reference them.
struct DatabaseSink {
    pool: PgPool,
}

impl DatabaseSink {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Sink for DatabaseSink {
    async fn write_batch(&self, events: &[Envelope]) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        // Auto-register any unknown printers.
        auto_register_printers(&self.pool, events).await?;

        // Batch insert telemetry events.
        insert_telemetry_batch(&self.pool, events).await?;

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // PostgreSQL writes are synchronous per-batch — no buffering to flush.
        Ok(())
    }
}

// ── Printer Auto-Registration ───────────────────────────────────────

async fn auto_register_printers(pool: &PgPool, events: &[Envelope]) -> Result<()> {
    // Collect unique printer IDs from this batch.
    let mut ids: Vec<&str> = events.iter().map(|e| e.printer_id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();

    // INSERT ... ON CONFLICT DO NOTHING for idempotent registration.
    for id in &ids {
        let name = format!("Printer {:.8}", id);
        sqlx::query(
            r#"INSERT INTO printers (id, name) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET last_seen = now()"#,
        )
        .bind(Uuid::parse_str(id).unwrap_or_else(|_| Uuid::nil()))
        .bind(&name)
        .execute(pool)
        .await
        .map_err(|e| Error::Database(format!("printer registration failed: {e}")))?;
    }

    Ok(())
}

// ── Telemetry Batch Insert ──────────────────────────────────────────

async fn insert_telemetry_batch(pool: &PgPool, events: &[Envelope]) -> Result<()> {
    let count = events.len();

    let ids: Vec<Uuid> = events.iter().map(|e| e.event_id).collect();
    let printer_ids: Vec<Uuid> = events
        .iter()
        .map(|e| Uuid::parse_str(&e.printer_id).unwrap_or_else(|_| Uuid::nil()))
        .collect();
    let event_types: Vec<String> = events.iter().map(|e| event_type_name(&e.payload)).collect();
    let payloads: Vec<serde_json::Value> = events
        .iter()
        .map(|e| serde_json::to_value(&e.payload).unwrap_or_default())
        .collect();
    let timestamps: Vec<chrono::DateTime<chrono::Utc>> =
        events.iter().map(|e| e.timestamp).collect();

    sqlx::query(
        r#"INSERT INTO telemetry_events (id, printer_id, event_type, payload, recorded_at)
           SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::text[], $4::jsonb[], $5::timestamptz[])"#,
    )
    .bind(&ids)
    .bind(&printer_ids)
    .bind(&event_types)
    .bind(&payloads)
    .bind(&timestamps)
    .execute(pool)
    .await
    .map_err(|e| Error::Database(format!("telemetry insert failed: {e}")))?;

    tracing::debug!(count, "telemetry events persisted");
    Ok(())
}

/// Map a canonical Event variant to a short event type string for indexing.
pub fn event_type_name(event: &layermind_shared::event::Event) -> String {
    match event {
        layermind_shared::event::Event::Connected => "connected",
        layermind_shared::event::Event::Disconnected { .. } => "disconnected",
        layermind_shared::event::Event::PrinterReady => "printer_ready",
        layermind_shared::event::Event::StateChanged { .. } => "state_changed",
        layermind_shared::event::Event::TemperatureUpdate { .. } => "temperature_update",
        layermind_shared::event::Event::HeaterFault { .. } => "heater_fault",
        layermind_shared::event::Event::PositionUpdate { .. } => "position_update",
        layermind_shared::event::Event::SpeedUpdate { .. } => "speed_update",
        layermind_shared::event::Event::FanUpdate { .. } => "fan_update",
        layermind_shared::event::Event::PrintStarted { .. } => "print_started",
        layermind_shared::event::Event::PrintProgress { .. } => "print_progress",
        layermind_shared::event::Event::PrintPaused { .. } => "print_paused",
        layermind_shared::event::Event::PrintResumed => "print_resumed",
        layermind_shared::event::Event::PrintCompleted { .. } => "print_completed",
        layermind_shared::event::Event::PrintFailed { .. } => "print_failed",
        layermind_shared::event::Event::PrintCancelled => "print_cancelled",
        layermind_shared::event::Event::GcodeResponse { .. } => "gcode_response",
        layermind_shared::event::Event::Error { .. } => "error",
        layermind_shared::event::Event::Warning { .. } => "warning",
        layermind_shared::event::Event::Raw { .. } => "raw",
    }
    .into()
}

// ── Query Repository ────────────────────────────────────────────────

/// High-level query interface for common data access patterns.
pub struct Repository {
    pool: PgPool,
}

impl Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Return the most recent telemetry events for a printer.
    pub async fn recent_events(
        &self,
        printer_id: Uuid,
        limit: i64,
    ) -> Result<Vec<models::TelemetryEvent>> {
        sqlx::query_as::<_, models::TelemetryEvent>(
            r#"SELECT id, printer_id, print_job_id, event_type, payload, recorded_at
               FROM telemetry_events
               WHERE printer_id = $1
               ORDER BY recorded_at DESC
               LIMIT $2"#,
        )
        .bind(printer_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("query failed: {e}")))
    }

    /// Return all print jobs for a printer, newest first.
    pub async fn print_history(
        &self,
        printer_id: Uuid,
        limit: i64,
    ) -> Result<Vec<models::PrintJob>> {
        sqlx::query_as::<_, models::PrintJob>(
            r#"SELECT id, printer_id, filename, status, start_time, end_time, duration
               FROM print_jobs
               WHERE printer_id = $1
               ORDER BY start_time DESC
               LIMIT $2"#,
        )
        .bind(printer_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("query failed: {e}")))
    }

    /// Return all telemetry events for a specific print job.
    pub async fn telemetry_for_print(
        &self,
        print_job_id: Uuid,
    ) -> Result<Vec<models::TelemetryEvent>> {
        sqlx::query_as::<_, models::TelemetryEvent>(
            r#"SELECT id, printer_id, print_job_id, event_type, payload, recorded_at
               FROM telemetry_events
               WHERE print_job_id = $1
               ORDER BY recorded_at ASC"#,
        )
        .bind(print_job_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("query failed: {e}")))
    }

    /// Return all registered printers.
    pub async fn list_printers(&self) -> Result<Vec<models::Printer>> {
        sqlx::query_as::<_, models::Printer>(
            r#"SELECT id, name, model, firmware, created_at, last_seen
               FROM printers
               ORDER BY last_seen DESC"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(format!("query failed: {e}")))
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn mask_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        format!(
            "{}://{}:***@{}{}",
            parsed.scheme(),
            parsed.username(),
            parsed.host_str().unwrap_or(""),
            parsed.path()
        )
    } else {
        url.to_string()
    }
}
