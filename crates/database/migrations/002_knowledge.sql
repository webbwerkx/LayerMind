-- Knowledge Engine persistence layer.
--
-- Three tables track the output of the Knowledge Engine:
-- 1. knowledge_observations — observation lifecycle with scoring
-- 2. printer_profiles — evolving, aggregated printer knowledge
-- 3. printer_timeline — chronological important events

-- ── Knowledge Observations ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS knowledge_observations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id      TEXT NOT NULL,
    observation_id  UUID NOT NULL,
    category        TEXT NOT NULL,
    severity        TEXT NOT NULL DEFAULT 'info',
    importance      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    confidence      DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    status          TEXT NOT NULL DEFAULT 'active',
    resolution      TEXT,
    resolved_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT valid_knowledge_status CHECK (status IN (
        'active', 'acknowledged', 'resolved'
    ))
);

-- Lookup observations by printer and status (dashboard).
CREATE INDEX IF NOT EXISTS idx_knowledge_printer_status
    ON knowledge_observations (printer_id, status);

-- Lookup specific observation by source observation_id.
CREATE INDEX IF NOT EXISTS idx_knowledge_observation
    ON knowledge_observations (observation_id);

-- Most recent active issues.
CREATE INDEX IF NOT EXISTS idx_knowledge_active
    ON knowledge_observations (created_at DESC)
    WHERE status = 'active';

-- ── Printer Profiles ─────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS printer_profiles (
    printer_id              TEXT PRIMARY KEY,
    hardware                JSONB NOT NULL DEFAULT '{}',
    behavior                JSONB NOT NULL DEFAULT '{}',
    reliability_score       DOUBLE PRECISION,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ── Printer Timeline ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS printer_timeline (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    printer_id      TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    description     TEXT NOT NULL,
    severity        TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}',
    occurred_at     TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Chronological timeline query: most recent events first.
CREATE INDEX IF NOT EXISTS idx_timeline_printer_time
    ON printer_timeline (printer_id, occurred_at DESC);

-- Filter timeline by event type (e.g. all failures).
CREATE INDEX IF NOT EXISTS idx_timeline_type
    ON printer_timeline (event_type);