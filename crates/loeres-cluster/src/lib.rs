//! `loeres-cluster` — the server-side developer interface.
//!
//! Environment: `std`, heap allocation, optional async/parallel/observability/
//! FFI behind feature gates. Optimizes for ergonomics, dynamic problem sizes,
//! throughput, and integration. Server-only: it must never be depended on by
//! edge-facing crates, and its dynamic-dispatch conveniences must not leak into
//! `loeres` contracts used by device code.
//!
//! Public module topography (external design §1.5):
//! `model`, `solve`, `batch`, `runtime`, `observe`, `gateway`.
//!
//! Phase 0 skeleton: modules are documented placeholders.

pub mod batch;
pub mod gateway;
pub mod model;
pub mod observe;
pub mod runtime;
pub mod solve;
