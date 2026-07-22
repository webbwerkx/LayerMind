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

### Milestone 1.2 — Moonraker Integration
- [ ] WebSocket connection with authentication
- [ ] Subscription to printer objects
- [ ] Automatic reconnection with exponential backoff
- [ ] Raw message normalization
- [ ] Integration tests with mock Moonraker server

### Milestone 1.3 — Telemetry Pipeline
- [ ] Event buffering and batching
- [ ] Database schema and migrations
- [ ] Telemetry event storage
- [ ] Print job lifecycle tracking
- [ ] Event replay capability

### Milestone 1.4 — Basic Dashboard
- [ ] CLI tool for printer status
- [ ] Live temperature display
- [ ] Print progress monitoring
- [ ] Event history viewer

---

## Phase 2: Analysis & Intelligence

**Goal: LayerMind detects problems and provides recommendations.**

### Milestone 2.1 — Event Detection
- [ ] Temperature instability detection
- [ ] Print failure classification
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
