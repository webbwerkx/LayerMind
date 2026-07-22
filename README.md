# LayerMind

**AI-powered operating system for additive manufacturing.**

LayerMind is an intelligence layer that sits above your existing 3D printer tools. It observes, learns, and recommends — turning every print into a data point that makes your next print better.

> A printer should become smarter every time it prints.

## What LayerMind Is

- An **intelligence layer** above Klipper, Moonraker, OrcaSlicer, and OctoPrint
- A **telemetry engine** that collects and preserves every printer event
- An **AI analysis system** that detects patterns and recommends improvements
- A **printer health monitor** that tracks degradation over time

## What LayerMind Is Not

- A slicer
- A Klipper dashboard
- A chatbot
- An OctoPrint replacement

LayerMind integrates with your existing tools. It does not replace them.

## Architecture

```
                LayerMind Desktop (Tauri + React)
                          |
                   LayerMind Core
                          |
        ----------------------------------
        |                |               |
   Printer Layer     AI Engine     Knowledge Engine
        |
    Integrations (Moonraker, Klipper, ...)
```

## Current Status

**Phase 1: Foundation & Observation** — building the telemetry pipeline and Moonraker integration.

## Development

### Prerequisites

- Rust 1.80+
- PostgreSQL 16+ (optional, for persistence)

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Run

```bash
cargo run -p layermind-core
```

## License

Proprietary. All rights reserved.
