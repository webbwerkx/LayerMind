# LayerMind Roadmap

## Phase 1: Foundation & Observation (Current)

**Goal: LayerMind becomes the best printer data collection system.**

### Milestone 1.1 — Project Foundation
- [x] Rust workspace setup
- [x] Crate structure with clear boundaries
- [x] Shared types and error handling
- [x] Configuration system
- [x] Structured logging
- [x] Architecture documentation
- [x] Project builds and passes checks
- [x] Git repository initialized
- [x] README, ARCHITECTURE, ROADMAP, CONTRIBUTING written

### Milestone 1.2 — Moonraker Integration ✅
- [x] WebSocket connection with authentication
- [x] Subscription to printer objects (8 objects: heater_bed, extruder, print_stats, virtual_sdcard, toolhead, motion_report, gcode_move, fan)
- [x] Automatic reconnection with exponential backoff (1s→60s cap)
- [x] JSON-RPC message parsing and typed status update model
- [x] Event normalization with change detection (temp 0.5°C, position 0.1mm, speed 1.0mm/s, fan 2%)
- [x] Print state machine (idle→printing→paused→complete/failed/cancelled)
- [x] Integration tests with mock Moonraker WebSocket server (3 tests)
- [x] Unit tests for backoff, parsing, normalization, change suppression (7 tests)
- [x] Live pipeline wired in core: Moonraker → Printer → Telemetry
- [x] Graceful shutdown via watch channel
- [x] Heartbeat monitoring with configurable interval

### Milestone 1.3 — Telemetry Pipeline ✅
- [x] PostgreSQL storage via sqlx with connection pooling
- [x] Schema: printers, print_jobs, telemetry_events, calibration_events, ai_observations
- [x] Batch INSERT via UNNEST for high-throughput telemetry
- [x] Printer auto-registration on first event
- [x] Repository queries: recent events, print history, telemetry for print
- [x] Storage-agnostic Sink trait (shared crate, async)
- [x] DatabaseSink implementing Sink for PostgreSQL
- [x] Graceful degradation: in-memory sink when DB unavailable
- [x] Migrations via sqlx::migrate!()
- [x] Unit tests: event type mapping, uniqueness (3 tests)
- [x] Integration tests: auto-registration, persistence, idempotency (3 tests, DB-optional)
- [x] Database design docs + future scaling strategy in ARCHITECTURE.md
- [x] Live pipeline: Moonraker → Printer → Telemetry → DatabaseSink

### Milestone 1.4 — Analyzer Engine ✅
- [x] Deterministic rules engine (crates/analyzer) — no AI/LLM dependency
- [x] PrintTracker: per-printer print lifecycle tracking (start→progress→complete/fail)
- [x] HealthMetrics: temperature stability, success rate, error frequency, uptime
- [x] TemperatureStabilityRule — flags when avg deviation > 3°C
- [x] ErrorFrequencyRule — flags when error/warning count exceeds thresholds
- [x] FailurePatternRule — flags consecutive print failures (2=warning, 5=critical)
- [x] CalibrationStalenessRule — flags when calibration > 7 days old
- [x] Observation type in shared crate (PrintLifecycle, PrintSummary, HealthSnapshot, AnomalyDetected)
- [x] AnalyzerEngine consumes printer broadcast, produces Observation broadcast
- [x] Wired into core pipeline (5 tasks: moonraker, printer, bridge, telemetry, analyzer)
- [x] 17 unit tests: metrics, print tracker, all 4 rules (edge cases included)
- [x] Architecture docs updated

### Milestone 1.5 — Knowledge Engine ✅
- [x] Knowledge types in shared crate (Knowledge, KnowledgeKind, PrinterProfile, TimelineEntry)
- [x] ObservationTracker — lifecycle management (active → acknowledged → resolved)
- [x] PrinterProfiler — aggregating per-printer profiles (hardware, behavior, known issues)
- [x] TimelineBuilder — chronological printer event history with milestones
- [x] KnowledgeScorer — importance (severity × repeat penalty) and confidence (evidence-based)
- [x] KnowledgeEngine subscribes to analyzer broadcast, produces Knowledge broadcast
- [x] Database migration 002: knowledge_observations, printer_profiles, printer_timeline
- [x] Wired into core pipeline (6 tasks: moonraker, printer, bridge, telemetry, analyzer, knowledge)
- [x] 18 unit tests: scoring ×4, tracker ×4, profiler ×5, timeline ×5
- [x] Architecture docs updated

### Milestone 1.6 — AI Context Engine ✅
- [x] Context types in shared (PrinterContext, PrinterSummary, PrintHistorySummary, etc.)
- [x] EvidenceQuality enum: Observed (sensor), Inferred (rule), Confirmed (human)
- [x] Evidence struct: fact_type, statement, quality, confidence, timestamp, source_id
- [x] ContextEngine subscribes to Knowledge broadcast, caches per-printer state
- [x] query context(printer_id) → PrinterContext (AI-ready briefing)
- [x] Designed for multi-view: TroubleshootingContext, CalibrationContext, MaintenanceContext (future)
- [x] Profile updates populate print history, known issues, patterns, health, reliability
- [x] Timeline events populate failures, evidence ledger
- [x] Wired into core pipeline (7 tasks: moonraker, printer, bridge, telemetry, analyzer, knowledge, context)
- [x] 4 unit tests: empty context, profile population, resolved issue clearing, health includes reliability
- [x] Architecture docs updated

### Milestone 1.7 — Basic Dashboard
- [ ] CLI tool for printer status
- [ ] Live temperature display
- [ ] Print progress monitoring
- [ ] Event history viewer

---

## Phase 2: AI Reasoning

**Goal: LayerMind provides trustworthy, evidence-backed AI recommendations.**

### Milestone 2.1 — AI Reasoning Architecture ✅
- [x] Recommendation types in shared (Recommendation, Action, Reference, TrustAssessment)
- [x] EvidenceQuality provenance: Observed (sensor), Inferred (rule), Confirmed (human)
- [x] AiUsage tracking: provider, model, tokens, estimated cost
- [x] Feedback types: Fixed, NotFixed, Incorrect, Helpful (future learning loop)
- [x] AiProvider trait — swappable between OpenAI, OpenRouter, local models
- [x] OpenAiProvider — covers entire /v1/chat/completions ecosystem
- [x] MockProvider for tests — no real API key required
- [x] PromptBuilder — PrinterContext → system + user prompts
- [x] ResponseParser — handles valid/malformed JSON, missing fields, recovery
- [x] TrustValidator — deterministic cross-reference of AI claims vs context evidence
- [x] PrintDoctor — full end-to-end diagnostic pipeline
- [x] 17 unit tests: parser ×7, prompt ×3, trust ×4, diagnostic ×2, openai ×1
- [x] crates/ai marked deprecated

### Milestone 2.2 — Wire Print Doctor into Core ✅
- [x] diagnose_printer() public API in core
- [x] ContextStore → PrintDoctor → ValidatedRecommendation flow
- [x] Typed errors (MissingContext, ProviderError)
- [x] Provider-agnostic (MockProvider for tests)
- [x] TrustValidator always executes
- [x] 11 integration tests covering happy path and all failure modes

### Milestone 2.3 — Advanced Diagnostics ✅
- [x] Multi-issue diagnosis (AI identifies all active issues)
- [x] Historical comparison with trend labels (NEW, RECURRING, WORSENING, etc.)
- [x] Deterministic confidence calibration (evidence quantity, quality, recency, agreement, conflicts)
- [x] Evidence ranking (recency × confidence × repetition × severity)
- [x] Recommendation prioritization (health-impact, safety, historical relevance)
- [x] Contradiction detection (5 rules: resolved/active, temp stability, success vs failures, idle state, opposing categories)
- [x] Explainability (ExplanationFactor per action: reason, evidence_refs, assumptions, observation_type)
- [x] Prompt optimization (structured sections, ranked evidence, contradiction inclusion)
- [x] TrustValidator enhancements (historical agreement, multi-source matching, contradiction awareness)
- [x] New modules: evidence.rs, contradiction.rs, confidence.rs, prioritization.rs
- [x] 34 reasoning tests (was 17, +17 new)
- [x] 100 total workspace tests

### Milestone 2.4 — Local Models & Streaming
- [ ] Local model support (llama.cpp, Ollama via same provider interface)
- [ ] Streaming responses for long-running diagnostics
- [ ] Temperature/config tuning per diagnostic type

### Milestone 2.5 — Learning Loop
- [ ] User feedback collection (confirm/deny recommendations)
- [ ] Feedback → knowledge update pipeline
- [ ] Model preference tracking per printer
- [ ] Anomaly detection on sensor data
- [ ] Mechanical issue detection (ringing, layer shifts)

### Milestone 2.2 — AI Analysis
- [ ] Python ML service integration
- [ ] Print failure image analysis
- [ ] Pattern recognition across print history
- [ ] Confidence scoring for recommendations

### Milestone 2.3 — Recommendations
- [ ] PID tuning recommendations
- [ ] Acceleration/speed optimization
- [ ] Z-offset calibration assistance
- [ ] Maintenance scheduling
- [ ] Filament profile suggestions

---

## Phase 3: Learning & Optimization

**Goal: LayerMind learns from every print.**

### Milestone 3.1 — Printer Health Score
- [ ] Multi-factor health scoring
- [ ] Trend analysis over time
- [ ] Predictive maintenance alerts
- [ ] Component wear tracking

### Milestone 3.2 — Print Intelligence
- [ ] Post-print analysis and summary
- [ ] Success/failure root cause analysis
- [ ] Comparative analysis (this print vs. best print)
- [ ] Automated profile optimization

### Milestone 3.3 — Learning System
- [ ] User feedback incorporation
- [ ] Successful print pattern learning
- [ ] Failed print pattern learning
- [ ] Calibration result tracking and optimization

---

## Phase 4: Desktop Application

**Goal: Native desktop experience.**

### Milestone 4.1 — Tauri Shell
- [ ] Cross-platform window management
- [ ] System tray integration
- [ ] Auto-start capability
- [ ] Native notifications

### Milestone 4.2 — Dashboard UI
- [ ] Printer overview
- [ ] Real-time telemetry charts
- [ ] Print history browser
- [ ] Recommendation inbox
- [ ] Settings management

### Milestone 4.3 — Advanced UI
- [ ] Multi-printer fleet view
- [ ] Print comparison tools
- [ ] Calibration wizards
- [ ] Dark/light themes

---

## Phase 5: Fleet & Enterprise

**Goal: Manage multiple printers at scale.**

### Milestone 5.1 — Fleet Management
- [ ] Multi-printer dashboard
- [ ] Fleet-wide analytics
- [ ] Comparative printer metrics
- [ ] Batch operations

### Milestone 5.2 — Enterprise Features
- [ ] Role-based access control
- [ ] Audit logging
- [ ] Data export and reporting
- [ ] API for third-party integration

### Milestone 5.3 — Cloud (Opt-in)
- [ ] Optional cloud sync
- [ ] Fleet management from anywhere
- [ ] Community-shared profiles
- [ ] Anonymous failure pattern sharing

---

## Future Horizons

- **Material intelligence**: automatic material detection and profile selection
- **Multi-material optimization**: purge tower optimization, wipe strategies
- **Slicer integration**: direct feedback loop with slicer settings
- **Industrial support**: ISO/ASTM compliance features, traceability
- **Marketplace**: community-contributed profiles, calibrations, recommendations
