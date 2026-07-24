# LayerMind

**AI-powered operating system for additive manufacturing.**

[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![CI](https://img.shields.io/github/actions/workflow/status/webbwerkx/LayerMind/ci.yml?branch=main)](https://github.com/webbwerkx/LayerMind/actions)
[![License](https://img.shields.io/badge/license-proprietary-red.svg)](LICENSE)

LayerMind is an intelligence layer that sits above your 3D printer stack — Klipper, Moonraker, Mainsail/Fluidd. It observes every print, learns from every failure, and recommends actionable improvements. Your printer gets smarter every time it prints.

---

## Features

- **Real-time monitor** — Terminal UI showing temperatures, progress, events, and diagnostics in a dark-industrial theme
- **Hardware discovery** — Auto-detects extruders, hotends, MCUs, probes, fans, accelerometers, and motion systems from Moonraker
- **Capability detection** — Identifies what your printer can do (input shaping, pressure advance, BLTouch, CAN bus, etc.)
- **Timeline engine** — Records every significant event: print lifecycle, failures, config changes, hardware swaps
- **Learning analysis** — Detects recurring patterns like "config change followed by failures" and tracks component health trends
- **AI diagnostics** — Plug in any provider (OpenAI, OpenRouter, Ollama, Anthropic) for intelligent print failure analysis with actionable recommendations
- **Context engine** — Maintains a searchable cache of printer health, history, known issues, and machine profile
- **CLI dashboard** — `printer test`, `monitor`, `diagnose` commands for quick inspection
- **Graceful degradation** — No database? Uses in-memory storage. No AI provider? Still monitors and collects.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                         LAYERMIND CLI/TUI                        │
│              printer test · monitor · diagnose · run             │
└──────────────────────────┬───────────────────────────────────────┘
                           │
┌──────────────────────────▼───────────────────────────────────────┐
│                       layermind-core                             │
│                Pipeline orchestration & lifecycle                │
└──────────────────────────┬───────────────────────────────────────┘
                           │
           ┌───────────────┼───────────────────┐
           │               │                   │
┌──────────▼──────┐ ┌─────▼──────┐ ┌──────────▼──────┐
│   Moonraker     │ │  Printer   │ │    Machine      │
│  WebSocket      │ │Normalizer  │ │   Discovery     │
│  Client         │ │            │ │                 │
└──────┬──────────┘ └─────┬──────┘ └──────────┬──────┘
       │                  │                   │
       │           ┌──────▼──────┐            │
       │           │  Telemetry  │            │
       ├──────────▶│   Engine    │◄───────────┤
       │           └──────┬──────┘            │
       │                  │                   │
       │           ┌──────▼──────┐            │
       │           │  Analyzer   │            │
       └──────────▶│   Engine    │            │
                   └──────┬──────┘            │
                          │                   │
                   ┌──────▼──────┐            │
                   │  Knowledge  │            │
                   │   Engine    │            │
                   └──────┬──────┘            │
                          │                   │
                   ┌──────▼──────┐            │
                   │   Context   │            │
                   │   Engine    │◄───────────┤
                   │             │            │
                   │ ContextStore│            │
                   └──────┬──────┘            │
                          │                   │
              ┌───────────┼───────────┐       │
              │           │           │       │
      ┌───────▼────┐ ┌────▼───┐ ┌────▼───┐   │
      │  Learning  │ │History │ │  AI    │   │
      │  Analysis  │ │ Bridge │ │Diagnose│   │
      └────────────┘ └────────┘ └────────┘   │
                                              │
      ┌───────────────────────────────────────┘
      │
┌─────▼────────────────────────────────────────────────────────────┐
│                       Sink (Database / Memory)                   │
└──────────────────────────────────────────────────────────────────┘
```

### Crate Map

| Crate | Role |
|---|---|
| `layermind-core` | Pipeline orchestration, lifecycle management |
| `layermind-shared` | Base types: events, profiles, capabilities, recommendations |
| `layermind-config` | Environment-driven configuration |
| `layermind-moonraker` | WebSocket client for Klipper's Moonraker API |
| `layermind-printer` | Printer data normalization |
| `layermind-telemetry` | Event collection, buffering, flushing |
| `layermind-analyzer` | Pattern detection rules |
| `layermind-knowledge` | Knowledge record production |
| `layermind-context` | Cached printer context engine |
| `layermind-learning` | Trend analysis and prediction |
| `layermind-history` | Immutable timeline store |
| `layermind-machine` | Hardware discovery, capability derivation, confidence engine |
| `layermind-reasoning` | AI diagnostic pipeline (prompt → parse → validate → prioritize) |
| `layermind-ai` | Provider abstractions (OpenAI, OpenRouter, Ollama, Anthropic) |
| `layermind-database` | PostgreSQL persistence (optional, in-memory fallback) |

---

## Quick Start

### Prerequisites

- Rust 1.85+ (`rustup update`)
- A Klipper-based 3D printer with Moonraker running

### Setup

```fish
# Clone
git clone https://github.com/webbwerkx/LayerMind.git
cd LayerMind

# Build
cargo build --release

# Install to PATH
cp target/release/layermind ~/.local/bin/
cp target/release/layermind-tui ~/.local/bin/

# Point at your printer
set -x MOONRAKER_URL ws://your-printer.local:7125/websocket

# Test connection
layermind printer test
```

### Commands

| Command | What it does |
|---|---|
| `layermind printer test` | Connect to Moonraker and print full hardware report |
| `layermind run` | Start the daemon pipeline (telemetry → analysis → context) |
| `layermind monitor` | Show live printer context (requires daemon) |
| `layermind diagnose` | Run AI diagnostic (requires daemon + AI provider configured) |
| `layermind-tui` | Full-screen terminal UI with real-time monitoring |

### AI Provider Setup

```fish
set -x LAYERMIND_PROVIDER openrouter
set -x LAYERMIND_MODEL deepseek/deepseek-chat
layermind run
layermind diagnose
```

See [`USAGE.md`](USAGE.md) for complete setup instructions, provider configs, troubleshooting, and TUI keyboard controls.

---

## Terminal UI

```
┌─ LAYERMIND ◆ voron-0 ◆ IDLE ──────── 01:23:45  42/84 ─┐
│ ┌─ STATE ────────────┐ ┌─ TEMPERATURES ───────────┐   │
│ │  Host:     voron-0  │ │  Extruder 220°C / 240°C  │   │
│ │  Status:   IDLE     │ │  ████████░░░░░░░░░░░░░░░  │   │
│ │  Print:    benchy   │ │  Bed       60°C /  60°C   │   │
│ │  Progress: 45.2%    │ │  ████████████████████████  │   │
│ │  Position: X125.0   │ └──────────────────────────┘   │
│ │           Y150.0    │ ┌─ PROGRESS ─────────────────┐ │
│ │           Z200.0    │ │  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░ │ │
│ │  Speed:    80 mm/s  │ │  45.2%   Layer 42/84       │ │
│ └─────────────────────┘ └──────────────────────────┘   │
│ ┌─ EVENTS ───────────────┐ ┌─ DIAGNOSTICS ───────────┐│
│ │  ◆ Connected to MR     │ │  Press d for diagnostic ││
│ │  ⚠ PID deviation       │ └─────────────────────────┘│
│ └────────────────────────┘                             │
├─ q:quit │ d:diagnose │ m:machine │ TAB:focus ─────────┤
└───────────────────────────────────────────────────────┘
```

Dark-industrial Ratatui TUI. Real-time Moonraker polling every 2 seconds. Hotkeys for diagnostics, machine info popup, and panel focus cycling.

---

## Development

### Build

```fish
cargo build --release
```

### Test

```fish
cargo test --workspace  # 240+ tests, all passing
```

### Project Structure

```
apps/
├── layermind/           # CLI binary
└── layermind-tui/       # TUI binary
crates/
├── core/                # Pipeline orchestration
├── shared/              # Base types
├── config/              # Configuration
├── moonraker/           # WebSocket client
├── printer/             # Normalization
├── telemetry/           # Event collection
├── analyzer/            # Pattern detection
├── knowledge/           # Knowledge records
├── context/             # Context engine
├── learning/            # Trend analysis
├── history/             # Timeline store
├── machine/             # Hardware discovery
├── reasoning/           # AI diagnostics
├── ai/                  # AI providers
├── database/            # PostgreSQL
└── logging/             # Logging config
```

### Tech Stack

- **Language:** Rust (edition 2024)
- **Async runtime:** Tokio
- **TUI:** Ratatui 0.29 + Crossterm 0.28
- **Database:** SQLx + PostgreSQL (optional)
- **AI providers:** OpenAI, OpenRouter, Anthropic, Ollama, Gemini, custom
- **Serialization:** Serde + Serde JSON
- **WebSocket:** tokio-tungstenite
- **Logging:** Tracing + tracing-subscriber

---

## Status

| Phase | Status | Description |
|---|---|---|
| 1 | ✅ | Foundation: telemetry pipeline, Moonraker integration, config |
| 2 | ✅ | Intelligence: AI diagnostics, trust validation, prioritization |
| 3 | ✅ | Memory: timeline history, component health, failure prediction |
| 4 | ✅ | Optimization: learning analysis, pattern detection, hardware library |
| 5a | ✅ | CLI polish: --help, --version, error messages, printer test with real data |
| 5b | ✅ | TUI: real-time monitoring, diagnostics, machine info, responsive layout |
| 5c | ⏳ | Desktop app (Tauri) |
| 6 | 📋 | Fleet management, multi-printer dashboards |

---

## License

Proprietary. All rights reserved.
