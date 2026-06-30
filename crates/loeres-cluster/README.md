# loeres-cluster

Server-side solving: dynamic models, batch with per-item failure isolation, orchestration, observability, and optional audited FFI. **Server-only.**

- **Environment:** `std`
- **Depends on:** `loeres`, `loeres-backend-std`
- **Status:** RFC 008 (v0.13.0) populates the orchestration foundation in `batch`,
  `runtime`, and `solve`. `model`, `observe`, and `gateway` remain placeholders owned
  by later RFCs.

## What's implemented (RFC 008, v0.13.0)

Cluster orchestration **infrastructure — not a production numerical cluster solver.**
There is no std-side solver kernel yet (the core crate exposes only the solve-outcome
vocabulary, and the device kernel is edge-only and unreachable here), so `ClusterJob` is
the stable seam where a future kernel attaches, and the machinery is exercised by
deterministic test jobs that validate orchestration behavior, not numerical correctness.

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

## Features

The baseline synchronous batch path is unconditional and runtime-agnostic — no Tokio or
Rayon type appears in the baseline public surface. Optional, default-off:

- `parallel-rayon` — a bounded Rayon worker pool for parallel batch execution.
- `async-tokio` — a Tokio blocking-pool offload exposing `solve_batch_async`.

See the workspace [README](../../README.md), the [architecture](../../docs/src/architecture.md)
chapter, and the [RFC index](../../rfcs/README.md).

Licensed under Apache-2.0.
