# LayerMind — Architecture & Status

> Auto-generated reference document. Last updated: 2025-07-23.
> Reflects project state after Phase 4 freeze.
>
> **Phase 4: Optimization Engine complete. 213 tests. 18 packages. 0 clippy errors.**

---

## Phase Completion Status

| Phase | Status | Tests | Key |
|-------|--------|-------|-----|
| 1 — Telemetry & Data Collection | ✅ | 55 | Moonraker, database, telemetry, analyzer, knowledge, context |
| 2 — AI Diagnostic Foundation | ✅ | 86 | Provider abstraction, PrintDoctor, trust, confidence, strategies |
| 2.5.1 — Foundation Freeze | ✅ | 26 | Provider tests, config, retry, streaming, doc fixes |
| 2.6 — Machine Intelligence | ✅ | 22 | MachineProfile, CapabilityEngine, hardware library |
| 3.0 — Historical Timeline | ✅ | 17 | TimelineStore, QueryEngine, Snapshots, Diffs |
| 3.0.1 — Runtime Application | ✅ | 5 | CLI, bootstrap, commands (test/monitor/diagnose/run) |
| 3.1 — Learning Engine | ✅ | 13 | Pattern detection, trend analysis, print comparison, calibration tracking |
| 3.2 — Failure Prediction | ✅ | 19 | Component health, aging predictions, early warnings |
| 4 — Optimization Engine | ✅ | 28 | Tuning suggestions, calibration plans, maintenance actions |

---

## Crate Map

```
crates/
├── shared/          Canonical types (19 modules). Zero business logic.
├── config/          Typed configuration (env vars, XDG).
├── logging/         Tracing subscriber.
├── moonraker/       WebSocket client (JSON-RPC 2.0).
├── printer/         Event normalization. Moonraker → canonical Envelope.
├── telemetry/       Pipeline: buffer → batch → Sink. Never-drop guarantee.
├── database/        PostgreSQL via sqlx. DatabaseSink, migrations.
├── analyzer/        Deterministic rules engine (4 detection rules).
├── knowledge/       Stateful knowledge: Tracker, Profiler, Timeline, Scorer.
├── context/         Query layer. ContextEngine + ContextStore (Arc).
├── reasoning/       AI diagnostic pipeline. PrintDoctor, 9 stages.
├── ai/              Provider implementations. 3 providers, 10+ backends.
├── machine/         Hardware intelligence. Discovery, capability, confidence.
├── history/         Timeline: immutable events, snapshots, diffs, queries.
├── learning/        Pattern detection, trends, prediction, optimization.
└── core/            Orchestration. Wires all tasks, lifecycle.

apps/
└── layermind/       Runtime binary. CLI entry point.
```

## Dependency Graph

```
shared ← config, logging, moonraker, printer, telemetry, database,
         analyzer, knowledge, context, reasoning, machine, history, learning
reasoning ← ai (provider implementations)
core ← all of the above
apps/layermind ← core + all crates
```

Zero cycles. Reasoning has zero HTTP deps. Learning depends only on shared.

---

## Runtime Architecture

```
Moonraker WebSocket
    │
    ▼
Printer (normalization) ──→ Telemetry → DatabaseSink → PostgreSQL
    │
    ├──→ Analyzer → Knowledge → ContextStore ──→ diagnose_printer()
    │                                              │
    │                                    PrintDoctor → AiProvider → ValidatedRecommendation
    │
    └──→ (future) History recording
              │
         TimelineStore ──→ LearningEngine::analyze()
              │
         BehaviorSummary { patterns, trends, aging, health, optimization }
              │
         ContextStore.set_learning() ──→ PrinterContext.learning
```

## Data Flow Layers

| Layer | Crate | Output |
|-------|-------|--------|
| Raw telemetry | moonraker, printer, telemetry | 10Hz sensor data |
| Analysis | analyzer, knowledge | Observations |
| Organization | context | PrinterContext |
| AI reasoning | reasoning, ai | ValidatedRecommendation |
| Hardware | machine | MachineProfile, CapabilitySet |
| Memory | history | TimelineEvent, Snapshot |
| **Learning** | **learning** | **BehaviorSummary → patterns, trends, health, optimization** |

## What LayerMind Knows About Every Printer

1. **Current State** (Phase 1) — what's happening NOW
2. **Machine Intelligence** (Phase 2.6) — what hardware EXISTS
3. **Historical Knowledge** (Phase 3.0) — what CHANGED and WHEN
4. **Behavioral Patterns** (Phase 3.1) — what RECURS and what TRENDS
5. **Component Health** (Phase 3.2) — what's DEGRADING, what will FAIL
6. **Optimization Opportunities** (Phase 4) — what to TUNE, CALIBRATE, MAINTAIN

## Integration Gaps (not yet wired)

| Gap | Impact |
|-----|--------|
| Runtime periodic analysis loop | BehaviorSummary never computed during daemon run |
| Hardware discovery from Moonraker | `MachineProfileBuilder::discover_hardware()` returns defaults |
| Telemetry→History bridge | TimelineEvents never auto-generated from telemetry |

## Key Design Decisions

- Provider-agnostic AI (single `AiProvider` trait, 10+ backends)
- Deterministic pipeline (every non-AI step is mechanical)
- Property<T> confidence model (source + confidence on every fact)
- Strong typing (28 enums in machine crate, 8 event categories in history)
- Append-only timeline (never delete, never modify events)
- Threshold-based learning (explicit constants, no ML models)
- Human-approval gating (all suggestions advisory, never auto-applied)

## Next: Integration

The three wiring gaps above. After those, `layermind monitor` against a real printer shows everything.

---

*Updated after Phase 4 freeze.*
