-- LayerMind initial schema.
-- Designed for future TimescaleDB migration:
--   SELECT create_hypertable('telemetry_events', 'recorded_at');
--   SELECT add_retention_policy('telemetry_events', INTERVAL '90 days');

CREATE TABLE IF NOT EXISTS printers (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    model       TEXT,
    firmware    TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS print_jobs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id  UUID NOT NULL REFERENCES printers(id),
    filename    TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'printing',
    start_time  TIMESTAMPTZ NOT NULL DEFAULT now(),
    end_time    TIMESTAMPTZ,
    duration    DOUBLE PRECISION,
    CONSTRAINT valid_status CHECK (status IN (
        'printing', 'paused', 'completed', 'failed', 'cancelled'
    ))
);

CREATE TABLE IF NOT EXISTS telemetry_events (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id   UUID NOT NULL REFERENCES printers(id),
    print_job_id UUID REFERENCES print_jobs(id),
    event_type   TEXT NOT NULL,
    payload      JSONB NOT NULL DEFAULT '{}',
    recorded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Primary query path: get events for a printer, ordered by time.
CREATE INDEX IF NOT EXISTS idx_telemetry_printer_time
    ON telemetry_events (printer_id, recorded_at DESC);

-- Lookup by event type (e.g. all TemperatureUpdate events).
CREATE INDEX IF NOT EXISTS idx_telemetry_type
    ON telemetry_events (event_type);

-- Link events to their print job.
CREATE INDEX IF NOT EXISTS idx_telemetry_job
    ON telemetry_events (print_job_id) WHERE print_job_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS calibration_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id  UUID NOT NULL REFERENCES printers(id),
    cal_type    TEXT NOT NULL,
    values      JSONB NOT NULL DEFAULT '{}',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_calibration_printer
    ON calibration_events (printer_id, recorded_at DESC);

CREATE TABLE IF NOT EXISTS ai_observations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id  UUID NOT NULL REFERENCES printers(id),
    category    TEXT NOT NULL,
    observation TEXT NOT NULL,
    confidence  DOUBLE PRECISION,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ai_observations_printer
    ON ai_observations (printer_id, created_at DESC);
