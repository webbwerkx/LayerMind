<p align="center">
  <img src="https://img.shields.io/badge/rust-1.85+-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/license-proprietary-red?style=flat-square" alt="License">
  <img src="https://img.shields.io/github/last-commit/webbwerkx/LayerMind?style=flat-square&color=teal" alt="Last Commit">
  <img src="https://img.shields.io/badge/tests-241_passing-2ea44f?style=flat-square" alt="Tests">
  <img src="https://img.shields.io/badge/status-phase_5b-blue?style=flat-square" alt="Status">
</p>

<br>

<pre align="center">
██╗     █████╗ ██╗   ██╗███████╗██████╗  ███╗   ███╗██╗███╗   ██╗██████╗
██║     ██╔══██╗╚██╗ ██╔╝██╔════╝██╔══██╗████╗ ████║██║████╗  ██║██╔══██╗
██║     ███████║ ╚████╔╝ █████╗  ██████╔╝██╔████╔██║██║██╔██╗ ██║██║  ██║
██║     ██╔══██║  ╚██╔╝  ██╔══╝  ██╔══██╗██║╚██╔╝██║██║██║╚██╗██║██║  ██║
███████╗██║  ██║   ██║   ███████╗██║  ██║██║ ╚═╝ ██║██║██║ ╚████║██████╔╝
╚══════╝╚═╝  ╚═╝   ╚═╝   ╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝╚═════╝
</pre>

<p align="center">
  <strong>AI-powered intelligence layer for 3D printing</strong><br>
  <em>Your printer gets smarter every time it prints.</em>
</p>

<br>

<p align="center">
  <a href="#-features">Features</a> •
  <a href="#-quick-start">Quick Start</a> •
  <a href="#-architecture">Architecture</a> •
  <a href="#-terminal-ui">Terminal UI</a> •
  <a href="#-commands">Commands</a> •
  <a href="#-development">Development</a>
</p>

<br>

---

LayerMind sits above your existing 3D printer stack — Klipper, Moonraker, Mainsail, Fluidd — and turns every print into a data point. It observes hardware, tracks health over time, detects failure patterns, and runs AI diagnostics that give you actionable recommendations.

**No slicer replacement. No dashboard replacement. No chatbot.** It integrates with what you already use and makes it smarter.

<br>

---

## ✦ Features

<table>
<tr>
<td width="50%">

**📡 Real-time Monitoring**  
Full-screen terminal UI with live temperature gauges, progress bars, event timeline, and AI diagnostics panel. Polls Moonraker every 2 seconds.

**🔧 Hardware Discovery**  
Auto-detects extruders, hotends, MCUs, probes, fans, accelerometers, motion systems, and build volume from Moonraker's API.

**🧠 Capability Detection**  
Identifies what your printer can do — input shaping, pressure advance, sensorless homing, CAN bus, BLTouch, high-temperature printing — with confidence scores for each.

</td>
<td width="50%">

**📜 Timeline Engine**  
Immutable event log: print lifecycle, failures, config changes, hardware swaps, firmware updates. Queryable by printer, category, and component.

**📈 Learning Analysis**  
Detects recurring patterns like "config change followed by consecutive failures" and tracks component health degradation over time.

**🤖 AI Diagnostics**  
Plug in any provider (OpenAI, OpenRouter, Anthropic, Ollama, Gemini) for intelligent failure analysis. Prompt → parse → trust-validate → prioritize.

</td>
</tr>
<tr>
<td width="50%">

**💾 Context Engine**  
Searchable cache of printer health, history, known issues, and machine profile. Shared between the daemon pipeline and all CLI/TUI consumers.

**🛡️ Graceful Degradation**  
No PostgreSQL? Uses in-memory storage. No AI provider? Still monitors and collects. No configuration file? Environment variables only.

</td>
<td width="50%">

**⌨️ CLI Dashboard**  
Four commands: `printer test` to verify connectivity, `run` to start the daemon, `monitor` to inspect live context, `diagnose` to run AI analysis.

**🎨 Dark Industrial Theme**  
Navy/charcoal backgrounds, teal/cyan accents, clean rounded borders. Professional monitoring aesthetic — not a sci-fi HUD.

</td>
</tr>
</table>

<br>

---

## ✦ Quick Start

```shell
# ── 1. Clone & build ──────────────────────────────────────────────
git clone https://github.com/webbwerkx/LayerMind.git
cd LayerMind
cargo build --release

# ── 2. Install to PATH ────────────────────────────────────────────
cp target/release/layermind ~/.local/bin/
cp target/release/layermind-tui ~/.local/bin/

# ── 3. Point at your printer ──────────────────────────────────────
export MOONRAKER_URL=ws://voron-0.local:7125/websocket

# ── 4. Test the connection ────────────────────────────────────────
layermind printer test

# ── 5. Launch the real-time TUI ───────────────────────────────────
layermind-tui
```

> **Don't have a printer handy?** The `printer test` command requires a live Moonraker connection. Everything else — building, testing, exploring the code — works offline.

<br>

---

## ✦ Terminal UI

A full-screen real-time monitoring interface with dark-industrial styling.

```
 LAYERMIND ◆ voron-0 ◆ PRINTING ◆ 02:34:12 ◆ Layer 142/300
───────────────────────────────────────────────────────────
 STATE                  TEMPERATURES
─────────────────────  ───────────────────────────────────
 Host:     voron-0     Extruder   235°C / 240°C
 Status:   PRINTING    ▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░
 Print:    benchy      Bed        105°C / 110°C
 Progress: 47.3%       ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
 Position: X125.0      ───────────────────────────────────
           Y150.0      PROGRESS
           Z200.0      ───────────────────────────────────
 Speed:    80 mm/s     ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░
                       47.3%   Layer 142/300  ETA 00:42:15
───────────────────────────────────────────────────────────
 EVENTS                 DIAGNOSTICS
─────────────────────  ───────────────────────────────────
 ● Connected to MR      Press d to run AI diagnostic
 ● Print started:       to analyze printer health and
   benchy.gcode         get actionable recommendations
 ▲ PID deviation
 ✖ Print failed:
   thermal runaway
───────────────────────────────────────────────────────────
 q:quit  d:diagnose  m:machine info  TAB:cycle focus
```

| Key | Action | | Key | Action |
|:---:|:-------|---|:---:|:-------|
| `q` | Quit | | `Tab` | Cycle focus panel |
| `d` | Run AI diagnostic | | `↑` | Scroll events up |
| `m` | Open machine info popup | | `↓` | Scroll events down |
| `M` | Close machine info popup | | | |

> The TUI polls Moonraker every 2 seconds and never freezes — if the
> network is slow it skips frames instead of blocking the UI.

<br>

---

## ✦ Architecture

```
                    ┌──────────────────────┐
                    │   CLI / TUI / API    │
                    │  printer test        │
                    │  monitor             │
                    │  diagnose            │
                    │  layermind-tui       │
                    └──────────┬───────────┘
                               │
                    ┌──────────▼───────────┐
                    │     layermind-core    │
                    │  Pipeline orchestrator│
                    └──────────┬───────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
┌─────────▼──────┐   ┌────────▼───────┐  ┌─────────▼──────┐
│   Moonraker    │   │    Printer     │  │    Machine     │
│  WebSocket     │   │  Normalizer    │  │   Discovery    │
│  Client        │   │                │  │                │
└────────┬───────┘   └────────┬───────┘  └────────┬───────┘
         │                    │                    │
         │           ┌────────▼───────┐            │
         │           │   Telemetry    │            │
         ├──────────▶│    Engine      │◄───────────┤
         │           └────────┬───────┘            │
         │                    │                    │
         │           ┌────────▼───────┐            │
         │           │   Analyzer     │            │
         └──────────▶│    Engine      │            │
                     └────────┬───────┘            │
                              │                    │
                     ┌────────▼───────┐            │
                     │   Knowledge    │            │
                     │    Engine      │            │
                     └────────┬───────┘            │
                              │                    │
                     ┌────────▼───────┐            │
                     │    Context     │            │
                     │    Engine      │◄───────────┤
                     │                │            │
                     │  ContextStore   │            │
                     └────────┬───────┘            │
                              │                    │
                ┌─────────────┼────────────┐       │
                │             │            │       │
        ┌───────▼────┐  ┌────▼────┐ ┌────▼────┐   │
        │  Learning  │  │ History │ │    AI   │   │
        │  Analysis  │  │ Bridge  │ │Diagnose │   │
        └────────────┘  └─────────┘ └─────────┘   │
                                                   │
        ┌──────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────┐
│            Sink (Database / Memory)               │
└──────────────────────────────────────────────────┘
```

### Crate Map

| Crate | Lines | Role |
|---|---|---|
| `layermind-core` | 674 | Pipeline orchestration, lifecycle management |
| `layermind-shared` | 1,931 | Base types: events, profiles, capabilities, recommendations |
| `layermind-config` | 178 | Environment-driven configuration |
| `layermind-moonraker` | 556 | WebSocket client for Klipper's Moonraker API |
| `layermind-printer` | 364 | Printer data normalization |
| `layermind-telemetry` | 461 | Event collection, buffering, flushing |
| `layermind-analyzer` | 581 | Pattern detection rules |
| `layermind-knowledge` | 232 | Knowledge record production |
| `layermind-context` | 528 | Cached printer context engine |
| `layermind-learning` | 403 | Trend analysis and prediction |
| `layermind-history` | 677 | Immutable timeline store |
| `layermind-machine` | 1,102 | Hardware discovery, capability derivation, confidence engine |
| `layermind-reasoning` | 1,024 | AI diagnostic pipeline (prompt → parse → validate → prioritize) |
| `layermind-ai` | 647 | Provider abstractions (OpenAI, OpenRouter, Ollama, Anthropic, Gemini) |
| `layermind-database` | 213 | PostgreSQL persistence (optional, in-memory fallback) |
| `layermind-logging` | 48 | Logging configuration |

<br>

---

## ✦ Commands

```
layermind <command> [args]

Commands:
  printer test [id]    Connect to Moonraker and print full hardware report
  run                  Start the daemon pipeline (telemetry → analysis → context)
  monitor [id]         Show live printer context (requires daemon running)
  diagnose [id]        Run AI diagnostic (requires daemon running + AI provider)

Flags:
  -h, --help           Show this help
  -V, --version        Show version

Environment:
  MOONRAKER_URL                  WebSocket URL (default: ws://localhost:7125/websocket)
  LAYERMIND_MOONRAKER_API_KEY    API key (optional)
  LAYERMIND_PROVIDER             AI provider: openai, openrouter, ollama, anthropic, gemini, custom
  LAYERMIND_MODEL                Model name (e.g., deepseek/deepseek-chat, gpt-4o, llama3.3)
```

### AI Provider Setup

```shell
# OpenRouter (recommended — works with free models)
export LAYERMIND_PROVIDER=openrouter
export LAYERMIND_MODEL=deepseek/deepseek-chat

# Ollama (local, no API key needed)
export LAYERMIND_PROVIDER=ollama
export LAYERMIND_MODEL=llama3.3

# OpenAI
export LAYERMIND_PROVIDER=openai
export LAYERMIND_MODEL=gpt-4o
```

<br>

---

## ✦ Development

### Build & Test

```shell
cargo build --release          # Release build (all packages)
cargo test --workspace         # 241 tests, all passing
cargo clippy --workspace       # Zero errors
```

### Project Structure

```
apps/
├── layermind/           #    558 lines — CLI binary
└── layermind-tui/       #    677 lines — TUI binary
crates/
├── core/                #    674 lines — Pipeline orchestration
├── shared/              #  1,931 lines — Base types
├── config/              #    178 lines — Configuration
├── moonraker/           #    556 lines — WebSocket client
├── printer/             #    364 lines — Normalization
├── telemetry/           #    461 lines — Event collection
├── analyzer/            #    581 lines — Pattern detection
├── knowledge/           #    232 lines — Knowledge records
├── context/             #    528 lines — Context engine
├── learning/            #    403 lines — Trend analysis
├── history/             #    677 lines — Timeline store
├── machine/             #  1,102 lines — Hardware discovery
├── reasoning/           #  1,024 lines — AI diagnostics
├── ai/                  #    647 lines — AI providers
├── database/            #    213 lines — PostgreSQL
└── logging/             #     48 lines — Logging config
```

### Tech Stack

| Technology | Purpose |
|---|---|
| **Rust** (edition 2024) | Systems programming language |
| **Tokio** | Async runtime |
| **Ratatui 0.29** + **Crossterm 0.28** | Terminal UI framework |
| **SQLx** + **PostgreSQL** | Optional persistence |
| **tokio-tungstenite** | WebSocket client |
| **Serde** + **Serde JSON** | Serialization |
| **Tracing** + **tracing-subscriber** | Structured logging |
| **OpenAI / OpenRouter / Anthropic / Ollama / Gemini** | AI providers |

<br>

---

## ✦ Roadmap

| Phase | Status | Description |
|:------|:------:|:------------|
| 1 — Foundation | ✅ | Telemetry pipeline, Moonraker integration, configuration |
| 2 — Intelligence | ✅ | AI diagnostics, trust validation, prompt building, prioritization |
| 3 — Memory | ✅ | Timeline history, component health, failure prediction, diffs |
| 4 — Optimization | ✅ | Learning analysis, pattern detection, hardware library, confidence |
| 5a — CLI Polish | ✅ | --help/--version, error messages, real Moonraker printer test |
| 5b — TUI | ✅ | Real-time monitoring, diagnostics, machine info, responsive layout |
| 5c — Desktop | ⏳ | Tauri desktop app with React frontend |
| 6 — Fleet | 📋 | Multi-printer management, dashboards, alerts |

<br>

---

<p align="center">
  <a href="https://github.com/webbwerkx/LayerMind/issues">Report a bug</a> •
  <a href="https://github.com/webbwerkx/LayerMind/discussions">Feature request</a> •
  <a href="USAGE.md">Full usage guide</a>
</p>

<p align="center">
  <sub>Built with Rust · Licensed under proprietary terms</sub>
</p>
