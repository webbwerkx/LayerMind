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

### `printer`
Normalization layer. Consumes raw protocol messages from integration crates and produces canonical `Event` envelopes. Maintains printer state machine. This is where Moonraker-specific JSON becomes generic LayerMind events.

### `telemetry`
Central event pipeline. Buffers incoming envelopes, enriches with metadata, batches writes to configured sinks (database, file, AI engine). Design guarantees: never drop events, timestamp at ingress, immutable events.

### `database`
PostgreSQL schema, migrations, and typed query interfaces. Entity models: Printer, PrintJob, TelemetryEvent, Filament, Failure, Recommendation, etc.

### `ai`
Event-driven intelligence engine. Subscribes to telemetry, runs detectors for known patterns (temperature instability, ringing, first layer failures), and generates structured recommendations. NOT a chatbot — background service producing structured output.

### `core`
Service orchestration. Loads config, initializes logging, wires the dependency graph, manages graceful shutdown. Entry point for the LayerMind daemon.

## Data Flow

```
Moonraker WebSocket
        │
        ▼
  [moonraker crate]  ── RawMessage ──►
        │
        ▼
  [printer crate]    ── Envelope ──►
        │
        ▼
  [telemetry crate]  ── batch ──►  [database crate]
        │
        ▼
  [ai crate]         ── Recommendation ──►  [database crate]
```

## Event Bus

Internal communication uses `tokio::sync::broadcast` channels. Each service publishes to its channel; consumers subscribe. This provides:

- Loose coupling — services don't know about each other
- Backpressure handling — lagging consumers get `Lagged` errors and can recover
- Easy testing — inject events directly into channels

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
