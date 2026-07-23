# LayerMind — Architecture & Status

> Auto-generated reference document. Last updated: 2025-07-21.
> Reflects the state of the project after Phase 2.5 freeze.

---

## Project Overview

LayerMind is an AI-powered operating system for additive manufacturing (3D printing). It connects to printers via Moonraker's WebSocket API, builds a real-time knowledge graph of printer state, and provides evidence-backed AI diagnostics through a provider-agnostic reasoning pipeline.

- **Language**: Rust (workspace of 13 crates)
- **Database**: PostgreSQL via sqlx (TimescaleDB-ready schema)
- **AI**: Provider-agnostic via `AiProvider` trait — supports OpenAI, OpenRouter, Anthropic, Gemini, Ollama, LM Studio, vLLM, LocalAI, llama.cpp
- **Tests**: 115 (as of Phase 2.5)
- **Version**: 0.2.0 (recommended after Phase 2 freeze)

---

## Crate Map

```
crates/
├── shared/          Canonical types, error enums, Sink trait. Zero business logic.
│                    Every crate depends on this; this depends on nothing internal.
├── config/          Typed configuration loading (env vars, future XDG files).
├── logging/         Tracing subscriber init (human-readable + JSON output).
├── moonraker/       WebSocket client for Moonraker's JSON-RPC 2.0 protocol.
├── printer/         Normalization layer. Moonraker → canonical Envelope events.
│                    Change detection (temp 0.5°C, position 0.1mm, speed 1.0mm/s, fan 2%).
├── telemetry/       Event pipeline: buffer → batch → Sink. Never-drop guarantee.
├── database/        PostgreSQL via sqlx. DatabaseSink, Repository, migrations.
├── analyzer/        Deterministic rules engine. 4 detection rules, PrintTracker, HealthMetrics.
├── knowledge/       Stateful knowledge layer. Tracker, Profiler, Timeline, Scorer.
├── context/         Query layer. ContextEngine (ingestion) + ContextStore (Arc, queryable).
├── reasoning/       AI diagnostic pipeline. PrintDoctor, 9 deterministic stages, provider-agnostic.
├── ai/              Provider implementations. OpenAiCompatible, Anthropic, Gemini. Retry wrapper.
└── core/            Orchestration. Wires all tasks, manages lifecycle, exposes diagnose_printer().
```

**Dependency Graph:**
```
shared ← config, logging, moonraker, printer, telemetry, database, analyzer, knowledge, context, reasoning
                                                                                                            ↑
reasoning ← ai (provider implementations)
                                                                                                            ↑
core ← all of the above
```

No cycles. `reasoning` has zero HTTP/networking dependencies. `ai` owns all provider networking.

---

## Phase Completion Status

### Phase 1 — Foundation & Observation ✅

| Milestone | Status | Tests | Key Deliverables |
|-----------|--------|-------|-----------------|
| 1.1 Project Foundation | ✅ | — | Workspace, 9 crates, docs, git init |
| 1.2 Moonraker Integration | ✅ | 4 | WebSocket client, JSON-RPC 2.0, 8 printer objects, reconnect with backoff |
| 1.3 Memory Engine | ✅ | 6 | Sink trait, DatabaseSink, batch UNNEST, auto-registration, graceful MemorySink fallback |
| 1.4 Analyzer Engine | ✅ | 17 | 4 detection rules, PrintTracker, HealthMetrics, Observation broadcast |
| 1.5 Knowledge Engine | ✅ | 18 | Tracker, Profiler, Timeline, Scorer, database migration 002 |
| 1.6 Context Engine | ✅ | 4 | PrinterContext, EvidenceQuality, ContextEngine + ContextStore separation |
| 1.7 Basic Dashboard | ⏳ | — | Deferred |

### Phase 2 — AI Reasoning ✅

| Milestone | Status | Tests | Key Deliverables |
|-----------|--------|-------|-----------------|
| 2.1 AI Architecture | ✅ | 17 | AiProvider trait, PrintDoctor, PromptBuilder, ResponseParser, TrustValidator, MockProvider |
| 2.2 Core Integration | ✅ | 11 | diagnose_printer() in core, ContextStore→PrintDoctor flow, typed errors |
| 2.3 Advanced Diagnostics | ✅ | 34 | ContradictionDetector, EvidenceRanker, ConfidenceCalibrator, Prioritizer, explainability, historical trends |
| 2.4 Universal Providers | ✅ | 33 | OpenAiCompatible (7+ backends), Anthropic, Gemini, retry wrapper, streaming trait, ProviderConfig |
| 2.5 Diagnostic Strategies | ✅ | 39 | Rapid/Standard/Thorough presets, DiagnosticOrchestrator, strategy flows through pipeline |

### Phase 3 — Learning & Optimization (Future)

| Milestone | Status |
|-----------|--------|
| 3.1 Printer Health Score | ⏳ |
| 3.2 Print Intelligence | ⏳ |
| 3.3 Learning System | ⏳ |

### Phase 4 — Desktop Application (Future)

| Milestone | Status |
|-----------|--------|
| 4.1 Tauri Shell | ⏳ |
| 4.2 Dashboard UI | ⏳ |
| 4.3 Advanced UI | ⏳ |

### Phase 5 — Fleet & Enterprise (Future)

| Milestone | Status |
|-----------|--------|
| 5.1 Fleet Management | ⏳ |
| 5.2 Enterprise Features | ⏳ |
| 5.3 Cloud (Opt-in) | ⏳ |

---

## Runtime Architecture

### Data Pipeline (7 concurrent tasks in core)

```
Moonraker WebSocket (ws://host:7125/websocket)
        │
        ▼
  MoonrakerClient.run()  ──── broadcast(RpcMessage)
        │
        ▼
  Printer.run_from_moonraker() ──── broadcast(Envelope)
        │
        ├──→ bridge task → TelemetryEngine.run(&sink) → DatabaseSink → PostgreSQL
        │
        └──→ AnalyzerEngine.run() ──── broadcast(Observation)
                │
                ▼
          KnowledgeEngine.run() ──── broadcast(Knowledge)
                │
                ▼
          ContextEngine.run() → ContextStore (Arc, std::sync::RwLock)
                │
                ▼
          diagnose_printer(store, doctor, printer_id)
                │
                ▼
          PrintDoctor.diagnose(&context) → ValidatedRecommendation
```

### AI Diagnostic Pipeline (10 stages, 9 deterministic)

```
PrinterContext
  │
  ├─ 1. ContradictionDetector::detect()          → Vec<Contradiction>
  ├─ 2. EvidenceRanker::rank(strategy)           → RankedContext
  ├─ 3. PromptBuilder::build(strategy)           → PromptPair
  ├─ 4. AiProvider::complete()                   → AiResponse          [only non-deterministic]
  ├─ 5. ResponseParser::parse()                  → ParsedRecommendation
  ├─ 6. ConfidenceCalibrator::calibrate()        → adjusted f64
  ├─ 7. Prioritizer::prioritize()                → Vec<Action> (reordered)
  ├─ 8. TrustValidator::validate()               → TrustAssessment
  └─ 9. build_explanation_factors()              → Vec<ExplanationFactor>

ValidatedRecommendation { recommendation, trust, disclaimers,
                          explanation_factors, contradictions }
```

### Provider Architecture

```
AiProvider trait (reasoning)
  ├── complete(AiRequest) → AiResponse
  ├── name() → &str
  ├── model() → &str
  └── supports_structured_output() → bool

Implementations (ai crate):
  OpenAiCompatibleProvider ─── /v1/chat/completions (OpenAI, OpenRouter,
  │                            Ollama, LM Studio, vLLM, LocalAI, llama.cpp)
  AnthropicProvider         ─── /v1/messages (native)
  GeminiProvider            ─── /v1beta/models/{model}:generateContent (native)

Infrastructure:
  RetryingProvider<T>       ─── wraps any provider with exponential backoff
  StreamingAiProvider (opt) ─── additive trait for future streaming

Factory:
  create_provider(&ProviderConfig) → Arc<dyn AiProvider>
  Supports: openai, openrouter, anthropic, gemini, ollama, custom
```

### Diagnostic Strategies

```
DiagnosticStrategy { max_evidence, max_issues, max_observations, ... }

  RAPID    (5/3/5, 512 tok, 0.5 temp)  ─── fast, minimal context
  STANDARD (15/10/10, 1024 tok, 0.3)   ─── Phase 2.4 default
  THOROUGH (25/15/15, 2048 tok, 0.1)  ─── deep analysis

DiagnosticOrchestrator  ─── selects strategy → PrintDoctor::with_strategy() → diagnose()
```

---

## Test Matrix (115 total)

| Crate | Tests | Coverage |
|-------|-------|----------|
| analyzer | 17 | Detection rules, metrics, print tracker |
| context | 4 | Empty context, profile population, resolved clearing, health |
| core | 21 | Integration: full flow, errors, multi-issue, confidence, prioritization, contradictions, explainability, strategies |
| database | 6 | Event mapping, auto-registration, persistence, idempotency |
| knowledge | 18 | Scoring, tracker lifecycle, profiler, timeline |
| moonraker | 4 | Backoff, parsing, integration |
| printer | 6 | Normalizer, change detection, state machine |
| reasoning | 39 | Parser, prompt, trust, diagnostic, evidence, confidence, prioritization, contradiction, strategy |
| **Total** | **115** | |

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Rust primary language | Performance, reliability, memory safety for infrastructure software |
| Shared types crate | Single source of truth for all cross-crate contracts. Zero business logic |
| Sink trait in shared | Storage-agnostic telemetry pipeline. PostgreSQL/SQLite/files/cloud — same trait |
| Broadcast channels | Loose coupling between pipeline stages. Easy testing, zero-copy in-process |
| Provider-agnostic AI | Single `AiProvider` trait covering 10+ backends. PrintDoctor never sees HTTP |
| AI crate for implementations | Reasoning has zero networking deps. All provider HTTP lives in `ai` |
| Deterministic pipeline | Every non-AI step is reproducible. Confidence, prioritization, trust all mechanical |
| ContextStore via Arc<RwLock> | Queryable cache separated from ingestion engine. No lock held across await |
| Strategy as config, not enum | Custom strategies are struct literals. No match-everywhere overhead |
| One AI request per diagnosis | Simplicity. Multi-step reasoning, chaining, agent loops deferred to future phases |

---

## Extension Points

| Area | How to extend |
|------|--------------|
| New AI backend (OpenAI-compatible) | 1 line in `create_provider()` factory |
| New AI backend (native API) | ~100-line provider file + 1 factory arm |
| New diagnostic strategy | `DiagnosticStrategy { ... }` literal — no code changes |
| New detection rule | 1 rule file in analyzer + registration in DetectionEngine |
| New knowledge consumer | Subscribe to Knowledge broadcast, like ContextEngine does |
| New context view | Add method to ContextStore (e.g. `calibration_context()`) |
| New storage backend | Implement `Sink` trait in shared |
| Streaming AI responses | Implement `StreamingAiProvider` on providers, add stream path in PrintDoctor |
| Provider failover | `FallbackProvider` wrapping `Vec<Arc<dyn AiProvider>>` |
| Conversational diagnosis | New `ChatDoctor` alongside PrintDoctor, uses same ContextStore |
| CLI / REST API | Call `diagnose_printer(store, doctor, id)` from any consumer |
| Multi-printer fleet | `printer_id` on every event. Architecture is multi-printer by design |

---

## Technical Debt Register

All items are non-blocking, low-effort improvements documented during architecture reviews.

| # | Source | Issue | Priority | Target |
|---|--------|-------|----------|--------|
| 1 | `reasoning/src/evidence.rs` | `ScoredEvidence/Issue/Observation` are `pub` but only used crate-internally. Should be `pub(crate)` | Low | 3.x |
| 2 | `reasoning/src/prompt.rs` | `system_prompt()` and `user_prompt()` are `pub` but only called from `build()`. Should be `pub(crate)` | Low | 3.x |
| 3 | `reasoning/src/evidence.rs` | `score_evidence()` adds constant 0.45 weight that doesn't differentiate. Remove or document | Low | 3.x |
| 4 | `core/src/lib.rs:102` | `_printer_tx` clones a broadcast Sender and immediately drops it | Low | 3.x |
| 5 | `context/src/store.rs:190` | `printer_count()` defined but never called. Useful for future dashboard | Trivial | 3.x |
| 6 | `reasoning/src/provider.rs` | `AiError::NotConfigured` variant unused after Phase 2.4 | Low | 3.x |
| 7 | `ai/src/lib.rs` | No unit tests for `create_provider()` factory | Low | 3.x |

---

## Future Horizons (from ROADMAP.md)

- **Material intelligence**: automatic material detection and profile selection
- **Multi-material optimization**: purge tower optimization, wipe strategies
- **Slicer integration**: direct feedback loop with slicer settings
- **Industrial support**: ISO/ASTM compliance features, traceability
- **Marketplace**: community-contributed profiles, calibrations, recommendations

---

## Quick Reference: How Things Connect

```
User wants to diagnose printer "ender3":
  1. Config::load() → reads LAYERMIND_PROVIDER, LAYERMIND_MODEL env vars
  2. create_provider(&config.provider) → Arc<dyn AiProvider>
  3. PrintDoctor::new(provider) or DiagnosticOrchestrator::diagnose()
  4. context_store.context("ender3") → Option<PrinterContext>
  5. doctor.diagnose(&context) → ValidatedRecommendation
  6. validated.recommendation.actions[0].suggested_command → "PID_CALIBRATE ..."
```

```
Adding a new AI backend (e.g., Groq):
  1. ai/src/providers/groq.rs — impl AiProvider for GroqProvider
  2. ai/src/lib.rs create_provider() — add "groq" arm
  3. ai/src/providers/mod.rs — pub use
  Done. PrintDoctor, reasoning, core — zero changes.
```

```
Custom diagnostic strategy:
  let my_strategy = DiagnosticStrategy {
      max_evidence: 8,
      max_tokens: 768,
      temperature: 0.4,
      ..DiagnosticStrategy::STANDARD  // inherit rest
  };
  let result = DiagnosticOrchestrator::diagnose_with_strategy(
      &ctx, provider, my_strategy
  ).await;
```

---

*Generated from project state after Phase 2.5 freeze. See ARCHITECTURE.md for detailed design rationale, ROADMAP.md for future milestones.*
