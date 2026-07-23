# LayerMind Roadmap

## Phase 1 — Telemetry & Data Collection ✅

Goal: LayerMind becomes the best printer data collection system.

- [x] Moonraker WebSocket integration (JSON-RPC 2.0, 8 printer objects)
- [x] Database persistence (PostgreSQL, TimescaleDB-ready)
- [x] Telemetry pipeline (buffer → batch → Sink, never-drop guarantee)
- [x] Analyzer engine (4 detection rules, PrintTracker, HealthMetrics)
- [x] Knowledge engine (Tracker, Profiler, Timeline, Scorer)
- [x] Context engine (PrinterContext, EvidenceQuality, ContextStore)

---

## Phase 2 — AI Diagnostic Foundation ✅

Goal: Provider-agnostic AI diagnostics with deterministic trust.

- [x] AI provider abstraction (AiProvider trait, provider-agnostic)
- [x] Universal provider support (OpenAI-compatible ×7, Anthropic, Gemini)
- [x] Retry with exponential backoff (RetryingProvider)
- [x] Streaming trait (additive, not yet wired)
- [x] Prompt pipeline (PromptBuilder, ranked evidence, contradictions)
- [x] Structured output parsing (JSON + markdown-wrapped JSON)
- [x] Trust validation (deterministic keyword cross-reference)
- [x] Confidence calibration (deterministic, no AI)
- [x] Evidence ranking (recency + confidence + relevance)
- [x] Contradiction detection (5 deterministic rules)
- [x] Recommendation prioritization
- [x] Explainability (ExplanationFactor per action)
- [x] Diagnostic strategies (Rapid / Standard / Thorough)
- [x] DiagnosticOrchestrator (strategy selection → PrintDoctor)
- [x] Architecture freeze (Phase 2.5)
- [x] Foundation freeze — stabilization (Phase 2.5.1)

---

## Phase 2.6 — Machine Intelligence Foundation ✅

Goal: LayerMind knows what hardware physically exists in each printer.

- [x] MachineProfile with fully typed hardware components
- [x] Property<T> confidence model (source + confidence 0.0–1.0)
- [x] InformationSource enum (Moonraker, Config, HardwareProfile, Inference, etc.)
- [x] MachineIdentity (printer_id, manufacturer, model, firmware, motion type)
- [x] MachineHardware (20+ typed component categories)
- [x] MotionSystem (axes, drivers, endstops, rails, build volume)
- [x] Component<T> generic wrapper (id, name, details, known_profile, timestamps)
- [x] DriverChip enum (TMC2208–TMC2240, A4988, etc.) with capability methods
- [x] All component enums: ProbeType, HotendType, ExtruderType, SensorType, etc.
- [x] CapabilitySet (25+ derived capabilities, all Property<bool>)
- [x] HardwareDiscovery engine (parses Moonraker system_info, printer_info)
- [x] CapabilityEngine (deterministic derivation from hardware → capabilities)
- [x] ConfidenceEngine (calibrates confidence based on corroboration)
- [x] HardwareLibrary (compiled-in profiles: BLTouch, TMC2209, Dragon HF, Beacon, etc.)
- [x] MachineProfileBuilder (orchestrator: discovery → capability → confidence → profile)
- [x] HardwareHistory types (HardwareChange, HardwareChangeKind)
- [x] ConfigurationSnapshot types
- [x] Context integration (PrinterContext.machine: Option<MachineProfile>)
- [x] ContextStore.set_machine() method
- [x] PromptBuilder "Machine Intelligence" section
- [x] 22 machine crate tests + full integration

---

## Phase 3 — Learning Engine

Goal: LayerMind learns how THIS specific printer behaves over time.

- [ ] Print history analysis
- [ ] Failure clustering
- [ ] Pattern recognition (NEW / RECURRING / WORSENING → automated)
- [ ] Long-term memory
- [ ] Root-cause relationship inference
- [ ] Knowledge evolution
- [ ] Machine behavior modeling
- [ ] Predictive diagnostics
- [ ] Digital Twin evolution (MachineProfile + operational history)
- [ ] Feedback loop (user confirms/rejects recommendations → learning)

---

## Phase 4 — Autonomous Intelligence

Goal: LayerMind acts on its knowledge.

- [ ] Calibration planning
- [ ] Maintenance scheduling
- [ ] Tool calling (suggested commands → optional auto-execution)
- [ ] Workflow automation
- [ ] Print simulation / prediction
- [ ] Optimization recommendations
- [ ] Predictive maintenance
- [ ] Automatic tuning
- [ ] Fleet intelligence (cross-printer learning)

---

## Phase 5 — User Interface & Deployment

Goal: LayerMind is usable by real operators.

- [ ] Tauri desktop shell
- [ ] Dashboard UI (printer overview, health, diagnostics)
- [ ] CLI tool (diagnose, status, history)
- [ ] REST API
- [ ] Fleet management UI
- [ ] Enterprise features (RBAC, audit logs)
- [ ] Opt-in cloud sync

**Foundation complete. Ready for Phase 3: Learning Engine.**
