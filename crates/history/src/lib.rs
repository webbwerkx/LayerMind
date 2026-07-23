//! Historical timeline engine — immutable event recording, snapshots,
//! diffs, and strongly-typed queries.
//!
//! # Architecture
//!
//! ```text
//! TimelineStore ─── insert-only event log
//!     │
//!     ├── TimelineQueryEngine ─── filter/search the timeline
//!     │
//!     ├── SnapshotBuilder ─────── capture current state
//!     │
//!     └── SnapshotDiffEngine ─── compute deltas
//! ```
//!
//! All components are deterministic. No AI, no heuristics.
//! The history crate depends only on `shared`, never on `reasoning` or
//! `ai`.

pub mod diff;
pub mod query;
pub mod snapshot;
pub mod store;

pub use diff::SnapshotDiffEngine;
pub use query::TimelineQueryEngine;
pub use snapshot::SnapshotBuilder;
pub use store::TimelineStore;
