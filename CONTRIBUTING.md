# Contributing to LayerMind

## Philosophy

LayerMind is built for the long term. Every decision should prioritize:

1. **Data quality** — incorrect data is worse than no data
2. **Reliability** — the system must not crash or lose events
3. **Extensibility** — future integrations should not require rewrites
4. **Privacy** — user data stays local by default

## Development Principles

### Before Writing Code
1. Explain the goal
2. Explain the architecture
3. List files affected
4. Get alignment
5. Then implement

### Code Standards
- No unnecessary abstractions
- No over-engineering
- Keep modules independent
- Write tests
- Document important decisions
- Prefer boring, reliable solutions

### Commit Standards
- Logical, atomic commits
- Clear commit messages explaining WHY
- Reference issues/milestones when relevant

### Before Adding Dependencies
Explain why the dependency is necessary and why it cannot be reasonably implemented internally. Prefer:
1. Standard library
2. Well-maintained, widely-used crates
3. Minimal dependency trees

## Project Structure

```
LayerMind/
├── Cargo.toml          # Workspace root
├── crates/             # Rust crates (one per concern)
│   ├── shared/         # Types, errors, contracts
│   ├── config/         # Configuration
│   ├── logging/        # Structured logging
│   ├── moonraker/      # Moonraker protocol adapter
│   ├── printer/        # Printer abstraction
│   ├── telemetry/      # Event pipeline
│   ├── database/       # Storage layer
│   ├── ai/             # AI engine
│   └── core/           # Orchestration
├── apps/               # Applications
│   └── desktop/        # Tauri desktop app (future)
├── python/             # Python ML components
│   └── ai/             # ML models and experimentation
├── tests/              # Integration tests
├── scripts/            # Utility scripts
├── docker/             # Containerization
└── docs/               # Additional documentation
```

## Testing

- Unit tests in each crate (`#[cfg(test)] mod tests`)
- Integration tests in `tests/`
- Mock services for external dependencies (Moonraker, database)

## Getting Started

```bash
# Build everything
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy -- -D warnings
```

## Questions?

Open an issue or discussion. Architecture decisions should be documented in `ARCHITECTURE.md`.
