# Cluster User Guide

The server path: many problems, solved as a batch, with per-item outcomes. This
page is for an integrator embedding the cluster crate in a service, not for a
maintainer of the repository.

The runnable companion to this page is
[`examples/cluster-batch-solve/`](https://github.com/nabbisen/loeres/tree/main/examples/cluster-batch-solve).
Everything below appears there in full.

## What ships

The same solver family as the edge path — a **projected first-order** kernel, over
a box and over a box with linear inequalities — over dynamic, runtime-dimensioned
storage, plus the batch orchestration around it. No LP, SOCP, or interior-point
solver ships. See
[Terms of Engineering Use](https://github.com/nabbisen/loeres/blob/main/TERMS_OF_USE.md)
for the full scope statement.

| Crate | What you use from it |
|---|---|
| `loeres-cluster` | the batch entrypoint, the job adapter, the problem trait, the configs |
| `loeres-backend-std` | `DenseVector<S>` — dynamic dense storage |

The example enables `parallel-rayon` on `loeres-cluster` and `dense` on
`loeres-backend-std`. Every cluster feature is server-only; `ffi-gateway` is a
reserved activation gate and is never a default.

## The shape of a batch

**1. Implement the problem trait.** `ClusterProjectedFirstOrderProblem<S>` asks
for the dimension, the bounds, a step scale, and a gradient oracle — the dynamic
counterpart of the device trait, with the dimension a runtime value instead of a
const generic.

**2. Wrap each problem in a job.** `ClusterProjectedFirstOrderJob::new` takes the
problem, a starting iterate, and the numeric config. Then erase it:
`Box<dyn ClusterJob<S>>`. That erasure is the dispatch barrier — the
monomorphized kernel is erased once, at the orchestration boundary, so batch
handling stays one code path regardless of how many problem types you submit.

**3. Configure the batch.** `ClusterSolveConfig` carries `max_parallelism`, an
optional cooperative `timeout` observed at item boundaries, a cancellation poll
hint, the validation policy, and the execution policy. `BatchExecutionPolicy`
normalizes `Parallel` to `Sequential` when `parallel-rayon` is not compiled in,
so a build without the feature behaves deterministically rather than failing.

**4. Call `solve_batch`, then read each outcome.** The separation that matters:

> A top-level `Err(ClusterError)` means an **orchestration** failure — invalid
> global config, worker-pool init, runtime shutdown. Per-item solver failures
> never surface there. They are carried in the returned outcomes.

So one bad problem does not fail the batch. Its neighbours still solve.

## Reading per-item outcomes

`BatchItemOutcome` has four shapes, and the first one carries a distinction worth
reading twice:

| Outcome | Meaning |
|---|---|
| `Solved { solution, report }` | reached a structured terminal report and produced a solution — **`report.status()` may be `NotConverged`** |
| `Failed { error }` | a fail-safe `SolverError` prevented a solution |
| `Cancelled` | cooperative cancellation, pre-dispatch cancellation, or timeout |
| `Panicked` | a worker panic contained at the item boundary |

`Solved` does not mean converged. An item that hit its iteration cap is `Solved`
with a `NotConverged` report and a usable last iterate; what to do about it is
the caller's decision, not an error path's. `BatchSummary` counts
`solved_converged` and `solved_not_converged` separately for exactly this reason
— a dashboard that sums them into "succeeded" throws away the distinction.

`Panicked` has one precondition: worker-panic containment applies only when the
host process uses `panic = "unwind"`. Under `panic = "abort"` a panic aborts the
process and no containment is promised.

Both `ClusterSolution` and `SolveStatus` are `#[non_exhaustive]` downstream.
Match with a catch-all arm.

## What running the example shows

```text
interior optimum:          converged in 93 iteration(s); ...
bounds active:             converged in 2 iteration(s); ...
cap reached:               not converged (cap reached) in 5 iteration(s); ...
inverted bounds:           failed: InvalidInput

4 item(s): 2 converged, 1 not converged, 1 failed, 0 cancelled, 0 panicked
```

Four items: two that converge, one whose cap is deliberately too low, and one
with `lower > upper`, which is rejected before the loop as a per-item `Failed`.
The batch itself returns `Ok`.

## Constrained quadratic programs

The runnable companion for this section is
[`examples/cluster-qp-constrained/`](https://github.com/nabbisen/loeres/tree/main/examples/cluster-qp-constrained).

When the feasible set is a polyhedron, use the constrained kernel (RFC 027):
`minimize ½xᵀQx + cᵀx` over `lo <= x <= hi` and `Ax <= b`. The problem is
described by the `loeres` contract — implement `QuadraticObjective`, `BoxBounds`
and `LinearInequalities`, which together give `QuadraticProgram` — over dynamic
storage; `A` may be dense, CSR, or any `MatrixAccess`. Then call
`solve_constrained_projected_first_order_dyn` with the problem, a step scale, an
iterate, a `ClusterConstrainedWorkspace::new(n, m)` (allocated once, reusable), a
`ConstrainedProjectedConfig`, and a `ClusterExecutionContext`.

It returns a `ConstrainedSolveRecord`: the terminal report, `projection_cap_hits`,
and `max_constraint_violation` — the terminal constraint violation
`max(0, max_i(a_i . x - b_i))`. **Read the violation beside the status.** An inner
projection cap hit is not an error, so the returned point may be only
feasible-approximate.

**`Converged` means feasible.** The final iterate is stationary **and** within
`projection_tolerance` of every constraint. `NotConverged` with
`TerminationReason::NoProgress` on a constrained solve indicates an infeasible or
too-tightly-capped polyhedron. The example shows all three cases:

```text
slack halfspace:     converged in 56 iteration(s); x = [1.714286, 1.142857]; violation = 0.000e0; projection cap hits = 0
active halfspace:    converged in 41 iteration(s); x = [1.500000, 0.500000]; violation = 0.000e0; projection cap hits = 0
infeasible:          not converged (no progress: stationary but not feasible) in 46 iteration(s); x = [0.000000, 2.000000]; violation = 1.000e0; projection cap hits = 46
```

The first two agree with the closed forms, `Q⁻¹(4, 2) = (12/7, 8/7)` and the KKT
point `(1.5, 0.5)`; the third violates `x0 <= -1` against `x0 >= 0` by exactly 1.

**The batch seam carries the status only.** `ClusterConstrainedJob` erases the
solve to a `BatchItemOutcome`, which holds the core report. A caller reading a
`Solved` outcome sees `Converged` or `NotConverged` — truthful about feasibility —
but not `projection_cap_hits` or `max_constraint_violation`. A caller who needs
the magnitudes uses the typed entrypoint above. The `m = 0` case (no inequalities)
is accepted and performs the single exact box projection with no Dykstra sweep —
the RFC 016 step, operation for operation. Results match RFC 016 up to the sign of
zero **when the two oracles coincide** (`Q = I`, `c = −t`); for a general `Q` the
oracles differ in floating point and the agreement is within tolerance, not exact
(RFC 027 §0.2.5).

**Limits of this kernel** (RFC 027 §11.6):

- LP is expressible (`Q = 0`) but not solved.
- Infeasibility is not detected; it is reported as `NotConverged` / `NoProgress`
  with a positive violation that does not shrink as `projection_max_sweeps` is
  raised. It is never an error.
- The projection is inexact by design. Its rate is set by the angles between
  constraint normals, nearly parallel constraints can make the inner cap bind
  routinely, and `projection_max_sweeps` has no default: it must suit
  `projection_tolerance`.
- No convergence rate is claimed; `step_scale` must lie in `(0, 2 / lambda_max(Q))`.
- `Q` symmetric positive semidefinite is a caller precondition and is not
  verified.
- Device and cluster results agree within tolerance, not bitwise.

The two caps multiply: worst-case work is `max_iterations x projection_max_sweeps`
sweeps. A service that takes those values from a request should bound both.

## Server-side boundaries to know about

These are stated in full in
[Terms of Engineering Use](https://github.com/nabbisen/loeres/blob/main/TERMS_OF_USE.md);
the short form:

- **Observability** is metadata-only and redacted by default. That reduces
  disclosure risk; it does not by itself establish multi-tenant isolation, and no
  multi-tenant stress or isolation evidence exists.
- **The gateway surface is a safe mock only.** No native or FFI adapter ships,
  and enabling `ffi-gateway` does not provide one.
- **Validation evidence caching is process-local** — neither persistent nor
  distributed — and never skips per-call current-iterate or hot-loop
  numerical-domain checks.
- **Conformance evidence is a bounded smoke corpus**, small and fixed-dimension.
  It is not broad numerical parity, large-N validation, or throughput evidence.
