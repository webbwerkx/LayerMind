# LayerMind Architecture

This document describes the technical architecture of LayerMind, the design decisions, and the rationale behind them.

## Overview

LayerMind follows a layered, event-driven architecture. Each layer has a clear responsibility and communicates through typed, versioned contracts.

## Layer Diagram

```
┌─────────────────────────────────────────────────┐
│                  Application                     │
│              (apps/desktop — Tauri)              │
├─────────────────────────────────────────────────┤
│                  Orchestration                   │
│               (crates/core)                      │
├──────────────┬────────────────┬─────────────────┤
│ Printer      │ AI Engine      │ Knowledge       │
│ Layer        │ (crates/ai)    │ Engine          │
│ (printer,    │                │ (future)        │
│  moonraker)  │                │                 │
├──────────────┴────────────────┴─────────────────┤
│             Telemetry Pipeline                   │
│            (crates/telemetry)                    │
├─────────────────────────────────────────────────┤
│              Data Storage                        │
│           (crates/database)                      │
├─────────────────────────────────────────────────┤
│           Shared Primitives                      │
│    (crates/shared, config, logging)              │
└─────────────────────────────────────────────────┘
```

## Crate Dependency Graph

```
core
 ├── ai ──────────┐
 ├── telemetry ───┤
 │    ├── printer ─┤
 │    │    └── moonraker
 │    └── database
 ├── database ────┤
 ├── logging ─────┤
 ├── config ──────┤
 └── shared ◄─────┘  (everything depends on shared)
```

## Crate Responsibilities

### `shared`
Canonical types, error enums, and trait interfaces. Zero business logic. Every crate depends on this; this crate depends on nothing internal.

Key types:
- `Event` — all observable printer events (temperature, progress, errors, etc.)
- `Envelope` — timestamped, identified event container
- `PrinterState` — high-level printer state enum
- `Error` — unified error type

### `config`
Configuration loading and validation. Reads from XDG config paths, environment variables, and config files. Provides typed, validated access to all settings.

### `logging`
Structured logging via `tracing`. Supports human-readable and JSON output modes.

### `moonraker`
Pure protocol adapter for Moonraker's WebSocket API. Connects, authenticates, subscribes to printer objects, and publishes raw JSON-RPC messages. No business logic.

Implementation:
- `tokio-tungstenite` for async WebSocket
- JSON-RPC 2.0 request/response with typed status update model
- Exponential backoff reconnect (1s → 2s → 4s ... → 60s cap)
- Heartbeat monitoring (configurable interval)
- Graceful shutdown via `tokio::sync::watch`

### `printer`
Normalization layer. Consumes raw protocol messages from integration crates and produces canonical `Event` envelopes. Maintains printer state machine. This is where Moonraker-specific JSON becomes generic LayerMind events.

Implementation:
- `NormalizerState` tracks last-seen values for change detection
- Temperature change threshold: 0.5°C
- Position change threshold: 0.1mm
- Speed change threshold: 1.0 mm/s
- Fan speed change threshold: 2%
- Stateful print state tracking (emits only on transitions)
- `run_from_moonraker()` async loop consuming broadcast channel

### `telemetry`
Central event pipeline. Buffers incoming envelopes, enriches with metadata,
batches writes to configured sinks. Design guarantees: never drop events,
timestamp at ingress, immutable events.

Storage-agnostic: the `Sink` trait (defined in `shared`) decouples the
pipeline from any specific storage backend. The pipeline accepts
`Arc<dyn Sink>` — PostgreSQL, TimescaleDB, SQLite, file export, or
cloud storage can all be plugged in without changing telemetry code.

### `database`
PostgreSQL storage backend via `sqlx`. Implements the `Sink` trait as
`DatabaseSink` for batched telemetry event persistence. Provides:
- Connection pooling (configurable max connections)
- Auto-registration of unknown printers
- Batch INSERT via UNNEST for high-throughput telemetry
- Repository with typed queries: recent events, print history,
  telemetry for a print, printer listing
- SQL migrations via `sqlx::migrate!()`

### `ai`
Future AI/LLM engine. Consumes Knowledge records from the Knowledge
Engine and observations from the Analyzer to generate natural-language
recommendations, answer questions, and learn from user feedback.
Currently a placeholder crate.

### `knowledge`
Stateful layer between Analyzer and future AI Engine. Transforms
observations into persistent, structured knowledge about printers.

- ObservationTracker — lifecycle management (active → acknowledged → resolved)
- PrinterProfiler — aggregating hardware, behavior, known issues, reliability
- TimelineBuilder — chronological history of important events
- KnowledgeScorer — importance (severity × repeat count) and confidence

Depends only on `shared`. Persistence via KnowledgeSink in database crate.
New tables: knowledge_observations, printer_profiles, printer_timeline.

### `context`
Query layer that synthesizes knowledge into AI-consumable printer
briefings. Subscribes to Knowledge broadcast, caches state, produces
PrinterContext on demand.

- `PrinterContext` — complete briefing: identity, health, print history,
  current state, known issues, historical patterns, evidence ledger
- Every fact carries `EvidenceQuality` (observed/inferred/confirmed)
- Designed for multiple future views: TroubleshootingContext,
  CalibrationContext, MaintenanceContext — all projections over the
  same cached state

Depends only on `shared`.

### `analyzer`
Deterministic rules engine. Consumes canonical Envelope events and
produces structured Observations. Independent of database, telemetry,
and Moonraker — depends only on `shared`.

Components:
- `PrintTracker` — per-printer print lifecycle (start → progress → complete/fail)
- `HealthMetrics` — rolling health indicators (temperature stability,
  success rate, error frequency, uptime)
- `DetectionEngine` — runs 4 rules against event windows:
  1. TemperatureStabilityRule — flags when avg deviation > 3°C
  2. ErrorFrequencyRule — flags when error/warning count exceeds threshold
  3. FailurePatternRule — flags consecutive print failures
  4. CalibrationStalenessRule — flags when calibration is > 7 days old

### `core`
Service orchestration. Loads config, initializes logging, wires the dependency graph, manages graceful shutdown. Entry point for the LayerMind daemon.

Wired pipeline:
```
MoonrakerClient.run() → broadcast(RpcMessage)
    → Printer.run_from_moonraker() → broadcast(Envelope)
        ├──→ bridge task → mpsc → TelemetryEngine.run(sink) → DatabaseSink → PostgreSQL
        └──→ AnalyzerEngine.run() → broadcast(Observation)
                └──→ KnowledgeEngine.run() → broadcast(Knowledge)
                        └──→ ContextEngine.run() → cached PrinterContext (queryable)
```

## Moonraker Integration Design

### Connection Lifecycle

1. **Connect** — `tokio_tungstenite::connect_async` to `ws://host:7125/websocket`
2. **Authenticate** — If `api_key` configured, send `access.oneshot_token`
3. **Subscribe** — JSON-RPC `printer.objects.subscribe` for 8 printer objects
4. **Receive** — Continuous stream of `notify_status_update` notifications
5. **Reconnect** — On error, close frame, or heartbeat timeout → backoff → reconnect
6. **Shutdown** — On `watch::Receiver` signal, send close frame and exit cleanly

### Subscribed Printer Objects

| Moonraker Object | LayerMind Events |
|-----------------|-----------------|
| `heater_bed` | `TemperatureUpdate` |
| `extruder` | `TemperatureUpdate` |
| `print_stats` | `PrintStarted`, `PrintPaused`, `PrintCompleted`, `PrintFailed`, `PrintCancelled`, `StateChanged` |
| `virtual_sdcard` | `PrintProgress` (with estimated time remaining) |
| `toolhead` | `PositionUpdate` |
| `motion_report` | `SpeedUpdate` |
| `gcode_move` | (future: feedrate/flow factor events) |
| `fan` | `FanUpdate` |

### Change Detection

To avoid flooding the event bus, the normalizer suppresses redundant events:

- Temperature: emit only when any sensor changes ≥ 0.5°C
- Fan: emit only when speed changes ≥ 2%
- Position: emit only when any axis moves ≥ 0.1mm
- Speed: emit only when velocity changes ≥ 1.0 mm/s
- Print state: emit only on actual state string transitions

### Reconnection Strategy

Exponential backoff: 1s → 2s → 4s → 8s → 16s → 32s → 60s (capped).
Backoff resets on successful connection. Shutdown signal during backoff
exits immediately.

### Error Handling

- WebSocket errors → reconnect with backoff
- JSON parse failures → logged, message skipped, connection maintained
- Broadcast channel full → oldest subscriber dropped (Lagged)
- Heartbeat timeout → treat as disconnection, reconnect
- Server close frame → reconnect

## Data Flow

```
Moonraker WebSocket (ws://host:7125/websocket)
        │
        ▼
  [moonraker crate]  ── RpcMessage (broadcast) ──►
        │  • tokio-tungstenite WebSocket client
        │  • JSON-RPC 2.0 subscribe/notify
        │  • Exponential backoff reconnect
        │  • Heartbeat monitoring
        │  • Graceful shutdown via watch channel
        ▼
  [printer crate]    ── Envelope (broadcast) ──►
        │  • Normalizes Moonraker objects → canonical Events
        │  • Change detection thresholds
        │  • Print state machine
        │  • Duplicate event suppression
        ▼
  [telemetry crate]  ── batch (mpsc) ──►  [database crate]
        │  • Buffer + timed flush
        │  • Never-drop guarantee
        ▼
  [ai crate]         ── Recommendation ──►  [database crate]
```

## Event Bus

Internal communication uses `tokio::sync::broadcast` channels for
fan-out (one producer → many consumers) and `tokio::sync::mpsc` for
point-to-point pipelines. Advantages:

- Loose coupling — services don't import each other
- Backpressure handling — lagging consumers get `Lagged` errors
- Easy testing — inject events directly into channels
- Zero-copy within the process

## Database Design

### Schema (5 tables)

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `printers` | Registered printer metadata | `id`, `last_seen` |
| `print_jobs` | Print job lifecycle summaries | `id`, `printer_id`, `status`, `start_time`, `end_time` |
| `telemetry_events` | Primary time-series event store | `id`, `printer_id`, `event_type`, `payload` (JSONB), `recorded_at` |
| `calibration_events` | Calibration results | `id`, `printer_id`, `cal_type`, `values` (JSONB) |
| `ai_observations` | AI-generated observations (future) | `id`, `printer_id`, `category`, `observation`, `confidence` |

### Data Lifecycle

```
Printer Event (canonical Envelope)
        │
        ▼
  TelemetryEngine::run(rx, sink)
        │
        ├── Buffer (up to config.buffer_size events)
        ├── Timed flush (config.flush_interval_secs)
        │
        ▼
  Sink::write_batch()
        │
        ▼
  DatabaseSink
        │
        ├── Auto-register unknown printers (UPSERT)
        └── Batch INSERT via UNNEST
               │
               ▼
         telemetry_events table
         (immutable, append-only)
```

### Batch Insert Strategy

Telemetry events arrive at high frequency (~10 status updates/second from
Moonraker). Row-by-row INSERT would create excessive round-trips. Instead,
`DatabaseSink` uses PostgreSQL's `UNNEST` to insert entire batches in a single
statement:

```sql
INSERT INTO telemetry_events (id, printer_id, event_type, payload, recorded_at)
SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::text[], $4::jsonb[], $5::timestamptz[])
```

This achieves near line-rate ingestion with a single database round-trip
per batch (up to 4096 rows by default).

### Index Strategy

- `idx_telemetry_printer_time` — (printer_id, recorded_at DESC) — primary query path
- `idx_telemetry_type` — (event_type) — filtering by event kind
- `idx_telemetry_job` — (print_job_id) WHERE NOT NULL — linking events to jobs

These indexes are compatible with TimescaleDB hypertable conversion.

## Future Telemetry Scaling

This section documents planned but not yet implemented scaling patterns.

### Telemetry Aggregation

High-frequency events (temperature updates at 1Hz, position at 10Hz) produce
large volumes of data. A future aggregation layer should:

- Downsample raw events into statistical summaries (min/max/avg/stddev per minute)
- Store raw data for a sliding window (e.g., 7 days)
- Store aggregated data indefinitely
- Allow query-time choice: recent precision vs. historical summary

### Raw vs. Summarized Events

| Tier | Retention | Resolution | Use Case |
|------|-----------|------------|----------|
| Hot (raw) | 7 days | Per-event | Live monitoring, debugging, AI analysis |
| Warm (1-min aggregates) | 90 days | Per-minute stats | Trend analysis, health scoring |
| Cold (1-hour aggregates) | Forever | Per-hour stats | Long-term printer history, fleet analytics |

### TimescaleDB Migration Path

Schema is already compatible. Migration steps:

1. `SELECT create_hypertable('telemetry_events', 'recorded_at');`
2. `SELECT add_compression_policy('telemetry_events', INTERVAL '7 days');`
3. `SELECT add_retention_policy('telemetry_events', INTERVAL '90 days');`

### Retention Policies

- Raw events: 7 days (configurable)
- Aggregated stats: 90 days
- Print job summaries: indefinitely
- AI observations: indefinitely
- Calibration history: indefinitely

### Compression

TimescaleDB native columnar compression can achieve 10-20x reduction on
telemetry data. JSONB payloads benefit especially from dictionary compression
due to repeated keys across events.

### Long-Running Printer Deployments

A printer running 24/7 at 10 events/second produces ~864,000 events/day.
Over a year: ~315M events. The aggregation and retention strategy above
keeps this manageable: ~6M raw events in the hot window, ~130K aggregate
rows in warm, and ~9K aggregate rows in cold — a 35,000:1 overall compression
ratio while preserving forensic detail for recent prints.

## Testing Strategy

### Unit Tests
- Backoff calculation correctness
- JSON-RPC message parsing
- Normalizer event conversion
- Change detection suppression
- Print state transitions

### Integration Tests
- Mock Moonraker WebSocket server
- Connection lifecycle (connect → receive → disconnect)
- Reconnection behavior
- Graceful shutdown
- Message flow end-to-end

## Design Decisions

### Why Rust?
Performance, reliability, memory safety, cross-platform support. The data pipeline needs to be fast and never crash. Rust is the right tool for infrastructure software.

### Why NOT Python for the core?
Python is used only for ML experimentation. The production system runs on Rust. Python models are served via a sidecar or ONNX runtime.

### Why PostgreSQL + TimescaleDB?
Telemetry data is time-series. TimescaleDB gives us automatic partitioning, compression, and retention policies on top of standard PostgreSQL. We get the best of both worlds: relational queries for entities and hypertables for telemetry.

### Why event-driven?
A 3D printer generates a continuous stream of events. An event-driven architecture naturally models this. It also allows pluggable consumers — the AI engine, database writer, and future notification system are all just subscribers.

### Why not gRPC/HTTP internally?
Internal services run in the same process. Cross-crate communication uses in-memory channels for zero-copy, zero-latency event delivery. A future split into microservices could add gRPC, but that's premature optimization.

### Why change detection in the normalizer?
Raw Moonraker pushes status updates at ~10Hz regardless of whether values changed. Without suppression, the telemetry pipeline would drown in redundant TemperatureUpdate events. The normalizer is the right place because it understands the semantics of the data.

## Scaling Considerations

### Single Printer → Print Farm
The `printer_id` field on every event means the system is multi-printer from day one. A print farm is just multiple printer instances feeding the same telemetry pipeline.

### Local → Cloud
The telemetry pipeline writes to a configurable sink. Local-only mode writes to SQLite or files. Cloud mode writes to PostgreSQL. The architecture doesn't change — just the sink implementation.

## Security & Privacy

- All data stays local by default
- No telemetry phones home
- Cloud features will be opt-in
- API keys stored with filesystem permissions
