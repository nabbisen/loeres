# loeres-cluster

Server-side solving: dynamic models, batch with per-item failure isolation, orchestration, observability, and optional audited FFI. **Server-only.**

- **Environment:** `std`
- **Depends on:** `loeres`, `loeres-backend-std`
- **Status:** RFC 008 (v0.13.0) populates the orchestration foundation in `batch`,
  `runtime`, and `solve`; RFC 016 (v0.14.0) adds the first std-side numerical kernel in
  `model` and `solve`. `observe` and `gateway` remain placeholders owned by later RFCs.

## What's implemented

RFC 008 (v0.13.0) delivered the orchestration foundation; RFC 016 (v0.14.0) added the
first production std-side numerical kernel plugged into the `ClusterJob` seam, so the
cluster now does real solving (not only orchestration of deterministic test jobs).

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

Validation note: `ClusterValidationPolicy::ValidateAllInputs` and, in v1,
`RespectBackendValidationState` both **scan inputs here** — there is no provided/cached
backend-state channel yet (that is RFC 015-owned). `TrustedByCaller` skips the pre-loop
finite scans for the asserted scope but never the hot-loop finiteness checks.

## Features

The baseline synchronous batch path is unconditional and runtime-agnostic — no Tokio or
Rayon type appears in the baseline public surface. Optional, default-off:

- `parallel-rayon` — a bounded Rayon worker pool for parallel batch execution.
- `async-tokio` — a Tokio blocking-pool offload exposing `solve_batch_async`.

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
