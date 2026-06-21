# RFC 008 — Async Orchestration and Monomorphization Budgets

**Status.** Proposed
**Tracks.** Phase 3 / Milestone 3 — Dynamic Infrastructure and Cloud Cluster
**Touches.** `loeres-cluster/src/executor.rs`, `loeres-cluster/src/batch.rs`, `loeres-cluster/src/cancel.rs`, `loeres-cluster/src/config.rs`, cluster orchestration modules

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-cluster`; uses `loeres-core` and `loeres-backend-std`

## 1. Executive Summary & Problem Statement

Cluster workloads require parallel throughput, async integration, cancellation, and batch execution. However, generic numerical code can explode binary size when instantiated across many scalar, backend, and solver combinations.

This RFC defines the cluster orchestration architecture, including a monomorphization budget, a hybrid dispatch barrier, `Rayon`/`Tokio` coordination, cancellation behavior, and per-item batch failure semantics.

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](001-stratified-scalar.md);
* [RFC 003](003-allocation-free-errors.md);
* [RFC 007](007-dynamic-sparse-adapters.md).

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-cluster` | May use `std`, async runtimes, thread pools, logging/tracing |
| `loeres-backend-std` | Provides dynamic storage |
| `loeres-core` | Provides minimal contracts |
| `loeres-device` | No dependency relation |
| `loeres-backend-static` | No dependency relation |

Cluster dynamic dispatch is allowed only at high-level orchestration boundaries. Inner numerical kernels should remain generic when doing so is within binary-size budgets.

## 3. Concrete Technical Specification

### 3.1 Cluster configuration

```rust
pub struct ClusterSolveConfig {
    pub max_parallelism: usize,
    pub timeout: Option<core::time::Duration>,
    pub cancellation_poll_interval: u32,
    pub validation_policy: ClusterValidationPolicy,
    pub dispatch_policy: DispatchPolicy,
}

pub enum ClusterValidationPolicy {
    ValidateAllInputs,
    RespectBackendValidationState,
    TrustedPipeline,
}

pub enum DispatchPolicy {
    PreferGenericKernels,
    PreferHybridDispatch,
    AutoByBudget,
}
```

The exact time type may be adjusted to `std::time::Duration` in implementation. Cluster APIs are allowed to use `std`; core and device APIs are not.

### 3.2 Batch outcome contract

Batch APIs must return per-item outcomes. One ill-conditioned model must not fail the entire batch.

```rust
use loeres_core::solver::SolveReport;

pub struct BatchSolveReport<Solution> {
    pub outcomes: Vec<BatchItemOutcome<Solution>>,
    pub summary: BatchSummary,
}

pub enum BatchItemOutcome<Solution> {
    Solved { solution: Solution, report: SolveReport },
    Failed { error: SolverError, diagnostic: DiagnosticSnapshot },
    Cancelled,
}
```

A model that runs to its bounded terminus without converging is `Solved { report: status == NotConverged, .. }`, not `Failed`. `Failed` is reserved for fail-safe `SolverError` conditions (RFC 014 §5.1), which preserves the status/error split at the batch layer. Note that `Solved` means the attempt reached a structured terminal report and produced the declared output container; it does not necessarily mean `SolveStatus::Converged`. The arity `BatchItemOutcome<Solution>` is unchanged because `SolveReport` is a concrete `Copy` core type.

A top-level `Err` is reserved for orchestration-level failures such as executor initialization failure, invalid global configuration, or complete runtime shutdown.

### 3.3 Hybrid dispatch barrier

The cluster stack has two layers:

1. orchestration layer: may use `dyn` to reduce code bloat;
2. numerical kernel layer: remains generic for hot loops where monomorphization is beneficial.

```rust
pub trait ClusterJob: Send + Sync {
    fn run_boxed(&self, ctx: &ClusterExecutionContext) -> BatchItemOutcome<ClusterSolution>;
}
```

This trait is not part of `loeres-core` and must never be used by `loeres-device`.

### 3.4 Monomorphization budget metric

The project must track compiled `.text` size for representative cluster builds. The budget is expressed as:

```text
text_growth_ratio = text_size(feature_set_with_new_generic_instantiations) /
                    text_size(reference_feature_set)
```

A proposed implementation that exceeds the configured growth threshold must either:

* reduce generic instantiation count;
* move orchestration to a dynamic dispatch barrier;
* split feature-gated solver families;
* justify the increased size as intentional.

The initial threshold is set by CI baseline recording, not by this RFC.

### 3.5 Async runtime and thread-pool coordination

Cluster public APIs may be async:

```rust
pub async fn solve_batch_async<M>(
    models: Vec<M>,
    config: ClusterSolveConfig,
    cancel: CancellationToken,
) -> Result<BatchSolveReport<ClusterSolution>, ClusterError>;
```

`Rayon` work must not block the async runtime's core scheduler threads. Blocking CPU work must be scheduled through a dedicated pool or `spawn_blocking` equivalent.

### 3.6 Cancellation semantics

Cancellation must be cooperative and observed at bounded intervals.

* The async task receives a cancellation token.
* Parallel worker loops poll cancellation between solver steps or after a bounded number of iterations.
* A cancelled item returns `BatchItemOutcome::Cancelled` or `SolverError::Cancelled` depending on API layer.
* Cancellation must not leave shared batch state inconsistent.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Send and Sync boundaries

Cluster jobs crossing worker threads must be `Send`. Shared data must be `Sync` or cloned explicitly. Interior mutability must be justified and tested for data races through safe Rust primitives.

### 4.2 Dynamic dispatch containment

`dyn` is accepted in cluster orchestration because cluster builds prioritize manageability and binary-size control. This does not weaken the RFC 002 static-dispatch rule for core/device kernels.

### 4.3 Binary-size pressure

The monomorphization budget must be checked before adding broad generic combinations such as multiple scalar types × dense/sparse backends × solver families.

### 4.4 Panic containment

Cluster code should not panic on malformed tenant input. Panics from worker tasks must be caught at task boundaries where possible and converted into item-level failures or orchestration errors.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Batch model failures are isolated per item.
2. Tenant A's invalid input cannot cancel tenant B's valid model unless global cancellation is requested.
3. Input validation policy is explicit.
4. Timeouts and cancellation return structured outcomes.
5. Numerical non-convergence is an item-level failure, not an executor crash.
6. Parallel loops must avoid shared mutable solver state unless protected by safe synchronization.

## 6. Verification, Validation, and CI Gates

### 6.1 Batch partial-failure tests

Tests must submit a mixed batch containing valid, ill-conditioned, malformed, and cancellation-triggered items. The valid items must still produce solved outcomes.

### 6.2 Cancellation tests

Tests must verify cancellation before dispatch, during execution, and during batch aggregation.

### 6.3 Async/thread-pool tests

Tests must verify that CPU-bound work does not block async runtime progress in representative scenarios.

### 6.4 Monomorphization budget check

`xtask size-budget` must produce `.text` size reports for representative cluster feature sets and fail when configured budgets are exceeded.

### 6.5 Dependency checks

Cluster may depend on backend-std, but no edge-facing crate may depend on cluster.

### 6.6 Acceptance criteria

RFC 008 may move to `done/` only when:

1. batch APIs return per-item outcomes;
2. cancellation is cooperative and bounded;
3. async orchestration and CPU thread pools have a clear boundary;
4. `.text` growth is measured by CI;
5. hybrid dispatch is confined to cluster orchestration.
