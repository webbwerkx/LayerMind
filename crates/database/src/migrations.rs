//! Database migrations.
//!
//! Versioned SQL migrations for the LayerMind schema.
//! Each migration is idempotent where possible.

/// Initial schema — creates all core tables.
pub const V1_INIT: &str = r#"
CREATE TABLE IF NOT EXISTS printers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    model TEXT,
    firmware TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS print_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id UUID NOT NULL REFERENCES printers(id),
    filename TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'started',
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    success BOOLEAN,
    filament_used_mm DOUBLE PRECISION,
    total_layers INTEGER,
    failure_reason TEXT,
    metadata JSONB,
    CONSTRAINT valid_status CHECK (status IN ('started', 'printing', 'paused', 'completed', 'failed', 'cancelled'))
);

CREATE TABLE IF NOT EXISTS telemetry_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id UUID NOT NULL REFERENCES printers(id),
    print_job_id UUID REFERENCES print_jobs(id),
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_telemetry_printer_time
    ON telemetry_events (printer_id, recorded_at DESC);

CREATE INDEX IF NOT EXISTS idx_telemetry_type
    ON telemetry_events (event_type);

CREATE TABLE IF NOT EXISTS filaments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    material TEXT NOT NULL,
    brand TEXT,
    color TEXT,
    diameter DOUBLE PRECISION NOT NULL DEFAULT 1.75,
    spool_weight_g DOUBLE PRECISION,
    cost_per_kg DOUBLE PRECISION
);

CREATE TABLE IF NOT EXISTS failures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    print_job_id UUID NOT NULL REFERENCES print_jobs(id),
    category TEXT NOT NULL,
    description TEXT NOT NULL,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE IF NOT EXISTS recommendations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id UUID REFERENCES printers(id),
    print_job_id UUID REFERENCES print_jobs(id),
    category TEXT NOT NULL,
    message TEXT NOT NULL,
    confidence DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    acknowledged BOOLEAN NOT NULL DEFAULT false,
    applied BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE IF NOT EXISTS calibrations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id UUID NOT NULL REFERENCES printers(id),
    calibration_type TEXT NOT NULL,
    result JSONB NOT NULL DEFAULT '{}',
    performed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS ai_observations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id UUID NOT NULL REFERENCES printers(id),
    print_job_id UUID REFERENCES print_jobs(id),
    observation_type TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail JSONB,
    confidence DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#;
