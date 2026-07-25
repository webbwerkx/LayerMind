# LayerMind — Complete Setup & Usage Guide

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Building](#2-building)
3. [Configuration](#3-configuration)
4. [Quick Start — Test Connection](#4-quick-start--test-connection)
5. [CLI Commands](#5-cli-commands)
6. [Terminal UI (TUI)](#6-terminal-ui-tui)
7. [Daemon Mode](#7-daemon-mode)
8. [AI Diagnostics](#8-ai-diagnostics)
9. [Troubleshooting](#9-troubleshooting)

---

## 1. Prerequisites

### Hardware & Software

- **3D printer** running **Klipper** with **Moonraker** (any Klipper-based printer — Voron, Creality, Prusa, etc.)
- **Moonraker WebSocket API** enabled (default: port 7125)
- **Rust toolchain** 1.85+ (edition 2024)

### Rust Installation

```fish
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify
rustc --version   # should be 1.85 or later
cargo --version
```

### Clone & Build

```fish
git clone https://github.com/layermind/layermind ~/dev/LayerMind
cd ~/dev/LayerMind

# Build everything (this takes a few minutes the first time)
cargo build --release
```

---

## 2. Building

### Quick Build (all packages)

```fish
cd ~/dev/LayerMind
cargo build --release
```

The binaries land at:

| Binary | Path |
|---|---|
| `layermind` (CLI) | `target/release/layermind` |
| `layermind-tui` (TUI) | `target/release/layermind-tui` |

### Install to PATH

```fish
# Add to ~/.local/bin for easy access
cp target/release/layermind ~/.local/bin/
cp target/release/layermind-tui ~/.local/bin/

# Verify
layermind --version
layermind-tui --version   # (TUI doesn't have a --version flag yet)
```

### Build Specific Packages

```fish
# CLI only
cargo build --release -p layermind

# TUI only
cargo build --release -p layermind-tui

# Daemon core only
cargo build --release -p layermind-core
```

### Run Tests

```fish
# All 241 tests
cargo test --workspace

# A specific crate
cargo test -p layermind-machine
cargo test -p layermind-history
cargo test -p layermind-core
```

---

## 3. Configuration

All configuration is **environment-variable driven**. No config file is needed to get started, though one can be added.

### Essential Variables

| Env Var | Required | Default | Example |
|---|---|---|---|
| `MOONRAKER_URL` | Yes | `ws://localhost:7125/websocket` | `ws://voron-0.local:7125/websocket` |
| `LAYERMIND_MOONRAKER_API_KEY` | No | — | `your-api-key` |
| `LAYERMIND_PROVIDER` | For AI | `custom` | `openrouter`, `openai`, `ollama` |
| `LAYERMIND_MODEL` | For AI | `gpt-4o` | `deepseek/deepseek-chat`, `gpt-4o` |

### Setting Variables

**Fish shell** (the user's shell):

```fish
# Set for the current session
set -x MOONRAKER_URL ws://voron-0.local:7125/websocket

# Make permanent
echo 'set -x MOONRAKER_URL ws://voron-0.local:7125/websocket' >> ~/.config/fish/config.fish
```

**Bash/Zsh**:

```bash
export MOONRAKER_URL=ws://voron-0.local:7125/websocket
```

### Sample Configurations

**Minimal (test connection only):**

```fish
set -x MOONRAKER_URL ws://voron-0.local:7125/websocket
```

**With AI diagnostics (OpenRouter):**

```fish
set -x MOONRAKER_URL ws://voron-0.local:7125/websocket
set -x LAYERMIND_PROVIDER openrouter
set -x LAYERMIND_MODEL deepseek/deepseek-chat
```

**With AI diagnostics (Ollama — local):**

```fish
set -x MOONRAKER_URL ws://voron-0.local:7125/websocket
set -x LAYERMIND_PROVIDER ollama
set -x LAYERMIND_MODEL llama3.3
```

**With AI diagnostics (OpenAI):**

```fish
set -x MOONRAKER_URL ws://voron-0.local:7125/websocket
set -x LAYERMIND_PROVIDER openai
set -x LAYERMIND_MODEL gpt-4o
```

### Optional Variables

| Env Var | Default | Purpose |
|---|---|---|
| `LAYERMIND_LOG_LEVEL` | `info` | Log verbosity: `trace`, `debug`, `info`, `warn`, `error` |
| `LAYERMIND_DATABASE_URL` | `postgres://localhost:5432/layermind` | PostgreSQL connection (falls back to in-memory if unavailable) |
| `LAYERMIND_PROVIDER_ENDPOINT` | Provider default | Custom API endpoint URL |

---

## 4. Quick Start — Test Connection

This is the fastest way to verify LayerMind can talk to your printer.

### Step 1: Set your Moonraker URL

```fish
set -x MOONRAKER_URL ws://your-printer.local:7125/websocket
```

Replace `your-printer.local` with your printer's actual hostname or IP address. Common values:

- `ws://voron-0.local:7125/websocket`
- `ws://192.168.1.100:7125/websocket`
- `ws://klipperpi.local:7125/websocket`

### Step 2: Run the printer test

```fish
layermind printer test
```

### What You'll See

```
Moonraker Connection

  URL: ws://voron-0.local:7125/websocket
  Printer: default

Machine:
  Motion: Cartesian
  Hostname: voron-0
  Klipper: v0.12.0-123-gabc12345
  Moonraker: v0.8.0-123-gdef67890
  MCUs: 2
  Manufacturer: LDO [Moonraker]
  Model: Voron 2.4 [Moonraker]

Hardware:
  Control board: Octopus v1.1
    Manufacturer: Makerbase
    MCU: rp2040
  Extruders: 1
    - Extruder (DirectDrive)
  Hotends: 1
  Heated bed: AC
  MCUs: 2
    - mcu (primary: true)
    - rpi_mcu (primary: false)
  Fans: 3
  Probes: 2
  Axes: 3
  Build volume: 350 × 350 × 350 mm

Capabilities:
  Input shaping: supported
  Pressure advance: supported
  Sensorless homing: not detected
  CAN bus: not detected
  BLTouch/CRTouch: supported
  Beacon probe: not detected
  High temperature: supported
  Filament sensor: supported
  Enclosure: not detected

Generated: 2026-07-24T15:30:00.123456Z
```

### If It Fails

```
Error: Failed to connect to Moonraker: ...
  Check that MOONRAKER_URL is correct and the printer is running.
```

Common reasons:
- Wrong URL/port — verify Moonraker is actually on port 7125
- Printer not reachable — `ping your-printer.local` to check
- WebSocket path wrong — some setups use `/websocket` but others may differ
- Firewall blocking port 7125

---

## 5. CLI Commands

### `layermind printer test [id]`

Tests the Moonraker connection and prints a full hardware report.

```fish
# With default printer ID
layermind printer test

# With explicit printer ID
layermind printer test voron-0
```

**What it does:**
1. Connects to Moonraker via WebSocket
2. Queries `printer.info`, `server.info`, and `printer.objects.query`
3. Runs the full hardware discovery pipeline
4. Prints identity, hardware components, capabilities, and confidence scores

**Good for:**
- Verifying Moonraker is reachable
- Seeing what hardware LayerMind detects
- Debugging missing hardware detection

### `layermind run`

Starts the full LayerMind daemon — the pipeline that collects telemetry, builds context, and runs learning analysis.

```fish
layermind run
```

**What it does:**
1. Loads configuration
2. Initializes logging
3. Connects to Moonraker
4. Starts the pipeline: Moonraker → Printer → Telemetry → Analyzer → Knowledge → ContextStore
5. Discovers machine hardware (3s after startup)
6. Runs learning analysis every 60 seconds
7. Runs the history bridge (telemetry → timeline store)
8. Waits for Ctrl+C to shut down

**Sample output:**
```
2026-07-24T15:30:00.123456Z  INFO  layermind_core: LayerMind starting
2026-07-24T15:30:00.234567Z  INFO  layermind_core: telemetry engine ready
2026-07-24T15:30:00.345678Z  INFO  layermind_core: printer instance created
2026-07-24T15:30:00.456789Z  INFO  layermind_core: Moonraker client ready
2026-07-24T15:30:00.567890Z  INFO  layermind_core: analyzer engine started
2026-07-24T15:30:00.678901Z  INFO  layermind_core: knowledge engine started
2026-07-24T15:30:00.789012Z  INFO  layermind_core: context engine started
2026-07-24T15:30:00.890123Z  INFO  layermind_core: learning analysis loop started
2026-07-24T15:30:00.901234Z  INFO  layermind_core: history bridge started
2026-07-24T15:30:00.912345Z  INFO  layermind_core: all services started — pipeline active
2026-07-24T15:30:00.912345Z  INFO  layermind_core: Moonraker → Printer → Telemetry → (sink)
2026-07-24T15:30:03.000000Z  INFO  layermind_core: machine profile discovered and stored
```

Press **Ctrl+C** to stop cleanly.

### `layermind monitor [id]`

Shows the current printer context from the daemon's ContextStore.

```fish
# Requires layermind run to be running first
layermind monitor
```

**What it shows:**
- Runtime started time
- AI provider and model
- Printer name, model, firmware
- Current status (printing/idle)
- Active print filename (if printing)
- Health metrics (temperature stability, success rate)
- Machine intelligence data (motion type, extruders, hotends, probes)
- Recent history (hardware changes, firmware updates, config changes)

**Sample output:**
```
LayerMind Runtime Status

  Runtime started: 2026-07-24T15:30:00.123456Z
  AI provider: OpenRouter (deepseek/deepseek-chat)

Printer: default

  Name: default
  Model: Voron 2.4
  Firmware: Klipper v0.12.0

  Status: IDLE

  Health:
    Temperature stability: 0.98
    Success rate: 0.92
    Total prints: 102
    Failed prints: 8

  Machine Intelligence:
    Motion: Cartesian
    Extruders: 1
    Hotends: 1
    Probes: BLTouch

  Recent History:
    Last hardware change: 2026-07-20T12:00:00Z
    Last firmware update: 2026-07-15T08:30:00Z
    Last config change: 2026-07-22T14:00:00Z
    Last calibration: 2026-07-23T18:00:00Z
```

### `layermind diagnose [id]`

Runs an AI diagnostic on the printer.

```fish
# Requires layermind run to be running first (for context data)
layermind diagnose
```

**What it does:**
1. Fetches the printer's context from the ContextStore
2. Builds a structured prompt with printer history, issues, and patterns
3. Sends to the configured AI provider
4. Parses and validates the AI response
5. Outputs actions, confidence, and usage statistics

**Sample output:**
```
AI Diagnostic

  Printer: default
  Status: IDLE
  Using: OpenRouter (deepseek/deepseek-chat)

  Summary: PID tuning recommended to improve temperature stability
  Confidence: 0.85

  Actions:
    1. Run PID calibration on extruder
       Priority: 1
       Command: PID_CALIBRATE HEATER=extruder TARGET=210
       Expected: Temperature deviation reduced to <1°C
    2. Verify silicone sock is properly seated on heater block
       Priority: 2
       Expected: Reduced thermal fluctuation from drafts

  Tokens: 450 prompt + 120 completion | Cost: $0.000450
  Provider: openrouter / deepseek/deepseek-chat
```

---

## 6. Terminal UI (TUI)

The TUI is a full-screen real-time monitoring interface.

### Launch

```fish
# With default Moonraker URL (from env)
layermind-tui

# Or set the URL inline
env MOONRAKER_URL=ws://voron-0.local:7125/websocket layermind-tui
```

### Layout

```
┌─ LAYERMIND ◆ voron-0 ◆ IDLE ─────────────────── 01:23:45  42/84 ─┐
│ ┌─ STATE ────────────┐ ┌─ TEMPERATURES ───────────┐              │
│ │  Host:     voron-0  │ │  Extruder   220°C / 240°C │              │
│ │  Status:   IDLE     │ │  ████████░░░░░░░░░░░░░░░  │              │
│ │  Print:    benchy   │ │  Bed        60°C /  60°C  │              │
│ │  Progress: 45.2%    │ │  ████████████████████████  │              │
│ │  Position: X125.0   │ └──────────────────────────┘                │
│ │           Y150.0    │ ┌─ PROGRESS ─────────────────┐              │
│ │           Z200.0    │ │  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░  │              │
│ │  Speed:    80 mm/s  │ │  45.2%   Layer 42/84      │              │
│ └─────────────────────┘ └──────────────────────────┘                │
│ ┌─ EVENTS ───────────────────┐ ┌─ DIAGNOSTICS ───────┐              │
│ │  ◆ Connected to Moonraker  │ │  Press d to run AI  │              │
│ │  ⚠ PID deviation detected  │ │  diagnostic         │              │
│ │  ✗ Print failed: thermal   │ └─────────────────────┘              │
│ └────────────────────────────┘                                      │
├─ q:quit │ d:diagnose │ m:machine │ TAB:focus ───────────────────────┤
└─────────────────────────────────────────────────────────────────────┘
```

### Keyboard Controls

| Key | Action |
|---|---|
| `q` | Quit |
| `d` | Run AI diagnostic (spawns async, shows result in diagnostics panel) |
| `m` | Open machine info popup (shows hardware profile) |
| `M` | Close machine info popup |
| `Tab` | Cycle focus between panels (Printer → Temps → Events → Recs) |
| `↑` | Scroll events panel up (when focused) |
| `↓` | Scroll events panel down (when focused) |

### What Each Panel Shows

**STATE panel:**
- Printer hostname, status, active print filename
- Print progress percentage
- Toolhead position (X, Y, Z)
- Current speed

**TEMPERATURES panel:**
- Extruder temp/actual and target
- Bed temp/actual and target
- Visual gauge bars

**PROGRESS panel:**
- Print progress bar
- Current layer / total layers
- ETA (estimated time remaining)

**EVENTS panel:**
- Connection events
- Print lifecycle events (started, completed, failed)
- Warnings and errors from Moonraker
- Auto-scrolls to newest events

**DIAGNOSTICS panel:**
- Idle: "Press `d` to run AI diagnostic"
- Running: "Running AI diagnostic..."
- Complete: Shows recommendation summary, confidence, and actions
- Error: Shows error message

### Machine Info Popup

Press `m` to open a popup showing the full machine profile:
- Hostname, machine type
- Hardware count (extruders, hotends, MCUs, fans, probes)
- Motion system details (axes, build volume)
- Capabilities (input shaping, pressure advance, etc.)

Press `M` to close.

### What Happens When Moonraker Disconnects

- The TUI continues running
- Events panel shows "Disconnected: ..." in red
- The polling loop tries to reconnect every 2 seconds
- When reconnected, shows "Connected to Moonraker" in green
- The UI never freezes (uses `try_lock` for rendering)

---

## 7. Daemon Mode

The daemon (`layermind run`) runs the full pipeline. This is what you need running for `monitor` and `diagnose` commands to work.

### Pipeline Architecture

```
Moonraker (WebSocket)
    │
    ▼
Printer (normalization)
    │
    ├──▶ Telemetry → Sink (database or memory)
    │
    ├──▶ History Bridge → TimelineStore → ContextStore
    │
    └──▶ Analyzer → Knowledge Engine → ContextEngine → ContextStore
    │
    └──▶ Learning Engine (every 60s) → ContextStore.set_learning()
    │
    └──▶ Machine Profile (one-shot at startup) → ContextStore.set_machine()
```

### What Happens at Startup

1. **Config loads** from environment variables
2. **Logging initializes** with the configured level
3. **Database connects** (or falls back to in-memory)
4. **Telemetry engine** starts to receive printer events
5. **Printer instance** is created (normalizes Moonraker data)
6. **Moonraker client** connects to your printer
7. **Analyzer engine** starts detecting patterns
8. **Knowledge engine** converts observations into knowledge records
9. **Context engine** maintains the cached printer context
10. **Learning engine** runs periodic analysis (every 60s)
11. **History bridge** maps significant events to the timeline store
12. **Machine profile discovery** queries hardware info (3s delayed)
13. Pipeline waits for Ctrl+C

### Graceful Shutdown

Press **Ctrl+C** to shut down. The daemon:
1. Sends shutdown signal to all services
2. Waits up to 10 seconds for clean shutdown
3. Logs "LayerMind shut down cleanly"

### Running in Background

```fish
# Start in background
layermind run &

# Use the CLI while it runs
layermind monitor
layermind diagnose

# Bring to foreground later
fg

# Or kill it
kill %1
```

### Running with Systemd (optional)

```ini
# ~/.config/systemd/user/layermind.service
[Unit]
Description=LayerMind AI Printer Intelligence
After=network.target

[Service]
Type=simple
ExecStart=%h/.local/bin/layermind run
Environment=MOONRAKER_URL=ws://voron-0.local:7125/websocket
Environment=LAYERMIND_PROVIDER=openrouter
Environment=LAYERMIND_MODEL=deepseek/deepseek-chat
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
```

Then:

```fish
systemctl --user daemon-reload
systemctl --user enable --now layermind
systemctl --user status layermind
```

---

## 8. AI Diagnostics

### How It Works

1. **Context is gathered** from the daemon's ContextStore (needs `layermind run` running)
2. **PromptBuilder** creates a structured prompt with:
   - Printer summary (name, model, firmware, uptime)
   - Print history (success rate, failure patterns)
   - Health metrics (temperature stability, error count)
   - Known issues (with occurrence counts, trends)
   - Historical patterns (recurring issues)
3. **AI provider** generates a response with:
   - Category (thermal, mechanical, firmware, etc.)
   - Severity (info, warning, critical)
   - Confidence score
   - Summary and explanation
   - Prioritized actions with suggested commands
4. **ResponseParser** extracts structured data from the AI response
5. **TrustValidator** cross-references AI claims against known context data
6. **Prioritizer** sorts actions by health impact and relevance

### Provider Configuration

**OpenRouter** (recommended — no API key needed with free models):

```fish
set -x LAYERMIND_PROVIDER openrouter
set -x LAYERMIND_MODEL deepseek/deepseek-chat
```

**OpenAI**:

```fish
set -x LAYERMIND_PROVIDER openai
set -x LAYERMIND_MODEL gpt-4o
```

**Ollama** (local):

```fish
set -x LAYERMIND_PROVIDER ollama
set -x LAYERMIND_MODEL llama3.3
```

**Anthropic**:

```fish
set -x LAYERMIND_PROVIDER anthropic
set -x LAYERMIND_MODEL claude-sonnet-4-20250514
```

**Custom** (any OpenAI-compatible endpoint):

```fish
set -x LAYERMIND_PROVIDER custom
set -x LAYERMIND_MODEL my-model
set -x LAYERMIND_PROVIDER_ENDPOINT https://my-custom-api.com/v1
```

### Running a Diagnostic

1. Start the daemon: `layermind run`
2. Let it collect data for a few minutes (or start a print)
3. In another terminal: `layermind diagnose`
4. Read the results

### Diagnostic Strategies

The system supports three strategies:
- **RAPID** — Fast analysis, single pass (default for CLI)
- **STANDARD** — Balanced analysis (default for orchestrator)
- **THOROUGH** — Multi-pass analysis with verification

---

## 9. Troubleshooting

### "Failed to connect to Moonraker"

**Check:**
```fish
# Is the printer reachable?
ping your-printer.local

# Is Moonraker running?
curl http://your-printer.local:7125/printer/info

# Is the WebSocket path correct?
# Try: ws://your-printer.local:7125/websocket
# Some setups use: ws://your-printer.local:7125/ws
```

### "No context available" for monitor/diagnose

The daemon (`layermind run`) must be running in the background. The `monitor` and `diagnose` commands read from the daemon's ContextStore.

### TUI shows "Disconnected" repeatedly

Check your Moonraker URL and network connectivity. The TUI retries every 2 seconds automatically.

### "Database unavailable, using in-memory sink"

This is fine! PostgreSQL is optional. The system falls back to in-memory storage gracefully.

### No hardware detected in `printer test`

Some Moonraker versions expose different data structures. The `discover_hardware()` function parses available data — if your printer has unusual hardware, it may show as "not detected" rather than wrong.

### AI diagnostic returns low confidence

The AI model needs sufficient context data. If the daemon has been running for less than a minute, there may not be enough data. Let it run through a print cycle.

### "Terminal too small" in TUI

Resize your terminal to at least 80×24 characters. The TUI requires this minimum.

### How to get verbose logging

```fish
set -x LAYERMIND_LOG_LEVEL debug
layermind run
```

### Full End-to-End Test

```fish
# 1. Set your printer URL
set -x MOONRAKER_URL ws://voron-0.local:7125/websocket

# 2. Test connection
layermind printer test

# 3. Start the daemon (in background or another terminal)
layermind run &

# 4. Wait 10 seconds for data collection
sleep 10

# 5. Monitor the printer
layermind monitor

# 6. Run AI diagnostic (if provider configured)
layermind diagnose

# 7. Launch the TUI
layermind-tui

# 8. In the TUI, press 'd' to run diagnostic, 'm' to see machine info
# 9. Press 'q' to quit

# 10. Stop the daemon
kill %1
```

---

## CLI Reference (Quick)

```
layermind <command> [args]

Commands:
  printer test [id]    Test Moonraker connection and show hardware
  monitor [id]         Show live printer context (daemon must be running)
  diagnose [id]        Run AI diagnostic (daemon must be running)
  run                  Start the full daemon pipeline

Flags:
  -h, --help           Show help
  -V, --version        Show version

Environment:
  MOONRAKER_URL                  Moonraker WebSocket URL
  LAYERMIND_MOONRAKER_API_KEY    Moonraker API key (optional)
  LAYERMIND_PROVIDER             AI provider (openai, openrouter, ollama, etc.)
  LAYERMIND_MODEL                Model name
  LAYERMIND_LOG_LEVEL            Log verbosity (trace, debug, info, warn, error)
```

---

## File Locations

```
~/dev/LayerMind/
├── apps/
│   ├── layermind/           # CLI application
│   │   └── src/
│   │       ├── main.rs          # CLI entry point + argument parsing
│   │       ├── commands.rs      # Command implementations
│   │       ├── application.rs   # Runtime bootstrap
│   │       └── runtime.rs       # Runtime struct definition
│   └── layermind-tui/       # TUI application
│       └── src/
│           ├── main.rs          # TUI entry point + event loop
│           ├── app.rs           # AppState and PrinterSnapshot
│           ├── client.rs        # Moonraker polling
│           ├── commands.rs      # Diagnose and machine info commands
│           ├── layout.rs        # Terminal layout and rendering
│           └── theme.rs         # Dark Industrial color theme
├── crates/
│   ├── core/                # Pipeline orchestration
│   ├── shared/              # Base types (events, profiles, capabilities)
│   ├── config/              # Configuration loading
│   ├── moonraker/           # Moonraker WebSocket client
│   ├── printer/             # Printer normalization layer
│   ├── telemetry/           # Telemetry collection and processing
│   ├── analyzer/            # Pattern detection rules
│   ├── knowledge/           # Knowledge record engine
│   ├── context/             # ContextStore and ContextEngine
│   ├── learning/            # Learning analysis and prediction
│   ├── history/             # Timeline store and bridge
│   ├── machine/             # Hardware discovery and capability engine
│   ├── reasoning/           # AI diagnostic pipeline
│   ├── ai/                  # AI provider abstractions
│   ├── database/            # PostgreSQL persistence
│   └── logging/             # Logging configuration
└── target/release/          # Built binaries
```