# Device User Guide

The edge path: a deterministic, allocation-free, bounded-iteration solve you can
run inside a control loop. This page is for an integrator writing that loop, not
for a maintainer of the repository.

The runnable companion to this page is
[`examples/device-box-pfo/`](https://github.com/nabbisen/loeres/tree/main/examples/device-box-pfo).
Everything below appears there in full.

## What ships

One solver family: a box/bound-constrained **projected first-order** kernel over
fixed-size static storage. `x <- clamp(x - alpha * grad f(x), lo, hi)`, stopping
when the largest coordinate change falls within tolerance. Generic LP, QP, and
SOCP problem contracts are not implemented. See
[Terms of Engineering Use](https://github.com/nabbisen/loeres/blob/main/TERMS_OF_USE.md)
for the full scope statement.

Two crates, both `#![no_std]` with no `alloc`:

| Crate | What you use from it |
|---|---|
| `loeres-device` | the kernel, the problem trait, the workspace, the solve config |
| `loeres-backend-static` | `FixedVector<S, N>` — the fixed-size storage the kernel works over |

Enable the `owned-arrays` feature on both. Nothing else is required, and default
features are off in the example on purpose.

## The shape of a solve

Four things, in this order.

**1. Implement the problem trait.** `ProjectedFirstOrderProblem<S, N>` asks for
the box bounds, a step scale, a first-order oracle that writes the gradient into
a caller-supplied buffer, and an objective for reporting. The dimension `N` is a
const generic, so the iterate, the gradient scratch, and the bounds are tied
together at compile time — a wrong-sized workspace is a compile error, not a
runtime check.

Read and write through the fallible accessors (`get` / `set`) rather than
indexing. The crates hold themselves to that discipline and your oracle is called
inside their loop.

**2. Build the workspace once, and keep it.** The workspace is caller-owned and
carries only scratch — the iterate is a separate `&mut x` argument. On a device it
lives in a static or on your stack. Reuse it across every solve; nothing in the
solve path allocates, so the second call costs no more memory than the first. The
example solves three problems with one workspace.

**3. Configure.** `max_iterations` is a hard cap and `tolerance` is the stopping
criterion on iterate change. `TimingMode::EarlyExitAllowed` returns as soon as
the criterion is met. The `constant-iteration` feature adds a mode that always
runs the full cap so the iteration *count* is stable — that is not constant-time
execution in the cryptographic sense and must not be relied on for side-channel
resistance.

**4. Call, then read the status.** This is the part most worth internalizing:

> **Non-convergence is not an error.** Reaching the iteration cap returns
> `Ok(report)` with `SolveStatus::NotConverged`, and `x` holds the last projected
> iterate. `Err(SolverError)` is reserved for invalid configuration, invalid
> bounds, dimension mismatch, and oracle failure.

A control loop that treats "did not converge in 5 iterations" as a failure will
discard a usable iterate. Decide deliberately what your loop does with a
`NotConverged` result — use it, extend the budget, or fall back — rather than
letting an error path make that decision for you.

`SolveStatus` is `#[non_exhaustive]` downstream, so match it with a catch-all
arm: a future status is something for your code to name, never a reason to panic.

## What running the example shows

```text
interior optimum:      converged in 93 iteration(s); ...
bounds active:         converged in 28 iteration(s); ...
cap reached:           not converged (iteration cap reached) in 5 iteration(s); ...
```

Three cases: an optimum inside the box, an optimum outside it (so the solution
sits on the boundary at `clamp(center, lower, upper)`), and a cap deliberately
set far below what the problem needs.

## What the example does and does not prove

It proves **dependency reachability**: a device integrator can solve a bounded
problem with no server-side crate in the graph. `cargo xtask examples` asserts
that against the example's *resolved* dependency graph — not its manifest — and
the example is excluded from the workspace precisely so that graph is its own
evidence rather than a product of feature unification with a sibling.

It does **not** prove bare-metal buildability. The example is a host program and
its own `main` uses `std`; that changes nothing about the edge crates, which are
`#![no_std]` with no `alloc`. The bare-metal claim belongs to `cargo xtask
no-std`, which builds the edge crates for `thumbv7em-none-eabihf`. The two
claims are separate and are kept separate deliberately — see
[Verification & Evidence](verification.md).

## Before you deploy this in something that matters

Read [Terms of Engineering Use](https://github.com/nabbisen/loeres/blob/main/TERMS_OF_USE.md).
Loeres holds no safety certification, is panic-averse rather than proven
panic-free, and makes target-scoped rather than universal determinism claims.
Timing, numerical, and certification evidence for your target is yours to
establish.
