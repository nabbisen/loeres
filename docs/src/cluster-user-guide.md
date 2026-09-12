# Cluster User Guide

The server path: many problems, solved as a batch, with per-item outcomes. This
page is for an integrator embedding the cluster crate in a service, not for a
maintainer of the repository.

The runnable companion to this page is
[`examples/cluster-batch-solve/`](https://github.com/nabbisen/loeres/tree/main/examples/cluster-batch-solve).
Everything below appears there in full.

## What ships

The same solver family as the edge path — a box/bound-constrained **projected
first-order** kernel — over dynamic, runtime-dimensioned storage, plus the batch
orchestration around it. Generic LP, QP, and SOCP problem contracts are not
implemented. See
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
