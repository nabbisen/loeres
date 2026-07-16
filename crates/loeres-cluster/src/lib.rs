//! `loeres-cluster` — the server-side developer interface.
//!
//! Environment: `std`, heap allocation, and optional async/parallel integration
//! behind feature gates. Optimizes for ergonomics, dynamic problem sizes,
//! throughput, and integration. Server-only: it must never be depended on by
//! edge-facing crates, and its dynamic-dispatch conveniences must not leak into
//! `loeres` contracts used by device code. The shipped gateway surface is safe
//! boundary categories plus [`gateway::MockGatewayJob`]; `ffi-gateway` is an
//! inert, default-off activation gate reserved for a future separately reviewed
//! native/legacy adapter.
//!
//! Public module topography (external design §1.5):
//! `model`, `solve`, `batch`, `runtime`, `observe`, `gateway`,
//! `validation_cache`.
//!
//! RFC 008 (v0.13.0) populates the orchestration foundation in `batch`,
//! `runtime`, and `solve`: the per-item batch contract, a runtime-agnostic
//! configuration / cancellation / executor layer (with `parallel-rayon` and
//! `async-tokio` backends behind feature gates), and the hybrid dispatch
//! barrier ([`ClusterJob`](solve::ClusterJob)). It consumes the RFC 012
//! validation vocabulary at the orchestration boundary.
//!
//! RFC 016 (v0.14.0) adds the first std-side numerical kernel: a dynamic
//! box/bound-constrained projected first-order solver over `DenseVector`
//! ([`model`] types plus [`solve_projected_first_order_dyn`](solve::solve_projected_first_order_dyn)
//! and its [`ClusterProjectedFirstOrderJob`](solve::ClusterProjectedFirstOrderJob)
//! adapter), plugged into the `ClusterJob` seam. RFC 009 adds cluster-only
//! metadata observability and the safe gateway boundary. RFC 015 adds the
//! cluster-only validation evidence cache for model-owned scans.

pub mod batch;
pub mod gateway;
pub mod model;
pub mod observe;
pub mod runtime;
pub mod solve;
pub mod validation_cache;

pub use batch::{BatchItemOutcome, BatchSolveReport, BatchSummary, ClusterSolution};
pub use gateway::{
    GatewayBackendKind, GatewayFailureKind, GatewayThreadSafety, MockGatewayJob,
    MockGatewayResponse, solver_error_from_gateway_failure,
};
pub use model::{
    ClusterProjectedFirstOrderProblem, ClusterProjectedFirstOrderWorkspace,
    ProjectedFirstOrderConfig, ProjectedFirstOrderFiniteEvidence, ProjectedFirstOrderSolveRecord,
};
pub use observe::{
    DimensionBucket, ElapsedBucket, IterationsBucket, NoopObserver, OutcomeKind, ProblemClassId,
    SolveObservationContext, SolveObserver, SolveTelemetryEvent, SolverFamilyId,
    observe_batch_report, observe_batch_report_with_metadata, outcome_kind_from_item,
    outcome_kind_from_solver_error, solve_batch_observed, telemetry_event_from_item,
};
pub use runtime::{
    BatchExecutionPolicy, ClusterCancellationToken, ClusterError, ClusterSolveConfig,
    ClusterValidationPolicy, DispatchPolicy, MissingCoverage,
};
pub use solve::{
    ClusterExecutionContext, ClusterJob, ClusterProjectedFirstOrderJob,
    ProjectedFirstOrderSolveOptions, solve_batch, solve_projected_first_order_dyn,
    solve_projected_first_order_dyn_cached,
};

#[cfg(feature = "async-tokio")]
pub use solve::solve_batch_async;
pub use validation_cache::{
    CacheableProjectedFirstOrderProblem, CachedValidationEvidence, ModelIdentity, MutationEpoch,
    ProblemClassId as ValidationProblemClassId, ProvidedValidationEvidence, ScalarFamilyId,
    SolverFamilyId as ValidationSolverFamilyId, ValidationEvidenceCache, ValidationEvidenceKey,
    ValidationEvidenceLookup,
};
