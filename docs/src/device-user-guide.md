# Device User Guide

The edge path: a deterministic, allocation-free, bounded-iteration solve you can
run inside a control loop. This page is for an integrator writing that loop, not
for a maintainer of the repository.

The runnable companion to this page is
[`examples/device-box-pfo/`](https://github.com/nabbisen/loeres/tree/main/examples/device-box-pfo).
Everything below appears there in full.

## What ships

One solver family: a **projected first-order** kernel over fixed-size static
storage. `x <- clamp(x - alpha * grad f(x), lo, hi)`, stopping when the largest
coordinate change falls within tolerance. It comes in two forms: over a box (the
rest of this page), and over a box together with linear inequalities `Ax <= b`
(a quadratic program; see [Adding linear inequalities](#adding-linear-inequalities)).
No LP, SOCP, or interior-point solver ships. See
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

## Adding linear inequalities

When the feasible set is a polyhedron rather than a box, use the constrained
kernel (RFC 027) instead. The problem is `minimize ½xᵀQx + cᵀx` over
`lo <= x <= hi` and `Ax <= b`, and it is described by the `loeres` contract, not
by a `loeres-device` trait: implement `QuadraticObjective`, `BoxBounds` and
`LinearInequalities` (which together give `QuadraticProgram`) over fixed-size
storage, then call `solve_constrained_projected_first_order`.

What differs from the box-only solve:

- **The constraint count `M` is a const generic and must be at least 1.** The
  workspace, `ConstrainedProjectedWorkspace<S, N, M>`, is caller-owned like the
  box-only one and reusable across solves; its footprint is
  `(3N + 2M) * size_of::<S>()` plus a 16-byte header, reported through
  `WorkspaceFootprint`. A problem with no inequalities is not an `M = 0`
  instantiation: it uses `solve_projected_first_order`.
- **The step scale is an argument**, since the contract carries no execution
  parameter. `Q` must be symmetric positive semidefinite, which is not verified.
  The step is only partly checked: a provably divergent step is rejected, and
  nothing else about it is verified (see **Choosing the step** below).
- **There are two caps and two tolerances.** `ConstrainedSolveConfig` adds
  `projection_max_sweeps` and `projection_tolerance` to the outer
  `max_iterations` and `tolerance`. The sweep cap has no default: Dykstra
  converges linearly, so a tight `projection_tolerance` needs a far larger cap
  than a loose one.
- **The report has two extra fields.** `projection_cap_hits` counts outer
  iterations whose projection hit its cap, and `max_constraint_violation` is
  `max(0, max_i(a_i . x - b_i))` at the returned iterate. A cap hit is not an
  error, and the returned point may then be only feasible-approximate: read both
  fields before trusting the answer.

**Read the status as a claim about feasibility.** `Converged` means the final
iterate is stationary **and** satisfies every constraint within
`projection_tolerance`. `NotConverged` with `TerminationReason::NoProgress` means
the iterate stopped moving without being feasible, which on a constrained solve
indicates an infeasible polyhedron or one whose projection cap is too tight.
Under `ConstantIteration` the criterion is evaluated at the **final** iteration
(RFC 029), so a run that dipped within tolerance and then diverged is
`NotConverged`.

**Limits of this kernel** (RFC 027 §11.6):

- LP is expressible (`Q = 0`) but not solved; projected gradient on a linear
  objective has no curvature to converge against.
- Infeasibility is not detected. It is reported as `NotConverged` / `NoProgress`
  with a positive violation that does not shrink as `projection_max_sweeps` is
  raised. It is never an error.
- The projection is inexact by design. Its rate is set by the angles between
  constraint normals, and nearly parallel constraints can make the inner cap bind
  routinely.
- The step is bounded, not chosen for you; see **Choosing the step** below. No
  numeric convergence rate is claimed.
- `Q` symmetric positive semidefinite is a caller precondition.
- Device and cluster results agree within tolerance, not bitwise.

**Choosing the step (RFC 032).** The kernels take `step_scale` as an argument and
never choose it. For symmetric positive semidefinite `Q`, projected gradient
converges only for a step below `2 / lambda_max(Q)`, which the library does not
compute. It gives you two cheap bounds instead, from
`QuadraticProgram::curvature_bounds()`:

| Bound | Value | Guarantees |
|---|---|---|
| `lambda_max_upper` (`U`, Gershgorin) | `max_i sum_j abs(Q_ij)` | `U >= lambda_max` |
| `lambda_max_lower` (`L`) | `max_i Q_ii` | `L <= lambda_max` |

| Your step `a` | What is known | What the kernel does |
|---|---|---|
| `a < 2 / U` | provably convergent | accepts |
| `2 / U <= a < 2 / L` | **indeterminate**: it holds steps that converge and steps that do not | **accepts, and makes no claim** |
| `a >= 2 / L` | provably divergent | rejects as `InvalidInput` |

`suggested_step_scale()` returns `1 / U`, which is always in the first row: safe,
never optimal (`U` overestimates `lambda_max`). **Nothing calls it for you** —
substituting a step silently would change results — so pass it yourself. If you
know `lambda_max`, or have measured your own step, use that. Both bounds are
**meaningless unless `Q` is symmetric positive semidefinite**, which is your
responsibility and is not verified.

**The rate.** With an exact projection, `Q` positive *definite* and a step in
`(0, 2 / U)`, the iteration contracts linearly by
`max(|1 - a*lambda_min|, |1 - a*lambda_max|)`. The library computes neither
eigenvalue, so it states this *form* and its dependence on `lambda_min`, not a
number; for a merely semidefinite `Q` no rate is claimed. The inexact projection
is a separate matter (see the limits above). Choosing the safe step does **not**
make the projection exact: the nearly-parallel-constraint fixtures in the
adversarial conformance suite fail *at* the suggested step.

The cluster counterpart, with a runnable example, is in the
[Cluster User Guide](cluster-user-guide.md#constrained-quadratic-programs).

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
