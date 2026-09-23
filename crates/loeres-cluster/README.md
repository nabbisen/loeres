# loeres-cluster

Server-side solving: dynamic models, batch with per-item failure isolation,
orchestration, observability, safe gateway categories, and a pure Rust mock
gateway. The default-off `ffi-gateway` feature is an inert activation gate
reserved for a future separately reviewed native/legacy adapter. **Server-only.**

- **Environment:** `std`
- **Depends on:** `loeres`, `loeres-backend-std`
- **Status:** RFC 008 (v0.13.0) populates the orchestration foundation in `batch`,
  `runtime`, and `solve`; RFC 016 (v0.14.0) adds the first std-side numerical kernel in
  `model` and `solve`; RFC 009 (v0.15.0) adds metadata observability and the safe
  gateway boundary in `observe` and `gateway`; RFC 015 (v0.19.0) adds the
  process-local validation evidence cache; RFC 027 (unreleased `0.21.1` tree)
  adds the constrained quadratic-program kernel in `solve`.

## What's implemented

RFC 008 (v0.13.0) delivered the orchestration foundation; RFC 016 (v0.14.0) added the
first production std-side numerical kernel plugged into the `ClusterJob` seam, so the
cluster now does real solving (not only orchestration of deterministic test jobs). RFC 009
(v0.15.0) adds redacted observability and a safe gateway boundary without changing that seam.

- `batch` — the per-item outcome contract: `BatchItemOutcome` (`Solved` / `Failed` /
  `Cancelled` / `Panicked`, preserving the RFC 014 status/error split — a non-converged
  but terminal solve is `Solved`, never `Failed`), `ClusterSolution`, `BatchSolveReport`,
  and an explicit-count `BatchSummary`.
- `runtime` — `ClusterSolveConfig`, `BatchExecutionPolicy`, a reserved-but-inert
  `DispatchPolicy`, `ClusterValidationPolicy` (consuming the RFC 012 validation
  vocabulary; `resolve` performs no scans — it resolves policy against recorded
  evidence), the cluster-owned `ClusterCancellationToken`, and a small `ClusterError`.
- `solve` — the `ClusterJob` hybrid-dispatch seam, `ClusterExecutionContext`,
  `solve_batch`, and (behind `async-tokio`) `solve_batch_async`.
- `observe` — metadata-only `SolveTelemetryEvent` categories, bounded static labels,
  the total `SolverError` / `BatchItemOutcome` classifiers, `SolveObserver` /
  `NoopObserver`, `observe_batch_report`, and `solve_batch_observed`.
- `gateway` — safe gateway boundary categories (`GatewayBackendKind`,
  `GatewayThreadSafety`, `GatewayFailureKind`) plus a pure Rust `MockGatewayJob`
  exercising status/error mapping and adapter-side rejection of unwrapped
  `SingleThreadOnly` backends. No concrete native solver adapter ships here.
- `validation_cache` — model identity/mutation epochs,
  `ValidationEvidenceCache`, `CacheableProjectedFirstOrderProblem`, and the
  cached `f64` PFO solve path. Evidence is process-local and model-carrier
  scoped; wrong identity/stale epoch fails closed.

### RFC 016 (v0.14.0) — std-side projected first-order kernel

- `model` — the typed problem surface: `ClusterProjectedFirstOrderProblem` (first-order
  oracle over box bounds), the reusable single-scratch `ClusterProjectedFirstOrderWorkspace`,
  `ProjectedFirstOrderConfig`, and `ProjectedFirstOrderSolveRecord` (the terminal report
  plus honest validation evidence: a `checked_scope` and a `ProjectedFirstOrderFiniteEvidence`
  naming whether finiteness was `Scanned`, `Trusted(..)`, or `DomainInapplicable`).
- `solve` — `solve_projected_first_order_dyn` (typed in/out-iterate entrypoint) and the
  `&self`-safe `ClusterProjectedFirstOrderJob` adapter onto `ClusterJob`. Dynamic
  box/bound-constrained projected first-order over `DenseVector`, step-norm convergence
  aligned with RFC 006; non-convergence at the cap is a *solved* `NotConverged`, never a
  failure; in-loop non-finite maps to `NumericalDomain` even under trust.

### RFC 027 (unreleased `0.21.1`) — constrained projected first-order kernel

- `solve` — `solve_constrained_projected_first_order_dyn` (the typed entrypoint) for
  any `loeres::QuadraticProgram` over dynamic storage: `min ½xᵀQx + cᵀx` over a
  box and linear inequalities `Ax <= b`, with `A` dense, CSR, or any
  `MatrixAccess` (the kernel needs no contiguous fast path). Configuration is
  `ConstrainedProjectedConfig`, scratch is `ClusterConstrainedWorkspace`
  (allocated once, sized `(n, m)`), and the result is `ConstrainedSolveRecord`:
  the terminal report, honest validation evidence, `projection_cap_hits`, and
  `max_constraint_violation`. `ClusterConstrainedJob` plugs it into `ClusterJob`.
  `m = 0` is accepted and performs the single exact box projection with no
  Dykstra sweep — the RFC 016 step, operation for operation. Results match
  RFC 016 up to the sign of zero **when the two oracles coincide** (`Q = I`,
  `c = −t`); for a general `Q` the oracles differ in floating point and the
  agreement is within tolerance, not exact (RFC 027 §0.2.5).
- `Converged` means **feasible within `projection_tolerance`** (Amendment 5).
  `NotConverged` with `NoProgress` indicates an infeasible or too-tightly-capped
  polyhedron.
- **The batch seam carries status only.** `ClusterConstrainedJob` erases to
  `BatchItemOutcome`, which holds the core `SolveReport`, so `projection_cap_hits`
  and `max_constraint_violation` are not visible through `solve_batch`. Because the
  status is truthful, that does not hide an infeasible answer as a converged one;
  a caller who needs the magnitudes uses the typed entrypoint.
- See `examples/cluster-qp-constrained/`.

Limits (RFC 027 §11.6): LP is expressible (`Q = 0`) but not solved; infeasibility
is not detected (reported as `NotConverged` / `NoProgress` with a positive
violation that does not shrink as the sweep cap is raised); the projection is
inexact by design, converging linearly at a rate set by constraint-normal
angles, so nearly parallel constraints can make the inner cap bind routinely and
`projection_max_sweeps` has no default; no convergence rate is claimed and
`step_scale` must lie in `(0, 2 / λ_max(Q))`; `Q` symmetric positive semidefinite
is a caller precondition and is not verified; and device and cluster agree within
tolerance, not bitwise (RFC 013).

Validation note: `ValidateAllInputs` scans eligible model-owned data.
`RespectBackendValidationState` may consume provided/current RFC 015 evidence
on the carrier-only cached path; missing or insufficient evidence falls back to
validation, while wrong identity or stale epoch fails closed. `TrustedByCaller`
may skip eligible pre-loop model finite scans for the asserted scope. None of
these paths skips current-iterate, step-scale, cancellation, workspace/config,
or hot-loop finiteness checks.

The cache is neither persistent nor distributed. Metadata redaction is not
proof of tenant isolation, and no broad throughput, large-N, memory-pressure,
or multi-tenant stress claim is made.

## Features

The baseline synchronous batch path is unconditional and runtime-agnostic — no Tokio or
Rayon type appears in the baseline public surface. Optional, default-off:

- `parallel-rayon` — a bounded Rayon worker pool for parallel batch execution.
- `async-tokio` — a Tokio blocking-pool offload exposing `solve_batch_async`.
- `observability-tracing` / `observability-metrics` — reserved default-off integration
  gates; the baseline observability types need no external telemetry dependency.
- `ffi-gateway` — reserved default-off gate for audited concrete native/legacy solver
  adapters. RFC 009 ships only the safe boundary and mock gateway.

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
