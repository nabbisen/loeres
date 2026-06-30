# RFC 008 — Async Orchestration and Monomorphization Budgets

**Status.** Implemented (v0.13.0) — orchestration-first cluster slice: the per-item batch contract (`BatchSolveReport` / `BatchItemOutcome` / `BatchSummary` / `ClusterSolution`), a runtime-agnostic configuration / cancellation / executor layer (`parallel-rayon` and `async-tokio` behind feature gates), and the `ClusterJob` dispatch seam — exercised by deterministic in-crate test jobs (orchestration behavior, not numerical correctness). No production std-side solver kernel yet; trusted-pipeline / caching deferred to a follow-on cluster RFC; the size-budget gate deferred to RFC 010.
**Tracks.** Phase 3 / Milestone 3 — Dynamic Infrastructure and Cloud Cluster
**Touches.** `loeres-cluster/src/batch.rs` (`BatchSolveReport` / `BatchItemOutcome` / `ClusterSolution`), `loeres-cluster/src/runtime.rs` (cancellation token, runtime config, worker/timeout policy, executor coordination — internal submodules `runtime::cancel` / `runtime::executor` permitted), `loeres-cluster/src/solve.rs` (public solve/batch entrypoints + the dispatch barrier). `observe` / `gateway` / `model` untouched by this RFC.

---

### Extended Metadata
* **Rust Edition Compliance:** Rust 2024 Baseline
* **Target Environment:** `loeres-cluster`; uses `loeres` and `loeres-backend-std`

## 1. Executive Summary & Problem Statement

Cluster workloads require parallel throughput, async integration, cancellation, and batch execution. However, generic numerical code can explode binary size when instantiated across many scalar, backend, and solver combinations.

This RFC defines the **orchestration-first** cluster foundation: the per-item batch contract, cancellation / timeout layering, a hybrid dispatch barrier, runtime-facing configuration, a runtime-agnostic public API (feature-gated `Rayon` / `Tokio` internals), and minimal consumption of the RFC 012 validation vocabulary. It *names* the monomorphization-budget metric but defers the enforcing gate to RFC 010, and defers trusted-pipeline / caching to a follow-on cluster RFC (§2).

## 2. Architectural Context & Dependency Alignment

This RFC depends on:

* [RFC 001](001-stratified-scalar.md) — scalar tiers;
* [RFC 003](003-allocation-free-errors.md) — `SolverError` / `DiagnosticSnapshot`;
* [RFC 007](007-dynamic-sparse-adapters.md) — dynamic dense/sparse storage;
* [RFC 012](012-validation-state-and-trusted-input-policy.md) — the `loeres::validation` vocabulary;
* [RFC 014](014-core-solver-outcome-state.md) — `SolveReport` / `SolveStatus`.

Dependency rules:

| Crate | Rule |
|---|---|
| `loeres-cluster` | May use `std`, async runtimes, thread pools, logging/tracing |
| `loeres-backend-std` | Provides dynamic storage |
| `loeres` | Provides minimal contracts incl. the validation vocabulary |
| `loeres-device` | No dependency relation |
| `loeres-backend-static` | No dependency relation |

Cluster dynamic dispatch is allowed only at high-level orchestration boundaries. Inner numerical kernels should remain generic when doing so is within binary-size budgets.

**Scope (F2, orchestration-first).** RFC 008 is the cluster orchestration foundation: the batch contract, cancellation / timeout layering, the hybrid dispatch barrier, runtime-facing configuration, a runtime-agnostic public API, and minimal RFC 012 validation-vocabulary consumption (§3.1). Deferred out of this RFC:

* **Trusted-pipeline mechanics, validation caching, model identity, and mutation epochs** — moved to a named follow-on cluster RFC (provisionally **RFC 015**). RFC 012 originally deferred these to RFC 008; RFC 008 re-defers them to that follow-on, which consumes the RFC 012 vocabulary (`TrustKind` is `#[non_exhaustive]` so a pipeline category lands there).
* **The `xtask size-budget` gate implementation and threshold policy** — RFC 010 (xtask Verification Governance) owns the gate; RFC 008 only names the metric (§3.4).
* **Observability / tracing integration** — the owning `observe` RFC; RFC 008 reserves no `observe` surface beyond extension points.

## 3. Concrete Technical Specification

### 3.1 Cluster configuration

Cluster configuration is `std` and **runtime-agnostic** (F4): it carries no Tokio or Rayon types. `std::time::Duration` is used directly.

```rust
pub struct ClusterSolveConfig {
    pub max_parallelism: usize,
    pub timeout: Option<std::time::Duration>,
    pub cancellation_poll_interval: u32,
    pub validation_policy: ClusterValidationPolicy,
    pub dispatch_policy: DispatchPolicy,
}

pub enum DispatchPolicy {
    PreferGenericKernels,
    PreferHybridDispatch,
    AutoByBudget,
}
```

**Validation policy maps onto the RFC 012 vocabulary (F1)** — `ClusterValidationPolicy` is not a parallel trust model:

```rust
use loeres::validation::{TrustedByCaller, ValidationCoverage, ValidationState};

pub enum ClusterValidationPolicy {
    /// The cluster boundary runs the required checks and records a
    /// `ValidationCoverage` (`ValidationState::Validated`) where applicable.
    ValidateAllInputs,
    /// Consume a `ValidationState` / `ValidationCoverage` produced by the backend
    /// or an earlier pipeline stage; validate any missing required coverage
    /// before dispatch.
    RespectBackendValidationState,
    /// The caller assumes responsibility for a coverage scope, carried directly
    /// as RFC 012 `TrustedByCaller` evidence.
    TrustedByCaller(TrustedByCaller),
}
```

`TrustedPipeline` (a named upstream pipeline asserting trust via a host-side token, with caching / model identity / mutation epochs) is **not** in this RFC; it moves to the follow-on cluster RFC (§2). RFC 012 left `TrustKind` `#[non_exhaustive]` for that pipeline category.

Cluster APIs are allowed to use `std`; core and device APIs are not.

### 3.2 Batch outcome contract

Batch APIs return per-item outcomes. One ill-conditioned model must not fail the entire batch. The solution is a **concrete enum** over the supported shapes (F5), not a boxed trait object:

```rust
use loeres::{SolveReport, SolverError};
use loeres_backend_std::DenseVector;

/// Concrete erased solution over the supported shapes. Dense-vector first;
/// other variants are added (feature-gated) only where implemented.
pub enum ClusterSolution<S> {
    DenseVector(DenseVector<S>),
    // sparse / dense-matrix variants added where implemented (feature-gated)
}

pub struct BatchSolveReport<S> {
    pub outcomes: Vec<BatchItemOutcome<S>>,
    pub summary: BatchSummary,
}

pub enum BatchItemOutcome<S> {
    Solved { solution: ClusterSolution<S>, report: SolveReport },
    Failed { error: SolverError },
    Cancelled,
    Panicked,
}
```

A model that runs to its bounded terminus without converging is `Solved { report: status == NotConverged, .. }`, **not** `Failed` (F6). `Failed` is reserved for fail-safe `SolverError` conditions (RFC 014 §5.1), preserving the status/error split at the batch layer. `Solved` means the attempt reached a structured terminal report and produced the declared `ClusterSolution`; it does not imply `SolveStatus::Converged`. `Panicked` is a worker-task panic caught at the item boundary (§4.4) — distinct from a `SolverError`. The arity is concrete because `SolveReport` is a `Copy` core type.

A top-level `Err` (a `ClusterError`) is reserved for orchestration-level failures such as executor initialization failure, invalid global configuration, or complete runtime shutdown (§3.5).

(Whether `Failed` should also carry a `DiagnosticSnapshot` alongside `error` is an implementation-decision item.)

### 3.3 Hybrid dispatch barrier

The cluster stack has two layers:

1. orchestration layer: may use `dyn` to reduce code bloat;
2. numerical kernel layer: remains generic for hot loops where monomorphization is beneficial.

A monomorphized generic kernel is erased into a `dyn ClusterJob<S>` at the orchestration boundary; its output is a **concrete** `ClusterSolution<S>` enum (§3.2), not a boxed solution:

```rust
pub trait ClusterJob<S>: Send + Sync {
    fn run_boxed(&self, ctx: &ClusterExecutionContext) -> BatchItemOutcome<S>;
}
```

`ClusterExecutionContext` is the cluster-owned, runtime-agnostic per-item context (cancellation handle, worker policy — §3.5 / §3.6). `ClusterJob` and `ClusterExecutionContext` are not part of `loeres` and must never be used by `loeres-device` (zero-bleed). The `dyn` barrier is confined to cluster orchestration (§4.2) and does not weaken the RFC 002 static-dispatch rule for core/device kernels.

### 3.4 Monomorphization budget metric (named here; gate owned by RFC 010)

RFC 008 **names** the metric it wants measured; the gate's implementation and threshold policy are **RFC 010-owned** (xtask Verification Governance). Acceptance of RFC 008 does not depend on a working `xtask size-budget` gate while that command is scaffolded.

The metric is the `.text` growth ratio across representative monomorphized cluster entrypoints:

```text
text_growth_ratio = text_size(feature_set_with_new_generic_instantiations) /
                    text_size(reference_feature_set)
```

When RFC 010 wires the gate, an implementation that exceeds the configured threshold should reduce generic instantiation count, move orchestration to the dynamic dispatch barrier (§3.3), split feature-gated solver families, or justify the increase as intentional. The threshold itself is set by RFC 010 CI baseline recording, not by this RFC.

### 3.5 Async runtime and thread-pool coordination

The public async surface is **runtime-agnostic** (F4): it returns cluster-owned types and takes a cluster-owned `ClusterCancellationToken`, never a Tokio or Rayon type.

```rust
pub async fn solve_batch_async<S>(
    jobs: Vec<Box<dyn ClusterJob<S>>>,
    config: ClusterSolveConfig,
    cancel: ClusterCancellationToken,
) -> Result<BatchSolveReport<S>, ClusterError>;
```

Feature posture:

* baseline (`sync` / `batch`): server-`std` but runtime-light; synchronous batch execution;
* `parallel-rayon`: Rayon-backed parallel execution as a feature-gated internal;
* `async-tokio`: Tokio integration helpers as a feature-gated internal.

Tokio and Rayon types must not appear in the stable public surface unless a function or module is *explicitly* Tokio-named. `Rayon` work must not block the async runtime's core scheduler threads: blocking CPU work is scheduled through a dedicated pool or a `spawn_blocking` equivalent, confined behind the feature-gated internals.

`ClusterError` is the cluster-owned orchestration-failure type returned in the top-level `Err` (§3.2): executor initialization failure, invalid global configuration, or complete runtime shutdown. Its exact variants are an implementation-decision item.

### 3.6 Cancellation semantics

Cancellation is cooperative and observed at bounded intervals, with a pinned layering (F9) so the two representations cannot diverge:

* The async task receives a cluster-owned `ClusterCancellationToken`.
* Parallel worker loops poll cancellation between solver steps or after a bounded number of iterations.
* **Pre-dispatch** cancellation (an item cancelled before its kernel runs) maps directly to `BatchItemOutcome::Cancelled`.
* If an **inner solver** returns `SolverError::Cancelled` (RFC 003), the batch layer maps it to `BatchItemOutcome::Cancelled`.
* A **top-level `Err`** (`ClusterError`) is reserved for orchestration failure that prevents a valid batch report from being produced at all — never for a single item's cancellation.
* Cancellation must not leave shared batch state inconsistent.

## 4. Rust Systems-Level Nuances & Memory Safety

### 4.1 Send and Sync boundaries

Cluster jobs crossing worker threads must be `Send`. Shared data must be `Sync` or cloned explicitly. Interior mutability must be justified and tested for data races through safe Rust primitives.

### 4.2 Dynamic dispatch containment

`dyn` is accepted in cluster orchestration because cluster builds prioritize manageability and binary-size control. This does not weaken the RFC 002 static-dispatch rule for core/device kernels.

### 4.3 Binary-size pressure

The monomorphization budget must be checked before adding broad generic combinations such as multiple scalar types × dense/sparse backends × solver families.

### 4.4 Panic containment

Cluster code should not panic on malformed tenant input; malformed input is a `SolverError`, not a panic. Worker-task panics are layered explicitly (F8):

* Under `panic=unwind`, the batch worker boundary may catch a panic and convert it into an item-level `BatchItemOutcome::Panicked`, so one tenant's bug does not necessarily collapse the whole batch.
* Under `panic=abort`, there is no catch boundary and the process aborts.
* Solver-domain failures are always represented as `SolverError` (→ `BatchItemOutcome::Failed`), **never** mapped from a panic. `Panicked` is reserved for genuine worker-task panics and is distinct from `Failed`.

This preserves multi-tenant isolation without pretending panics are normal solver errors.

## 5. Algorithmic & Numerical Fail-Safe Guardrails

1. Batch model failures are isolated per item.
2. Tenant A's invalid input cannot cancel tenant B's valid model unless global cancellation is requested.
3. Input validation policy is explicit and maps onto the RFC 012 vocabulary (§3.1).
4. Timeouts and cancellation return structured outcomes (§3.6).
5. Numerical non-convergence is a *solved* item carrying `SolveStatus::NotConverged` (§3.2), not a failure and not an executor crash.
6. A worker-task panic is contained at the item boundary as `Panicked` (§4.4), not an executor crash, where `panic=unwind` permits.
7. Parallel loops must avoid shared mutable solver state unless protected by safe synchronization.

## 6. Verification, Validation, and CI Gates

### 6.1 Batch partial-failure tests

Tests must submit a mixed batch containing valid, ill-conditioned, malformed, and cancellation-triggered items. The valid items must still produce solved outcomes.

### 6.2 Cancellation tests

Tests must verify cancellation before dispatch, during execution, and during batch aggregation.

### 6.3 Async/thread-pool tests

Tests must verify that CPU-bound work does not block async runtime progress in representative scenarios.

### 6.4 Monomorphization budget metric (recorded; gate deferred)

RFC 008 records the `text_growth_ratio` metric (§3.4) for representative cluster feature sets. The enforcing `xtask size-budget` gate and its threshold are **RFC 010-owned**; RFC 008 acceptance does not depend on that gate while the command is scaffolded. This is the RFC 010 integration point, not a hard gate in this RFC.

### 6.5 Dependency checks

Cluster may depend on backend-std, but no edge-facing crate may depend on cluster.

### 6.6 Acceptance criteria

RFC 008 may move to `done/` only when:

1. batch APIs return per-item outcomes (`Solved` / `Failed` / `Cancelled` / `Panicked`) preserving the status/error split (`NotConverged` is `Solved`);
2. cancellation is cooperative and bounded, with the §3.6 layering;
3. the public runtime API is runtime-agnostic (no leaked Tokio/Rayon types), and async orchestration and CPU thread pools have a clear boundary;
4. `ClusterValidationPolicy` consumes the RFC 012 vocabulary (no parallel trust model), with trusted-pipeline / caching deferred to the follow-on cluster RFC;
5. worker panics are contained per the §4.4 layering;
6. hybrid dispatch (`dyn ClusterJob<S>` → concrete `ClusterSolution<S>`) is confined to cluster orchestration;
7. the `text_growth_ratio` metric is named for RFC 010 (no hard size-budget gate required in this RFC).
