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
//! RFC 008 (v0.13.0) populates the orchestration foundation in `batch`,
//! `runtime`, and `solve`: the per-item batch contract, a runtime-agnostic
//! configuration / cancellation / executor layer (with `parallel-rayon` and
//! `async-tokio` backends behind feature gates), and the hybrid dispatch
//! barrier ([`ClusterJob`](solve::ClusterJob)). It consumes the RFC 012
//! validation vocabulary at the orchestration boundary.
//!
//! Scope note: this is cluster orchestration **infrastructure**, not a
//! production numerical cluster solver. No std-side solver kernel exists yet;
//! `ClusterJob` is the stable seam where one plugs in later, and the machinery
//! is validated against deterministic test jobs (orchestration behavior, not
//! numerical correctness). `model`, `observe`, and `gateway` remain placeholders
//! owned by later RFCs.

pub mod batch;
pub mod gateway;
pub mod model;
pub mod observe;
pub mod runtime;
pub mod solve;

pub use batch::{BatchItemOutcome, BatchSolveReport, BatchSummary, ClusterSolution};
pub use runtime::{
    BatchExecutionPolicy, ClusterCancellationToken, ClusterError, ClusterSolveConfig,
    ClusterValidationPolicy, DispatchPolicy, MissingCoverage,
};
pub use solve::{ClusterExecutionContext, ClusterJob, solve_batch};

#[cfg(feature = "async-tokio")]
pub use solve::solve_batch_async;
